//! One row, read in the order the fields depend on each other: the account decides the currency,
//! the kind decides which number carries the direction, and the direction decides whether the
//! instrument is required at all.

use super::cells::Cells;
use super::fields::{self, Index};
use super::status::{self, Dedupe};
use super::tally::Tallies;
use super::{ImportContext, ImportRow, TransactionDraft};
use crate::import::checks::{self, CheckContext};
use crate::import::mapping::{AmountSign, ImportField, ImportMapping};
use std::collections::BTreeMap;

/// What does not change from row to row.
pub(super) struct File<'a> {
    pub mapping: &'a ImportMapping,
    pub decimal_separator: char,
    pub date_format: Option<String>,
    pub amount_sign: AmountSign,
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
) -> ImportRow {
    let mut problems = Vec::new();
    let cells = Cells {
        raw: &raw,
        mapping: file.mapping,
        decimal_separator: file.decimal_separator,
        number,
    };

    let date = fields::date(&cells, &file.date_format, &mut problems);
    let (kind, ignored) = fields::kind(&cells, &mut tallies.kinds, &mut problems);
    let account_id = fields::account(&cells, &mut tallies.accounts, &mut problems);
    let account_id = fields::settled(account_id, kind, index, &cells, &mut problems);
    let currency = fields::currency(&cells, context, account_id.as_deref(), &mut problems);
    let amounts = fields::amounts(&cells, &mut problems);
    let kind = fields::directed(kind, &amounts, file.amount_sign, &cells, &mut problems);
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
            currency,
            fx_rate_to_base: amounts.fx_rate,
            link_id: cells.get(ImportField::LinkId).map(|s| s.to_string()),
            note: cells.get(ImportField::Note).map(|s| s.to_string()),
        }),
        _ => None,
    };

    status::check_transfer_keeps_its_instrument(&draft, number, &mut problems);
    let status = status::decide(number, ignored, &draft, dedupe, &mut problems);

    if let Some(d) = &draft
        && !matches!(status, super::RowStatus::Invalid | super::RowStatus::Ignored)
    {
        problems.extend(checks::check_row(number, d, &file.checks));
    }

    ImportRow {
        number,
        raw,
        draft,
        status,
        problems,
    }
}
