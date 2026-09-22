//! One row, read in the order the fields depend on each other: the account decides the currency,
//! the kind decides which number carries the direction, and the direction decides whether the
//! instrument is required at all.
//!
//! A row is usually one operation and sometimes several: a rule can say that a reinvested
//! dividend is an income *and* a purchase (ADR-0067). The parts share the file row's number,
//! because they are one line of the file, and are read the same way — a rule changes what a row
//! says, never how it is read.

use super::cells::Cells;
use super::fields::{self, Index};
use super::status::{self, Dedupe};
use super::tally::Tallies;
use super::{ImportContext, ImportRow, TransactionDraft};
use crate::import::checks::{self, CheckContext};
use crate::import::mapping::{
    AmountBasis, AmountSign, Emit, ImportField, ImportMapping, ImportRule, first_match, resolve,
};
use crate::model::TransactionKind;
use std::collections::BTreeMap;

/// What does not change from row to row.
pub(super) struct File<'a> {
    pub mapping: &'a ImportMapping,
    pub decimal_separator: char,
    pub date_format: Option<String>,
    pub amount_sign: AmountSign,
    pub amount_basis: AmountBasis,
    pub checks: CheckContext<'a>,
}

pub(super) fn read(
    number: usize,
    raw: BTreeMap<String, String>,
    file: &File<'_>,
    context: &ImportContext<'_>,
    index: &Index<'_>,
    tallies: &mut Tallies,
    dedupe: &mut Dedupe<'_>,
) -> Vec<ImportRow> {
    let plain = Cells {
        raw: &raw,
        mapping: file.mapping,
        decimal_separator: file.decimal_separator,
        number,
        emitted: BTreeMap::new(),
    };
    let value = |field: ImportField| plain.get(field).map(str::to_string);

    let Some(rule) = first_match(&file.mapping.rules, &value, file.decimal_separator) else {
        return vec![one(number, 1, &raw, None, file, context, index, tallies, dedupe)];
    };
    if rule.emit.is_empty() {
        // A rule that produces nothing is a line the file prints and the ledger has no room
        // for. It stays visible and counted, exactly like a wording on the skip list.
        return vec![skipped(number, raw)];
    }

    let parts = rule.emit.len();
    rule.emit
        .iter()
        .enumerate()
        .map(|(at, emit)| {
            let emitted = emitted_values(emit, rule, &value, number, at, parts);
            one(
                number,
                at + 1,
                &raw,
                Some((emit.kind, emitted)),
                file,
                context,
                index,
                tallies,
                dedupe,
            )
        })
        .collect()
}

/// What one emitted operation says instead of the row it came from. Two values are the rule's
/// own rather than the author's: the broker's identifier gains the part's number, so a
/// restatement still recognises each half (ADR-0065), and a linked pair shares a link id
/// derived from the row, so the preview stays reproducible.
fn emitted_values(
    emit: &Emit,
    rule: &ImportRule,
    value: &dyn Fn(ImportField) -> Option<String>,
    number: usize,
    at: usize,
    parts: usize,
) -> BTreeMap<ImportField, String> {
    let mut emitted: BTreeMap<ImportField, String> = emit
        .set
        .iter()
        .map(|(field, template)| (*field, resolve(template, value)))
        .collect();

    if parts > 1 {
        if let Some(id) = value(ImportField::ExternalId) {
            emitted
                .entry(ImportField::ExternalId)
                .or_insert_with(|| format!("{id}#{}", at + 1));
        }
        if rule.link {
            emitted
                .entry(ImportField::LinkId)
                .or_insert_with(|| format!("rule:{number}"));
        }
    }
    emitted
}

/// A row the file printed and no operation answers to.
fn skipped(number: usize, raw: BTreeMap<String, String>) -> ImportRow {
    ImportRow {
        number,
        part: 1,
        raw,
        draft: None,
        status: super::RowStatus::Ignored,
        problems: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn one(
    number: usize,
    part: usize,
    raw: &BTreeMap<String, String>,
    emitted: Option<(TransactionKind, BTreeMap<ImportField, String>)>,
    file: &File<'_>,
    context: &ImportContext<'_>,
    index: &Index<'_>,
    tallies: &mut Tallies,
    dedupe: &mut Dedupe<'_>,
) -> ImportRow {
    let mut problems = Vec::new();
    let (ruled_kind, overrides) = match emitted {
        Some((kind, values)) => (Some(kind), values),
        None => (None, BTreeMap::new()),
    };
    let cells = Cells {
        raw,
        mapping: file.mapping,
        decimal_separator: file.decimal_separator,
        number,
        emitted: overrides,
    };

    let date = fields::date(&cells, &file.date_format, &mut problems);
    // A rule has already said what the operation is; the wording is still counted, so the
    // wizard shows what the file called it.
    let (kind, ignored) = match ruled_kind {
        Some(kind) => {
            fields::count_kind(&cells, &mut tallies.kinds);
            (Some(kind), false)
        }
        None => fields::kind(&cells, &mut tallies.kinds, &mut problems),
    };
    let account_id = fields::account(&cells, index, &mut tallies.accounts, &mut problems);
    let account_id = fields::settled(account_id, kind, index, &cells, &mut problems);
    let currency = fields::currency(&cells, context, account_id.as_deref(), &mut problems);
    let mut amounts = fields::amounts(&cells, &mut problems);
    let fee_currency = currency
        .as_ref()
        .and_then(|c| fields::charge_currency(&cells, ImportField::FeeCurrency, c));
    let tax_currency = currency
        .as_ref()
        .and_then(|c| fields::charge_currency(&cells, ImportField::TaxCurrency, c));
    fields::restore_gross(
        &mut amounts,
        kind,
        file.amount_basis,
        fee_currency.is_none(),
        tax_currency.is_none(),
    );
    // A rule decided the direction itself, so the sign no longer votes on it: flipping what
    // the rule chose would answer a question the author already answered.
    let kind = match ruled_kind {
        Some(kind) => Some(kind),
        None => fields::directed(kind, &amounts, file.amount_sign, &cells, &mut problems),
    };
    let instrument = fields::instrument(&cells, index, kind, &mut tallies.symbols);

    let draft = match (date, kind, account_id, currency) {
        (Some(date), Some(kind), Some(account_id), Some(currency)) => Some(TransactionDraft {
            account_id,
            kind,
            date,
            symbol: instrument.symbol,
            isin: instrument.isin,
            security_name: instrument.file_name,
            security_id: instrument.security_id,
            quantity: amounts.quantity,
            price: amounts.price,
            amount: amounts.amount.abs(),
            fees: amounts.fees.abs(),
            taxes: amounts.taxes.abs(),
            fee_currency,
            tax_currency,
            currency,
            fx_rate_to_base: amounts.fx_rate,
            link_id: cells.get(ImportField::LinkId).map(|s| s.to_string()),
            external_id: cells.get(ImportField::ExternalId).map(|s| s.to_string()),
            replaces: None,
            note: cells.get(ImportField::Note).map(|s| s.to_string()),
        }),
        _ => None,
    };

    let mut draft = draft;
    status::check_transfer_keeps_its_instrument(&draft, number, &mut problems);
    let status = status::decide(number, ignored, &mut draft, dedupe, &mut problems);

    if let Some(d) = &draft
        && !matches!(status, super::RowStatus::Invalid | super::RowStatus::Ignored)
    {
        problems.extend(checks::check_row(number, d, &file.checks));
    }

    ImportRow {
        number,
        part,
        raw: raw.clone(),
        draft,
        status,
        problems,
    }
}
