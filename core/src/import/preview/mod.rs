//! Turning a parsed file into a preview: what each row would become, what is already in the
//! database, and what the user still has to decide.
//!
//! `build_preview` is a pipeline of named stages and nothing else. It is **pure** — securities
//! and fingerprints arrive as slices, never a `Store` — because the same file has to preview the
//! same way twice; `ImportService` is the only place that reads the database
//! (`.claude/rules/import.md`).

mod cells;
mod fields;
mod link;
mod row;
mod status;
mod tally;

use super::checks::{self, BasisVote, CheckContext, SignVote};
use super::dedupe::KnownRow;
use super::mapping::{AmountBasis, AmountSign, ImportField, ImportMapping};
use super::parse::{ImportProblem, ParseConfig, ParsedCsv, ProblemCode, parse_decimal};
use super::securities::SecurityDraft;
use crate::error::{Error, Result};
use crate::model::{Account, Security, Transaction, TransactionKind};
use crate::money::Currency;
use chrono::NaiveDate;
use fields::Index;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use status::Dedupe;
use std::collections::{BTreeMap, HashSet};
use tally::Tallies;

/// Transaction assembled from one row but not yet committed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionDraft {
    pub account_id: String,
    pub kind: TransactionKind,
    pub date: NaiveDate,

    pub symbol: Option<String>,
    pub isin: Option<String>,

    pub security_name: Option<String>,

    pub security_id: Option<String>,
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fees: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes: Decimal,
    pub currency: Currency,
    /// Set only when the file bills the charge somewhere other than the operation itself.
    #[serde(default)]
    pub fee_currency: Option<Currency>,
    #[serde(default)]
    pub tax_currency: Option<Currency>,
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub fx_rate_to_base: Option<Decimal>,
    pub link_id: Option<String>,
    /// The broker's own identifier for the row, when the file carries one.
    #[serde(default)]
    pub external_id: Option<String>,
    /// The stored operation this row restates, named by the external id it repeats. Set by the
    /// preview, never by a file: a row cannot claim which database row it replaces.
    #[serde(default)]
    pub replaces: Option<String>,
    pub note: Option<String>,
}

impl TransactionDraft {
    /// Converts the draft and validates the resulting transaction.
    pub fn to_transaction(&self) -> Result<Transaction> {
        if self.kind.requires_security() && self.security_id.is_none() {
            return Err(Error::Invalid(format!(
                "{:?} needs an instrument, and {} is not in the database",
                self.kind,
                self.symbol.as_deref().unwrap_or("no symbol given")
            )));
        }
        let mut t = Transaction::cash(
            &self.account_id,
            self.kind,
            self.date,
            self.amount,
            &self.currency,
        );
        t.security_id = self.security_id.clone();
        t.quantity = self.quantity;
        t.price = self.price;
        t.fees = self.fees;
        t.taxes = self.taxes;
        t.fee_currency = self.fee_currency.clone().filter(|c| *c != t.currency);
        t.tax_currency = self.tax_currency.clone().filter(|c| *c != t.currency);
        t.fx_rate_to_base = self.fx_rate_to_base;
        t.link_id = self.link_id.clone();
        t.external_id = self.external_id.clone();
        // Restating an operation writes the row that is already there; a new one gets its own id.
        if let Some(id) = &self.replaces {
            t.id = id.clone();
        }
        t.note = self.note.clone();
        t.validate()?;
        Ok(t)
    }
}

/// Import disposition shown for each row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RowStatus {
    Ready,

    Duplicate,

    /// The broker's own identifier is already in the database against different values: the
    /// statement was restated, so importing this row replaces the one that is there.
    Updated,

    UnknownSecurity,

    /// Its operation value is on the skip list: not a problem, just not imported.
    Ignored,

    Invalid,
}

/// One source row after overrides and model parsing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportRow {
    pub number: usize,

    pub raw: BTreeMap<String, String>,
    pub draft: Option<TransactionDraft>,
    pub status: RowStatus,
    pub problems: Vec<ImportProblem>,
}

/// Replaces one source cell before parsing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowOverride {
    pub number: usize,
    pub field: ImportField,
    pub value: String,
}

impl RowOverride {
    pub fn new(number: usize, field: ImportField, value: impl Into<String>) -> Self {
        RowOverride {
            number,
            field,
            value: value.into(),
        }
    }
}

/// Reference data used to build a deterministic preview.
pub struct ImportContext<'a> {
    pub securities: &'a [Security],

    pub accounts: &'a [Account],

    /// Operations the database already knows by the broker's own identifier.
    pub known_external: &'a [KnownRow],

    pub known_fingerprints: &'a HashSet<String>,

    pub base_currency: Option<&'a str>,
    pub today: Option<NaiveDate>,
}

impl Default for ImportContext<'_> {
    fn default() -> Self {
        static NO_SECURITIES: &[Security] = &[];
        static NO_ACCOUNTS: &[Account] = &[];
        static NO_EXTERNAL: &[KnownRow] = &[];

        ImportContext {
            securities: NO_SECURITIES,
            accounts: NO_ACCOUNTS,
            known_external: NO_EXTERNAL,
            known_fingerprints: Box::leak(Box::new(HashSet::new())),
            base_currency: None,
            today: None,
        }
    }
}

/// One distinct operation-kind value and its mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KindMapping {
    pub value: String,
    pub count: usize,

    pub kind: Option<TransactionKind>,

    /// The user chose to skip these rows instead of mapping them.
    #[serde(default)]
    pub ignored: bool,
}

/// One distinct account value and its mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountMapping {
    pub value: String,
    pub count: usize,
    pub account_id: Option<String>,
}

/// One distinct security value and its resolution plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolMapping {
    pub value: String,

    pub resolved: String,

    pub isin: Option<String>,

    pub file_name: Option<String>,
    pub count: usize,

    pub security_id: Option<String>,
    pub name: Option<String>,
    pub currency: Option<Currency>,

    pub planned: Option<SecurityDraft>,

    pub required: bool,
}

/// Counts displayed in the import summary.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportSummary {
    pub total: usize,
    pub ready: usize,
    pub duplicates: usize,
    #[serde(default)]
    pub updated: usize,
    pub unknown_securities: usize,
    pub ignored: usize,
    pub invalid: usize,

    pub warnings: usize,
}

/// Complete transaction import preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportPreview {
    pub config: ParseConfig,
    pub mapping: ImportMapping,
    pub rows: Vec<ImportRow>,

    pub problems: Vec<ImportProblem>,

    pub kinds: Vec<KindMapping>,

    pub symbols: Vec<SymbolMapping>,

    pub accounts: Vec<AccountMapping>,

    pub amount_sign: AmountSign,
    /// Whether the file's amount column was read as gross or as net of the row's charges.
    pub amount_basis: AmountBasis,
    pub summary: ImportSummary,
}

impl ImportPreview {
    pub fn ready_rows(&self) -> impl Iterator<Item = &ImportRow> {
        self.rows.iter().filter(|r| r.status == RowStatus::Ready)
    }

    pub fn has_anything_to_import(&self) -> bool {
        self.summary.ready > 0 || self.summary.updated > 0 || self.summary.unknown_securities > 0
    }

    pub fn unknown_kinds(&self) -> Vec<&str> {
        self.kinds
            .iter()
            .filter(|k| k.kind.is_none() && !k.ignored)
            .map(|k| k.value.as_str())
            .collect()
    }

    pub fn unknown_symbols(&self) -> Vec<&str> {
        self.symbols
            .iter()
            .filter(|s| s.required && s.security_id.is_none())
            .map(|s| s.resolved.as_str())
            .collect()
    }

    pub fn unresolved_symbols(&self) -> Vec<&SymbolMapping> {
        self.symbols
            .iter()
            .filter(|s| s.required && s.security_id.is_none() && s.planned.is_none())
            .collect()
    }

    pub fn unmapped_accounts(&self) -> Vec<&str> {
        self.accounts
            .iter()
            .filter(|a| a.account_id.is_none())
            .map(|a| a.value.as_str())
            .collect()
    }
}

/// Builds a preview without writing to storage or using the network.
pub fn build_preview(
    parsed: &ParsedCsv,
    mapping: &ImportMapping,
    overrides: &[RowOverride],
    context: &ImportContext<'_>,
) -> ImportPreview {
    // A saved layout answers the wordings it was made from; the file may hold others. The
    // keyword dictionary fills those in, so a template stays useful on the next export.
    let mapping = &match mapping.column(ImportField::Kind) {
        Some(column) => mapping
            .clone()
            .with_detected_kinds(parsed.column_values(column).into_iter()),
        None => mapping.clone(),
    };

    let mut problems: Vec<ImportProblem> = mapping
        .missing_required()
        .into_iter()
        .map(|field| {
            ImportProblem::file(
                ProblemCode::MissingColumn,
                format!("no column is assigned to the required field {field:?}"),
            )
        })
        .collect();

    let decimal_separator = parsed.config.decimal_separator.unwrap_or('.');
    let raw_rows = cells::rows_with_overrides(parsed, mapping, overrides);

    let amount_sign = match mapping.amount_sign {
        Some(chosen) => chosen,
        None => {
            let (sign, problem) = vote_on_signs(&raw_rows, mapping, decimal_separator);
            problems.extend(problem);
            sign
        }
    };

    let amount_basis = match mapping.amount_basis {
        Some(chosen) => chosen,
        None => {
            let (basis, problem) = vote_on_basis(&raw_rows, mapping, decimal_separator);
            problems.extend(problem);
            basis
        }
    };

    let file = row::File {
        mapping,
        decimal_separator,
        date_format: parsed.config.date_format.clone(),
        amount_sign,
        amount_basis,
        checks: CheckContext {
            base_currency: context.base_currency,
            today: context.today,
        },
    };
    let index = Index::of(context);
    let mut tallies = Tallies::default();
    let mut dedupe = Dedupe::against(context.known_fingerprints, context.known_external);

    let mut rows = Vec::with_capacity(parsed.rows.len());
    for (offset, raw) in raw_rows.into_iter().enumerate() {
        rows.push(row::read(
            offset + 1,
            raw,
            &file,
            context,
            &index,
            &mut tallies,
            &mut dedupe,
        ));
    }

    link::internal_transfers(&mut rows, mapping);

    problems.extend(parsed.problems.iter().cloned());
    let (kinds, symbols, accounts) = tallies.into_sorted();
    problems.extend(checks::check_file(&rows, &kinds));

    ImportPreview {
        config: parsed.config.clone(),
        mapping: mapping.clone(),
        summary: status::summarize(&rows),
        rows,
        problems,
        kinds,
        symbols,
        accounts,
        amount_sign,
        amount_basis,
    }
}

/// Whether the amount column is the trade's own value or what the account moved. Asked of the
/// file, like the sign: one row where the two readings differ by a commission answers it, and a
/// broker is consistent about which of the two it prints.
fn vote_on_basis(
    raw_rows: &[BTreeMap<String, String>],
    mapping: &ImportMapping,
    decimal_separator: char,
) -> (AmountBasis, Option<ImportProblem>) {
    let number = |raw: &BTreeMap<String, String>, field| {
        cells::cell_of(raw, mapping, field)
            .and_then(|v| parse_decimal(v, decimal_separator))
            .unwrap_or(Decimal::ZERO)
    };
    let mut vote = BasisVote::default();
    for raw in raw_rows {
        let Some(kind) = cells::cell_of(raw, mapping, ImportField::Kind).and_then(|v| mapping.kind_of(v))
        else {
            continue;
        };
        // A charge billed in another currency is not in this total and cannot be compared
        // against it, so the row abstains for that charge rather than voting on a mismatch.
        let charge = |amount_field, currency_field| match cells::cell_of(raw, mapping, currency_field) {
            Some(_) => Decimal::ZERO,
            None => number(raw, amount_field),
        };
        let charges = charge(ImportField::Fee, ImportField::FeeCurrency)
            + charge(ImportField::Tax, ImportField::TaxCurrency);
        checks::count_basis_vote(
            &mut vote,
            kind.charge_sign(),
            number(raw, ImportField::Quantity).abs(),
            number(raw, ImportField::Price).abs(),
            number(raw, ImportField::Amount),
            charges.abs(),
        );
    }
    checks::decide_amount_basis(vote)
}

/// Whether the file carries direction in the sign of its amounts. The question is asked of the
/// whole file rather than of a row: one type value can span both directions, and it is the
/// correlation across every cash-moving row that answers it (`.claude/rules/import.md`).
fn vote_on_signs(
    raw_rows: &[BTreeMap<String, String>],
    mapping: &ImportMapping,
    decimal_separator: char,
) -> (AmountSign, Option<ImportProblem>) {
    let mut vote = SignVote::default();
    for raw in raw_rows {
        let kind = cells::cell_of(raw, mapping, ImportField::Kind).and_then(|v| mapping.kind_of(v));
        let amount = cells::cell_of(raw, mapping, ImportField::Amount)
            .and_then(|v| parse_decimal(v, decimal_separator))
            .unwrap_or(Decimal::ZERO);
        checks::count_vote(&mut vote, kind, amount);
    }
    checks::decide_amount_sign(vote)
}
