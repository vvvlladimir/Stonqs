use super::dedupe::{fingerprint, fingerprint_of};
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

    #[serde(default = "default_new_security_source")]
    pub new_security_source: Option<String>,

    pub import_duplicates: bool,
}

fn default_new_security_source() -> Option<String> {
    Some(crate::sources::DEFAULT_QUOTES.to_string())
}

impl Default for ImportOptions {
    fn default() -> Self {
        ImportOptions {
            create_missing_securities: true,
            new_security_kind: SecurityKind::Other,
            new_security_source: default_new_security_source(),
            import_duplicates: false,
        }
    }
}

/// Counts and diagnostics returned by a transaction commit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,

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
            None if ibflex::is_flex(content) => ibflex::mapping(),
            None => ImportMapping::detect_with_values(&parsed.headers, &parsed.rows),
        };

        let securities = self.store.list_securities()?;
        let accounts = self.store.list_accounts()?;
        let known = self.known_fingerprints()?;
        let context = ImportContext {
            securities: &securities,
            accounts: &accounts,
            known_fingerprints: &known,
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
        let mut known = self.known_fingerprints()?;

        for row in &preview.rows {
            let importable = match row.status {
                RowStatus::Ready => true,
                RowStatus::Duplicate => options.import_duplicates,
                RowStatus::UnknownSecurity => options.create_missing_securities,
                RowStatus::Ignored | RowStatus::Invalid => false,
            };
            if !importable {
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

            if row.status != RowStatus::Duplicate && !known.insert(fingerprint(&draft)) {
                result.skipped += 1;
                continue;
            }

            match draft.to_transaction() {
                Ok(transaction) => {
                    self.store.save_transaction(&transaction)?;
                    result.imported += 1;
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

    fn known_fingerprints(&self) -> Result<HashSet<String>> {
        let accounts: Vec<String> = self.store.list_accounts()?.into_iter().map(|a| a.id).collect();
        Ok(self
            .store
            .transactions_for_accounts(&accounts, None)?
            .iter()
            .map(fingerprint_of)
            .collect())
    }
}
