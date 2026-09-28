//! Which instrument a row names, and what in the database answers to it. The ISIN is asked
//! first: a ticker names a listing, and the same one is reused by different companies.

use super::Index;
use crate::import::mapping::{ImportField, normalize_alias};
use crate::import::parse::{ImportProblem, ProblemCode, is_placeholder};
use crate::import::preview::SymbolMapping;
use crate::import::preview::cells::Cells;
use crate::import::securities::SecurityDraft;
use crate::model::{TransactionKind, is_isin};
use std::collections::BTreeMap;

/// The row's ISIN when it passes its own check digit. One that does not is a mistyped or shifted
/// cell: said once and left out, so the row is matched by its ticker and nothing stores an
/// identifier no registry answers to.
fn checked_isin(cells: &Cells, problems: &mut Vec<ImportProblem>) -> Option<String> {
    let value = cells.get(ImportField::Isin).filter(|s| !is_placeholder(s))?;
    if is_isin(value) {
        return Some(value.to_string());
    }
    problems.push(
        ImportProblem::cell(
            ProblemCode::InvalidIsin,
            cells.number,
            cells.column(ImportField::Isin),
            format!("{value:?} is not a valid ISIN: its check digit does not match"),
        )
        .with("isin", value)
        .warn(),
    );
    None
}

/// What the row says the instrument is, and what in the database answers to it.
pub(crate) struct Instrument {
    pub symbol: Option<String>,
    pub isin: Option<String>,
    pub file_name: Option<String>,
    pub security_id: Option<String>,
}

pub(crate) fn instrument(
    cells: &Cells,
    index: &Index,
    kind: Option<TransactionKind>,
    stats: &mut BTreeMap<String, SymbolMapping>,
    problems: &mut Vec<ImportProblem>,
) -> Instrument {
    // A broker prints "-" where a row has no instrument; taking it at face value creates an
    // instrument whose ticker is a dash, as `is_placeholder` already says of an absent number.
    let raw_symbol = cells
        .get(ImportField::Symbol)
        .filter(|s| !is_placeholder(s))
        .map(|s| s.to_string());
    let symbol = raw_symbol
        .as_deref()
        .map(|s| cells.mapping.symbol_of(s).to_string());

    let isin = checked_isin(cells, problems).or_else(|| {
        raw_symbol
            .as_deref()
            .filter(|s| is_isin(s))
            .map(str::to_uppercase)
    });
    let file_name = cells
        .get(ImportField::Name)
        .filter(|s| !is_placeholder(s))
        .map(|s| s.to_string());
    // ISIN before ticker: the same ticker names different companies often enough.
    let by_isin = isin
        .as_deref()
        .and_then(|i| index.by_isin.get(&normalize_alias(i)))
        .or_else(|| {
            symbol
                .as_deref()
                .and_then(|s| index.by_isin.get(&normalize_alias(s)))
        })
        .copied();
    let by_symbol = symbol
        .as_deref()
        .and_then(|s| index.by_symbol.get(&normalize_alias(s)))
        .copied();

    let security = match (by_isin, by_symbol) {
        (Some(found), _) => Some(found),
        // The ticker is known and carries another ISIN: not this instrument. The row keeps its
        // own identifiers and is treated as a new instrument rather than joined to that one.
        (None, Some(found))
            if isin.is_some()
                && found.isin.as_deref().is_some_and(|stored| {
                    normalize_alias(stored) != normalize_alias(isin.as_deref().unwrap_or(""))
                }) =>
        {
            problems.push(
                ImportProblem::row(
                    ProblemCode::TickerIsinConflict,
                    cells.number,
                    format!(
                        "ticker {} is already in the database under ISIN {}, and this row says {} \
                         — they are two instruments and one ticker cannot name both. Give this \
                         one a ticker of its own on the \"Instruments\" step",
                        found.symbol,
                        found.isin.as_deref().unwrap_or("-"),
                        isin.as_deref().unwrap_or("-")
                    ),
                )
                .with("symbol", &found.symbol)
                .with("stored", found.isin.as_deref().unwrap_or("-"))
                .with("isin", isin.as_deref().unwrap_or("-")),
            );
            None
        }
        (None, found) => found,
    };

    let planned: Option<SecurityDraft> = raw_symbol
        .as_deref()
        .filter(|_| security.is_none())
        .and_then(|s| cells.mapping.new_security_for(s))
        .cloned();

    if let Some(raw) = &raw_symbol {
        let resolved = symbol.clone().unwrap_or_else(|| raw.clone());
        let stat = stats.entry(raw.clone()).or_insert(SymbolMapping {
            value: raw.clone(),
            resolved: resolved.clone(),
            isin: isin.clone(),
            file_name: file_name.clone(),
            count: 0,
            security_id: security.map(|s| s.id.clone()),
            name: security
                .map(|s| s.name.clone())
                .or_else(|| planned.as_ref().map(|p| p.name.clone())),
            currency: security
                .map(|s| s.currency.clone())
                .or_else(|| planned.as_ref().map(|p| p.currency.clone())),
            planned: planned.clone(),
            required: false,
        });
        stat.count += 1;

        stat.required |= kind.is_some_and(|k| k.requires_security());
    }

    Instrument {
        symbol,
        isin,
        file_name,
        security_id: security.map(|s| s.id.clone()),
    }
}
