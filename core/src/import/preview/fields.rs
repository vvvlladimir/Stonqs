//! Reading one field out of one row. Each stage answers a single question, pushes whatever went
//! wrong onto the row's problems, and hands back `None` rather than a guess — a row missing a
//! date is a row the user has to look at, not a row dated today.

use super::cells::Cells;
use super::{AccountMapping, ImportContext, KindMapping, SymbolMapping};
use crate::import::checks::{self, Direction};
use crate::import::mapping::{AmountSign, ImportField, normalize_alias};
use crate::import::parse::{ImportProblem, ProblemCode, parse_date_any, parse_date_with};
use crate::import::securities::SecurityDraft;
use crate::model::{Account, AccountKind, Security, TransactionKind, is_isin};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// What is already in the database, keyed the way a row looks it up. An ISIN identifies the
/// instrument and a ticker only the listing, so a foreign file may print either one.
pub(super) struct Index<'a> {
    by_symbol: BTreeMap<String, &'a Security>,
    by_isin: BTreeMap<String, &'a Security>,
    accounts_by_id: BTreeMap<&'a str, &'a Account>,
}

impl<'a> Index<'a> {
    pub fn of(context: &ImportContext<'a>) -> Self {
        Index {
            by_symbol: context
                .securities
                .iter()
                .map(|s| (normalize_alias(&s.symbol), s))
                .collect(),
            by_isin: context
                .securities
                .iter()
                .filter_map(|s| s.isin.as_ref().map(|i| (normalize_alias(i), s)))
                .collect(),
            accounts_by_id: context.accounts.iter().map(|a| (a.id.as_str(), a)).collect(),
        }
    }
}

pub(super) fn date(
    cells: &Cells,
    date_format: &Option<String>,
    problems: &mut Vec<ImportProblem>,
) -> Option<NaiveDate> {
    match (cells.get(ImportField::Date), date_format) {
        (Some(value), Some(format)) => match parse_date_with(value, format) {
            Some(d) => Some(d),
            // One column can carry two shapes: the broker changed its export format
            // mid-history. A recognised date is a warning, not a lost row.
            None => match parse_date_any(value) {
                Some((date, other)) => {
                    problems.push(
                        ImportProblem::cell(
                            ProblemCode::BadDate,
                            cells.number,
                            cells.column(ImportField::Date),
                            format!("the date {value:?} is not in format {format}; it parsed as {other}"),
                        )
                        .warn(),
                    );
                    Some(date)
                }
                None => {
                    problems.push(ImportProblem::cell(
                        ProblemCode::BadDate,
                        cells.number,
                        cells.column(ImportField::Date),
                        format!("the date {value:?} does not fit format {format}"),
                    ));
                    None
                }
            },
        },
        (Some(_), None) => {
            problems.push(ImportProblem::row(
                ProblemCode::BadDate,
                cells.number,
                "no date format is set",
            ));
            None
        }
        (None, _) => {
            problems.push(ImportProblem::row(
                ProblemCode::MissingValue,
                cells.number,
                "the date is empty",
            ));
            None
        }
    }
}

/// The operation, and whether the user chose to skip this wording entirely. An ignored value is
/// not an unknown one: it stays visible in the preview so the choice can be taken back.
pub(super) fn kind(
    cells: &Cells,
    stats: &mut BTreeMap<String, KindMapping>,
    problems: &mut Vec<ImportProblem>,
) -> (Option<TransactionKind>, bool) {
    match cells.get(ImportField::Kind) {
        Some(value) => {
            let resolved = cells.mapping.kind_of(value);
            let ignored = cells.mapping.is_ignored(value);
            let stat = stats.entry(value.to_string()).or_insert(KindMapping {
                value: value.to_string(),
                count: 0,
                kind: resolved,
                ignored,
            });
            stat.count += 1;
            if resolved.is_none() && !ignored {
                problems.push(ImportProblem::cell(
                    ProblemCode::UnknownKind,
                    cells.number,
                    cells.column(ImportField::Kind),
                    format!("unknown transaction kind {value:?} — map it in the mapping"),
                ));
            }
            (resolved, ignored)
        }
        None => {
            problems.push(ImportProblem::row(
                ProblemCode::MissingValue,
                cells.number,
                "the transaction kind is empty",
            ));
            (None, false)
        }
    }
}

pub(super) fn account(
    cells: &Cells,
    stats: &mut BTreeMap<String, AccountMapping>,
    problems: &mut Vec<ImportProblem>,
) -> Option<String> {
    match cells.get(ImportField::Account) {
        Some(value) => {
            let mapped = cells
                .mapping
                .account_aliases
                .get(&normalize_alias(value))
                .cloned();
            let resolved = mapped.clone().or_else(|| cells.mapping.account_id.clone());
            let stat = stats.entry(value.to_string()).or_insert(AccountMapping {
                value: value.to_string(),
                count: 0,
                account_id: mapped,
            });
            stat.count += 1;
            if resolved.is_none() {
                problems.push(ImportProblem::cell(
                    ProblemCode::UnknownAccount,
                    cells.number,
                    cells.column(ImportField::Account),
                    format!("account {value:?} is mapped to no account in the database"),
                ));
            }
            resolved
        }
        None => match &cells.mapping.account_id {
            Some(id) => Some(id.clone()),
            None => {
                problems.push(ImportProblem::row(
                    ProblemCode::MissingValue,
                    cells.number,
                    "no account is set",
                ));
                None
            }
        },
    }
}

/// Which of the account's two legs the row lands on: shares stay on the securities account,
/// cash settles on the deposit account behind it.
pub(super) fn settled(
    account_id: Option<String>,
    kind: Option<TransactionKind>,
    index: &Index,
    cells: &Cells,
    problems: &mut Vec<ImportProblem>,
) -> Option<String> {
    match (account_id, kind) {
        (Some(id), Some(kind)) => match index.accounts_by_id.get(id.as_str()) {
            Some(account) if kind.requires_security() && account.kind == AccountKind::Deposit => {
                problems.push(ImportProblem::row(
                    ProblemCode::WrongAccountKind,
                    cells.number,
                    format!(
                        "account {:?} is a cash account while the transaction carries an instrument — choose a securities account",
                        account.name
                    ),
                ));
                None
            }
            Some(account) if !kind.requires_security() => Some(account.settlement_account_id().to_string()),
            _ => Some(id),
        },
        (id, _) => id,
    }
}

/// A file without a currency column is normal — the broker's own currency is implied. The
/// account the row lands on carries it, so ask that before giving up on the row.
pub(super) fn currency(
    cells: &Cells,
    context: &ImportContext<'_>,
    account_id: Option<&str>,
    problems: &mut Vec<ImportProblem>,
) -> Option<Currency> {
    let found = cells
        .get(ImportField::Currency)
        .map(normalize_currency)
        .or_else(|| cells.mapping.default_currency.clone())
        .or_else(|| {
            let id = account_id?;
            let account = context.accounts.iter().find(|a| a.id == id)?;
            Some(normalize_currency(&account.currency))
        });
    match found {
        Some(c) => Some(c),
        None => {
            problems.push(ImportProblem::row(
                ProblemCode::MissingValue,
                cells.number,
                "no currency is given and no default is set",
            ));
            None
        }
    }
}

/// Every number the row carries. `amount` falls back to quantity × price for a file that prints
/// only the two, and `signed_quantity` keeps the sign the file gave because a share movement has
/// no cash to carry direction.
pub(super) struct Amounts {
    pub signed_quantity: Decimal,
    pub quantity: Decimal,
    pub price: Decimal,
    pub fees: Decimal,
    pub taxes: Decimal,
    pub fx_rate: Option<Decimal>,
    pub amount: Decimal,
}

pub(super) fn amounts(cells: &Cells, problems: &mut Vec<ImportProblem>) -> Amounts {
    let signed_quantity = cells.decimal(ImportField::Quantity, problems);
    let quantity = signed_quantity.abs();
    let price = cells.decimal(ImportField::Price, problems).abs();
    let fees = cells.decimal(ImportField::Fee, problems);
    let taxes = cells.decimal(ImportField::Tax, problems);
    let fx_rate = cells
        .get(ImportField::FxRate)
        .and_then(|v| crate::import::parse::parse_decimal(v, cells.decimal_separator));

    let amount = match cells.get(ImportField::Amount) {
        Some(_) => cells.decimal(ImportField::Amount, problems),
        None => quantity * price,
    };

    Amounts {
        signed_quantity,
        quantity,
        price,
        fees,
        taxes,
        fx_rate,
        amount,
    }
}

/// The currency a fee or a tax was billed in, kept only when it differs from the operation's —
/// a file that repeats the same code in every column says nothing new.
pub(super) fn charge_currency(cells: &Cells, field: ImportField, currency: &Currency) -> Option<Currency> {
    cells.get(field).map(normalize_currency).filter(|c| c != currency)
}

/// The type column says *what* happened and the sign says *which way*. One wording can span both
/// directions, so a row whose sign disagrees with its kind is flipped rather than refused.
pub(super) fn directed(
    kind: Option<TransactionKind>,
    amounts: &Amounts,
    amount_sign: AmountSign,
    cells: &Cells,
    problems: &mut Vec<ImportProblem>,
) -> Option<TransactionKind> {
    kind.map(|kind| {
        // Which number the file signs: a share movement has no cash to sign.
        let (value, field) = if kind.cash_sign() == 0 && !amounts.signed_quantity.is_zero() {
            (amounts.signed_quantity, ImportField::Quantity)
        } else {
            (amounts.amount, ImportField::Amount)
        };
        match checks::resolve_direction(kind, value, amount_sign) {
            Direction::Keep => kind,
            Direction::Flipped(other) => {
                problems.push(
                    ImportProblem::cell(
                        ProblemCode::DirectionFromSign,
                        cells.number,
                        cells.column(field),
                        format!(
                            "the sign of {value} is opposite to kind {kind:?}: \
                             the row was parsed as {other:?}"
                        ),
                    )
                    .warn(),
                );
                other
            }
            Direction::Conflict => {
                let amount = amounts.amount;
                problems.push(
                    ImportProblem::cell(
                        ProblemCode::DirectionConflict,
                        cells.number,
                        cells.column(ImportField::Amount),
                        format!(
                            "the sign of amount {amount} is opposite to kind {kind:?}, \
                             but this transaction has no reverse kind — check the row"
                        ),
                    )
                    .warn(),
                );
                kind
            }
        }
    })
}

/// What the row says the instrument is, and what in the database answers to it.
pub(super) struct Instrument {
    pub symbol: Option<String>,
    pub isin: Option<String>,
    pub file_name: Option<String>,
    pub security_id: Option<String>,
}

pub(super) fn instrument(
    cells: &Cells,
    index: &Index,
    kind: Option<TransactionKind>,
    stats: &mut BTreeMap<String, SymbolMapping>,
) -> Instrument {
    let raw_symbol = cells.get(ImportField::Symbol).map(|s| s.to_string());
    let symbol = raw_symbol
        .as_deref()
        .map(|s| cells.mapping.symbol_of(s).to_string());

    let isin = cells.get(ImportField::Isin).map(|s| s.to_string()).or_else(|| {
        raw_symbol
            .as_deref()
            .filter(|s| is_isin(s))
            .map(str::to_uppercase)
    });
    let file_name = cells.get(ImportField::Name).map(|s| s.to_string());
    let security = symbol
        .as_deref()
        .and_then(|s| index.by_symbol.get(&normalize_alias(s)))
        .or_else(|| {
            isin.as_deref()
                .and_then(|i| index.by_isin.get(&normalize_alias(i)))
        })
        .or_else(|| {
            symbol
                .as_deref()
                .and_then(|s| index.by_isin.get(&normalize_alias(s)))
        })
        .copied();

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
