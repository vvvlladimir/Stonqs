//! Reading one field out of one row. Each stage answers a single question, pushes whatever went
//! wrong onto the row's problems, and hands back `None` rather than a guess — a row missing a
//! date is a row the user has to look at, not a row dated today.

use super::cells::Cells;
use super::{AccountMapping, ImportContext, KindMapping, SymbolMapping};
use crate::import::checks::{self, Direction};
use crate::import::mapping::{AmountBasis, AmountSign, ImportField, normalize_alias};
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
    accounts_by_name: BTreeMap<String, &'a Account>,
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
            accounts_by_name: context
                .accounts
                .iter()
                .map(|a| (normalize_alias(&a.name), a))
                .collect(),
        }
    }

    /// Currency of the account the row's *money* lands on — a depot keeps none of its own, so it
    /// is asked of the deposit account behind it. `None` when the account is unknown.
    pub(super) fn settlement_currency(&self, account_id: &str) -> Option<&'a str> {
        let account = self.accounts_by_id.get(account_id)?;
        let settles_on = account.settlement_account_id();
        match self.accounts_by_id.get(settles_on) {
            Some(cash) => Some(cash.currency.as_str()),
            None => Some(account.currency.as_str()),
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
/// Counts the file's own wording without deciding anything by it — a rule has already said
/// what the operation is, and the wizard still lists what the file called it.
pub(super) fn count_kind(cells: &Cells, stats: &mut BTreeMap<String, KindMapping>) {
    let Some(value) = cells.get(ImportField::Kind) else {
        return;
    };
    let stat = stats.entry(value.to_string()).or_insert(KindMapping {
        value: value.to_string(),
        count: 0,
        kind: cells.mapping.kind_of(value),
        ignored: cells.mapping.is_ignored(value),
    });
    stat.count += 1;
}

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
    index: &Index,
    stats: &mut BTreeMap<String, AccountMapping>,
    problems: &mut Vec<ImportProblem>,
) -> Option<String> {
    match cells.get(ImportField::Account) {
        Some(value) => {
            // A name the portfolio already carries is not a guess: our own export writes
            // accounts by name, and a broker that prints one means the same thing by it.
            let mapped = cells
                .mapping
                .account_aliases
                .get(&normalize_alias(value))
                .cloned()
                .or_else(|| {
                    index
                        .accounts_by_name
                        .get(&normalize_alias(value))
                        .map(|a| a.id.clone())
                });
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

/// Puts back what a net amount had taken out of it. The model stores the trade's own value and
/// the charges beside it, so a file printing the sum of the two has to be undone here — only for
/// the charges in the row's own currency, since the others were never in that total.
pub(super) fn restore_gross(
    amounts: &mut Amounts,
    kind: Option<TransactionKind>,
    basis: AmountBasis,
    fee_is_local: bool,
    tax_is_local: bool,
) {
    let Some(kind) = kind else { return };
    let sign = Decimal::from(kind.charge_sign());
    if basis != AmountBasis::Net || sign.is_zero() {
        return;
    }
    let charges = if fee_is_local {
        amounts.fees.abs()
    } else {
        Decimal::ZERO
    } + if tax_is_local {
        amounts.taxes.abs()
    } else {
        Decimal::ZERO
    };
    if charges.is_zero() {
        return;
    }
    // The file's own sign carries direction and must survive the correction.
    let direction = if amounts.amount.is_sign_negative() {
        Decimal::NEGATIVE_ONE
    } else {
        Decimal::ONE
    };
    amounts.amount = direction * (amounts.amount.abs() - sign * charges);
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
    problems: &mut Vec<ImportProblem>,
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
    // The ISIN identifies the instrument and a ticker only one of its listings, so the ISIN is
    // asked first. Two brokers print the same ticker for different instruments often enough —
    // a local listing, a renamed company — and joining them silently writes one company's
    // trades into another's position.
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
