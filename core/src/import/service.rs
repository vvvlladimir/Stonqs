use super::dedupe::{fingerprint, fingerprint_of, loose_fingerprint_of};
use super::ibflex;
use super::mapping::{ImportMapping, normalize_alias};
use super::parse::{ImportProblem, ParseConfig, ProblemCode, parse_csv};
use super::preview::{ImportContext, ImportPreview, RowOverride, RowStatus, build_preview};
use super::prices::{PriceImport, PriceMapping, build_price_import};
use super::securities::SecurityDraft;
use crate::error::Result;
use crate::model::{SecurityKind, is_isin};
use crate::money::normalize_currency;
use crate::storage::Store;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Controls which preview rows are written during commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportOptions {
    pub create_missing_securities: bool,

    pub new_security_kind: SecurityKind,

    /// Which source prices the instruments the import creates. `None` is priced by hand: the
    /// caller names a source, because the core ships no default one (ADR-0076).
    #[serde(default)]
    pub new_security_source: Option<String>,

    pub import_duplicates: bool,

    /// Write rows a stored operation only *resembles*. Off by default: the likeliest reading is
    /// that the stored row is this one, corrected by hand since (`RowStatus::Similar`).
    #[serde(default)]
    pub import_similar: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        ImportOptions {
            create_missing_securities: true,
            new_security_kind: SecurityKind::Other,
            new_security_source: None,
            import_duplicates: false,
            import_similar: false,
        }
    }
}

/// Counts and diagnostics returned by a transaction commit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportResult {
    pub imported: usize,
    /// Rows that replaced an operation already in the database, matched by the broker's own
    /// identifier. Counted apart from `imported`: nothing new entered the ledger.
    #[serde(default)]
    pub updated: usize,
    pub skipped: usize,

    /// Rows skipped because a stored operation resembles them. Counted apart from `skipped`, so
    /// the screen can offer to write them after all rather than leave the user guessing.
    #[serde(default)]
    pub similar: usize,

    pub created_securities: Vec<String>,
    pub problems: Vec<ImportProblem>,
}

/// Store-backed orchestration for preview and commit.
pub struct ImportService<'a> {
    store: &'a Store,
    base_currency: Option<String>,
    today: Option<NaiveDate>,
}

impl<'a> ImportService<'a> {
    pub fn new(store: &'a Store) -> Self {
        ImportService {
            store,
            base_currency: None,
            today: None,
        }
    }

    pub fn with_base_currency(mut self, currency: &str) -> Self {
        self.base_currency = Some(normalize_currency(currency));
        self
    }

    pub fn as_of(mut self, today: NaiveDate) -> Self {
        self.today = Some(today);
        self
    }

    /// Parses the file and builds a deterministic preview.
    pub fn preview(
        &self,
        content: &[u8],
        config: &ParseConfig,
        mapping: Option<&ImportMapping>,
        overrides: &[RowOverride],
    ) -> Result<ImportPreview> {
        let parsed = super::parse_file(content, config)?;
        let mapping = match mapping {
            Some(m) => m.clone(),
            // A Flex statement's columns are this crate's own, so detecting them off their
            // headers would be guessing at an answer we already know.
            // Our own file states the model's own wording, so detecting it off the headers
            // would be guessing at an answer the format already gives.
            None if super::canonical::is_canonical(content) => super::canonical::mapping(),
            None if ibflex::is_flex(content) => ibflex::mapping(),
            None => ImportMapping::detect_with_values(&parsed.headers, &parsed.rows),
        };

        let securities = self.store.list_securities()?;
        let accounts = self.store.list_accounts()?;
        let (known, loose) = self.known_fingerprints()?;
        let known_external = self.known_external()?;
        let context = ImportContext {
            securities: &securities,
            accounts: &accounts,
            known_external: &known_external,
            known_fingerprints: &known,
            known_loose: &loose,
            base_currency: self.base_currency.as_deref(),
            today: self.today,
        };
        Ok(build_preview(&parsed, &mapping, overrides, &context))
    }

    /// Commits selected rows in one storage transaction.
    pub fn commit(&self, preview: &ImportPreview, options: &ImportOptions) -> Result<ImportResult> {
        let mut result = ImportResult::default();

        let mut created: HashMap<String, String> = HashMap::new();
        let tx = self.store.conn().unchecked_transaction()?;

        // A preview is a snapshot of the store taken before this call. Between the two the
        // rows may already have been written — a second click on the same preview, another
        // import in between — so identity is checked again here, against the store, inside
        // the transaction. A row the preview already called a duplicate is excluded: writing
        // it is what `import_duplicates` was answered about.
        let (mut known, _) = self.known_fingerprints()?;

        for row in &preview.rows {
            let importable = match row.status {
                RowStatus::Ready => true,
                // A restatement is a correction of a row that is already there, so it is not
                // what the duplicate switch was answered about.
                RowStatus::Updated => true,
                RowStatus::Duplicate => options.import_duplicates,
                RowStatus::Similar => options.import_similar,
                RowStatus::UnknownSecurity => options.create_missing_securities,
                RowStatus::Ignored | RowStatus::Invalid => false,
            };
            if !importable {
                if row.status == RowStatus::Similar {
                    result.similar += 1;
                }
                result.skipped += 1;
                continue;
            }
            let Some(draft) = &row.draft else {
                result.skipped += 1;
                continue;
            };

            let mut draft = draft.clone();
            if draft.security_id.is_none()
                && let Some(symbol) = &draft.symbol
            {
                let key = normalize_alias(symbol);
                let id = match created.get(&key) {
                    Some(id) => id.clone(),
                    None => {
                        let plan = match preview.mapping.new_security_for(symbol) {
                            Some(found) => found.clone(),
                            None => {
                                let mut plan = SecurityDraft::unresolved(
                                    symbol,
                                    draft.security_name.as_deref(),
                                    &draft.currency,
                                );
                                plan.isin = plan.isin.or_else(|| draft.isin.clone());
                                plan.kind = options.new_security_kind;

                                if !is_isin(symbol) {
                                    plan.data_source = options.new_security_source.clone();
                                } else {
                                    result.problems.push(
                                        ImportProblem::row(
                                            ProblemCode::SecurityWithoutSource,
                                            row.number,
                                            format!(
                                                "instrument {symbol} was created without a quote source: \
                                             this is an ISIN, not a ticker — identify it on the \
                                             \"Instruments\" step"
                                            ),
                                        )
                                        .warn(),
                                    );
                                }
                                plan
                            }
                        };

                        let existing = self
                            .store
                            .find_security_by_symbol(&plan.symbol)?
                            .or(self.store.find_security_by_symbol(symbol)?);
                        // A ticker names a listing, an ISIN names the instrument. The stored row
                        // under this ticker carrying a different ISIN is a different company, and
                        // a ticker is unique, so neither reusing it nor creating beside it is
                        // possible: the row waits for a ticker of its own.
                        if let (Some(found), Some(wanted)) = (&existing, &plan.isin)
                            && found
                                .isin
                                .as_deref()
                                .is_some_and(|stored| normalize_alias(stored) != normalize_alias(wanted))
                        {
                            result.skipped += 1;
                            result.problems.push(ImportProblem::row(
                                ProblemCode::TickerIsinConflict,
                                row.number,
                                format!(
                                    "ticker {} is already in the database under ISIN {}, and this \
                                     row says {wanted} — give the new instrument a ticker of its own",
                                    found.symbol,
                                    found.isin.as_deref().unwrap_or("-")
                                ),
                            ));
                            continue;
                        }
                        let security = match existing {
                            Some(s) => s,
                            None => {
                                let s = plan.to_security();
                                self.store.save_security(&s)?;
                                result.created_securities.push(s.symbol.clone());
                                s
                            }
                        };
                        created.insert(key, security.id.clone());
                        security.id
                    }
                };
                draft.security_id = Some(id);
            }

            // A restatement writes over a row that is in the store already, so its content
            // may well match one — that is the point of it, not a reason to skip it.
            if !matches!(
                row.status,
                RowStatus::Duplicate | RowStatus::Updated | RowStatus::Similar
            ) && !known.insert(fingerprint(&draft))
            {
                result.skipped += 1;
                continue;
            }

            match draft.to_transaction() {
                Ok(transaction) => {
                    self.store.save_transaction(&transaction)?;
                    if row.status == RowStatus::Updated {
                        result.updated += 1;
                    } else {
                        result.imported += 1;
                    }
                }
                Err(e) => {
                    result.skipped += 1;
                    result.problems.push(ImportProblem::row(
                        ProblemCode::InvalidTransaction,
                        row.number,
                        e.to_string(),
                    ));
                }
            }
        }

        tx.commit()?;
        Ok(result)
    }

    pub fn preview_prices(
        &self,
        content: &[u8],
        config: &ParseConfig,
        mapping: Option<&PriceMapping>,
    ) -> Result<PriceImport> {
        let parsed = parse_csv(content, config)?;
        let mapping = match mapping {
            Some(m) => m.clone(),
            None => PriceMapping::detect(&parsed.headers),
        };
        let securities = self.store.list_securities()?;
        Ok(build_price_import(&parsed, &mapping, &securities))
    }

    pub fn commit_prices(&self, import: &PriceImport) -> Result<usize> {
        self.store.save_quotes(&import.quotes)
    }

    /// What the store knows by the broker's own identifier, for the rows that carry one.
    fn known_external(&self) -> Result<Vec<super::dedupe::KnownRow>> {
        let accounts: Vec<String> = self.store.list_accounts()?.into_iter().map(|a| a.id).collect();
        Ok(self
            .store
            .transactions_for_accounts(&accounts, None)?
            .iter()
            .filter_map(super::dedupe::KnownRow::of)
            .collect())
    }

    /// Both readings of what the store already holds: the exact content fingerprint, and the
    /// one that ignores what an operation is worth (`dedupe::loose_fingerprint_of`).
    fn known_fingerprints(&self) -> Result<(HashSet<String>, HashSet<String>)> {
        let accounts: Vec<String> = self.store.list_accounts()?.into_iter().map(|a| a.id).collect();
        let stored = self.store.transactions_for_accounts(&accounts, None)?;
        Ok((
            stored.iter().map(fingerprint_of).collect(),
            stored.iter().filter_map(loose_fingerprint_of).collect(),
        ))
    }
}
