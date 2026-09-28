//! Reading one field of one row: problems are pushed, and `None` is returned rather than a guess.

use super::cells::Cells;
use super::{AccountMapping, ImportContext, KindMapping};
use crate::import::checks::{self, Direction};
use crate::import::mapping::{AmountBasis, AmountSign, ImportField, normalize_alias};
use crate::import::parse::{ImportProblem, ProblemCode, parse_date_any, parse_date_with};
use crate::model::{Account, AccountKind, Security, TransactionKind};
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

/// Counts the file's own wording for the wizard's kind list without deciding anything by it.
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
        // A rate is looked up by this code for the life of the portfolio, so a cell that cannot
        // be one is refused here rather than stored and missing market data forever.
        Some(c) if !crate::money::is_currency_code(&c) => {
            problems.push(
                ImportProblem::cell(
                    ProblemCode::SuspiciousCurrency,
                    cells.number,
                    cells.column(ImportField::Currency),
                    format!("{c:?} cannot be a currency code"),
                )
                .with("currency", &c),
            );
            None
        }
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

/// `amount` falls back to quantity × price; `signed_quantity` keeps the file's sign.
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
        .and_then(|v| crate::import::parse::parse_decimal(v, cells.decimal_separator))
        .filter(|r| crate::money::in_range(*r));

    let amount = match cells.get(ImportField::Amount) {
        Some(_) => cells.decimal(ImportField::Amount, problems),
        // Both factors are already within range, so only their product can still overflow.
        None => quantity.checked_mul(price).unwrap_or_else(|| {
            problems.push(
                ImportProblem::row(
                    ProblemCode::NumberOutOfRange,
                    cells.number,
                    format!("quantity × price ({quantity} × {price}) is too large to be an amount"),
                )
                .with("quantity", quantity)
                .with("price", price),
            );
            Decimal::ZERO
        }),
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

/// Undoes a net amount, for charges in the row's own currency only.
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

mod instrument;

pub(super) use instrument::instrument;
