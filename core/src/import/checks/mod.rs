use super::mapping::{AmountBasis, AmountSign};
use super::parse::{ImportProblem, ProblemCode};
use super::preview::{ImportRow, KindMapping};
use crate::model::TransactionKind;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

mod row;
pub use row::check_row;

/// Optional context for plausibility checks.
#[derive(Debug, Clone, Copy, Default)]
pub struct CheckContext<'a> {
    pub base_currency: Option<&'a str>,

    pub today: Option<NaiveDate>,

    /// The landing account's currency; a mismatch is legal but is said once per row.
    pub account_currency: Option<&'a str>,
}

/// Agreement between operation kinds and amount signs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SignVote {
    pub votes: usize,

    pub agree: usize,

    pub negative: usize,
}

impl SignVote {
    pub fn agreement(&self) -> f64 {
        if self.votes == 0 {
            return 0.0;
        }
        self.agree as f64 / self.votes as f64
    }

    pub fn both_signs(&self) -> bool {
        self.negative > 0 && self.negative < self.votes
    }
}

const MIN_VOTES: usize = 5;

const SIGNED_THRESHOLD: f64 = 0.9;

// Require enough rows and both signs before inferring a signed file.
pub fn count_vote(vote: &mut SignVote, kind: Option<TransactionKind>, amount: Decimal) {
    let Some(kind) = kind else { return };
    // A transfer's direction is arbitrary, so it flips by sign but never votes (`.claude/rules/import.md`).
    if kind.cash_sign() == 0 || kind.is_linked_side() || amount.is_zero() {
        return;
    }
    vote.votes += 1;
    let sign: i8 = if amount.is_sign_negative() { -1 } else { 1 };
    if sign < 0 {
        vote.negative += 1;
    }
    if sign == kind.cash_sign() {
        vote.agree += 1;
    }
}

/// Infers whether amount signs encode direction for the file.
pub fn decide_amount_sign(vote: SignVote) -> (AmountSign, Option<ImportProblem>) {
    let agreement = vote.agreement();
    if vote.votes < MIN_VOTES || !vote.both_signs() {
        return (AmountSign::Unsigned, None);
    }
    if agreement >= SIGNED_THRESHOLD {
        return (AmountSign::Signed, None);
    }
    let percent = (agreement * 100.0).round();
    let problem = ImportProblem::file(
        ProblemCode::AmountSignAmbiguous,
        format!(
            "the sign of the amount agrees with the transaction direction in only {percent} % of \
             rows ({} of {}). Direction is taken from the transaction kind; if it should come from \
             the sign, check the kind mapping and the amount column",
            vote.agree, vote.votes
        ),
    )
    .with("percent", percent)
    .with("agree", vote.agree)
    .with("votes", vote.votes)
    .warn();
    (AmountSign::Unsigned, Some(problem))
}

/// How many rows say the amount is the trade's own value and how many say it is what the
/// account actually moved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BasisVote {
    pub gross: usize,
    pub net: usize,
}

/// A broker rounds, so the two readings are compared with a little room: two cents plus a
/// fifth of a basis point, which separates a commission from a rounding difference.
fn about_equal(a: Decimal, b: Decimal) -> bool {
    let tolerance = dec!(0.02) + (a.abs() * dec!(0.0002));
    (a - b).abs() <= tolerance
}

/// One row's vote on whether the amount is net of charges; only rows with both a price and a charge vote.
pub fn count_basis_vote(
    vote: &mut BasisVote,
    charge_sign: i8,
    quantity: Decimal,
    price: Decimal,
    amount: Decimal,
    charges: Decimal,
) {
    // Both factors are in range and their product still may not be: a vote is a heuristic and
    // abstains rather than overflowing (`.claude/rules/import.md` — checks only ever warn).
    let Some(traded) = quantity.checked_mul(price) else {
        return;
    };
    if charge_sign == 0 || traded.is_zero() || charges.is_zero() || amount.is_zero() {
        return;
    }
    let amount = amount.abs();
    let Some(net) = traded.checked_add(Decimal::from(charge_sign) * charges) else {
        return;
    };
    if about_equal(amount, traded) {
        vote.gross += 1;
    } else if about_equal(amount, net) {
        vote.net += 1;
    }
}

const MIN_BASIS_VOTES: usize = 3;

/// Infers whether the amount column already has the row's charges in it. Gross is the answer
/// when nothing can be compared — it is the model's own convention, and the wizard says so.
pub fn decide_amount_basis(vote: BasisVote) -> (AmountBasis, Option<ImportProblem>) {
    let total = vote.gross + vote.net;
    if total < MIN_BASIS_VOTES {
        return (AmountBasis::Gross, None);
    }
    let (basis, agree) = if vote.net > vote.gross {
        (AmountBasis::Net, vote.net)
    } else {
        (AmountBasis::Gross, vote.gross)
    };
    let agreement = agree as f64 / total as f64;
    if agreement >= SIGNED_THRESHOLD {
        return (basis, None);
    }
    let percent = (agreement * 100.0).round();
    let problem = ImportProblem::file(
        ProblemCode::AmountBasisAmbiguous,
        format!(
            "the amount matches quantity × price in {} rows and the same total with charges in              {} — only {percent} % agree. It is read as {basis:?}; set it by hand if the file              means the other one",
            vote.gross, vote.net
        ),
    )
    .with("percent", percent)
    .with("gross", vote.gross)
    .with("net", vote.net)
    .warn();
    (basis, Some(problem))
}

/// Result of applying a file-level sign convention to one row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Keep,

    Flipped(TransactionKind),

    Conflict,
}

/// Direction from the number that carries it: the amount for cash, the quantity for shares.
pub fn resolve_direction(kind: TransactionKind, value: Decimal, sign: AmountSign) -> Direction {
    if sign != AmountSign::Signed || value.is_zero() {
        return Direction::Keep;
    }
    let expected = match (kind.cash_sign(), kind.affects_quantity()) {
        (0, false) => return Direction::Keep,
        (0, true) => kind.quantity_sign(),
        (cash, _) => cash,
    };
    let file_sign: i8 = if value.is_sign_negative() { -1 } else { 1 };
    if file_sign == expected {
        return Direction::Keep;
    }
    match kind.reversed() {
        Some(other) => Direction::Flipped(other),
        None => Direction::Conflict,
    }
}

const MAX_SPAN_YEARS: i32 = 50;

const SINGLE_KIND_MIN_ROWS: usize = 20;

/// A split is a warning, never a refusal: a stock really can double.
const SPLIT_MIN_RATIO: f64 = 1.8;
const SPLIT_MAX_RATIO: f64 = 20.0;
const SPLIT_ROUNDNESS: f64 = 0.01;
/// Beyond a quarter a whole factor is market, not a split.
const SPLIT_MAX_DAYS: i64 = 90;

/// Prices stepping by a whole factor between adjacent trades: a split applied mid-statement.
fn check_split_steps(rows: &[ImportRow]) -> Vec<ImportProblem> {
    let mut by_symbol: BTreeMap<&str, Vec<(NaiveDate, Decimal)>> = BTreeMap::new();
    for draft in rows.iter().filter_map(|r| r.draft.as_ref()) {
        if !draft.kind.affects_quantity() || draft.price.is_zero() {
            continue;
        }
        let Some(symbol) = draft.symbol.as_deref() else {
            continue;
        };
        by_symbol
            .entry(symbol)
            .or_default()
            .push((draft.date, draft.price.abs()));
    }

    let mut out = Vec::new();
    for (symbol, mut prices) in by_symbol {
        prices.sort_by_key(|(date, _)| *date);
        for pair in prices.windows(2) {
            let ((was, before), (date, after)) = (pair[0], pair[1]);
            if (date - was).num_days() > SPLIT_MAX_DAYS {
                continue;
            }
            let Some(ratio) = step_ratio(before, after) else {
                continue;
            };
            out.push(
                ImportProblem::file(
                    ProblemCode::PossibleSplit,
                    format!(
                        "{symbol} trades at {before} on {was} and at {after} on {date} — a factor \
                         of exactly {ratio}. If the broker applied a split in between, the \
                         quantities on either side mean different shares; record the split on the \
                         instrument instead of importing the change"
                    ),
                )
                .with("symbol", symbol)
                .with("before", before)
                .with("after", after)
                .with("was", was)
                .with("date", date)
                .with("ratio", ratio)
                .warn(),
            );
            break;
        }
    }
    out
}

/// The whole factor two prices differ by, or `None` when the step is small or not exact.
fn step_ratio(before: Decimal, after: Decimal) -> Option<i64> {
    let (before, after) = (f64::try_from(before).ok()?, f64::try_from(after).ok()?);
    if before <= 0.0 || after <= 0.0 {
        return None;
    }
    let ratio = if before > after {
        before / after
    } else {
        after / before
    };
    if !(SPLIT_MIN_RATIO..=SPLIT_MAX_RATIO).contains(&ratio) {
        return None;
    }
    let whole = ratio.round();
    ((ratio - whole).abs() / whole <= SPLIT_ROUNDNESS).then_some(whole as i64)
}

pub fn check_file(rows: &[ImportRow], kinds: &[KindMapping]) -> Vec<ImportProblem> {
    let mut out = check_split_steps(rows);

    if kinds.len() == 1 && rows.len() >= SINGLE_KIND_MIN_ROWS {
        out.push(
            ImportProblem::file(
                ProblemCode::SingleKindValue,
                format!(
                    "all {} rows carry the same transaction kind {:?} — \
                     check that the right column is assigned to the kind",
                    rows.len(),
                    kinds[0].value
                ),
            )
            .with("rows", rows.len())
            .with("kind", &kinds[0].value)
            .warn(),
        );
    }

    let dates: Vec<NaiveDate> = rows
        .iter()
        .filter_map(|r| r.draft.as_ref().map(|d| d.date))
        .collect();
    if let (Some(min), Some(max)) = (dates.iter().min(), dates.iter().max())
        && max.year() - min.year() > MAX_SPAN_YEARS
    {
        out.push(
            ImportProblem::file(
                ProblemCode::ImplausibleDateSpan,
                format!(
                    "the file's dates span {min} to {max} — the date format is most likely \
                     detected wrong"
                ),
            )
            .with("min", min)
            .with("max", max)
            .warn(),
        );
    }

    out
}

#[cfg(test)]
mod tests;
