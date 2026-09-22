use super::mapping::{AmountBasis, AmountSign};
use super::parse::{ImportProblem, ProblemCode};
use super::preview::{ImportRow, KindMapping, TransactionDraft};
use crate::model::TransactionKind;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::BTreeMap;

/// Optional context for plausibility checks.
#[derive(Debug, Clone, Copy, Default)]
pub struct CheckContext<'a> {
    pub base_currency: Option<&'a str>,

    pub today: Option<NaiveDate>,

    /// Currency of the account this row lands on, when one was resolved. A row denominated in
    /// something else is legal — a multi-currency account is a real thing — but it is also what
    /// a mis-mapped currency column looks like, so it is said once per row.
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
    // A transfer's nominal direction is arbitrary — `TransferIn` is merely the reversible
    // side of the pair — so it can neither confirm nor deny that the sign carries direction.
    // It still flips by sign; it just does not vote. The mirror of Buy/Sell, which vote but
    // never flip. Without this a file of currency conversions votes against itself: every
    // second leg is negative by construction.
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

/// Counts one row's answer to "is the amount net of this row's charges". Only a row that has
/// both a quantity × price to compare against and a charge to find can answer at all, and a
/// row where the two readings coincide (no charge) says nothing.
pub fn count_basis_vote(
    vote: &mut BasisVote,
    charge_sign: i8,
    quantity: Decimal,
    price: Decimal,
    amount: Decimal,
    charges: Decimal,
) {
    let traded = quantity * price;
    if charge_sign == 0 || traded.is_zero() || charges.is_zero() || amount.is_zero() {
        return;
    }
    let amount = amount.abs();
    let net = traded + Decimal::from(charge_sign) * charges;
    if about_equal(amount, traded) {
        vote.gross += 1;
    } else if about_equal(amount, net) {
        vote.net += 1;
    }
}

/// The smallest number of rows that may decide the file's reading. Lower than the sign vote's:
/// a file that prints a commission on every trade answers this in a handful of rows, while a
/// sign convention is a claim about every cash row there is.
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

/// Resolves a row direction from the number that carries it. For a cash operation that is
/// the amount; a share movement carries no cash, so its direction is the quantity's — a
/// split printed as "-1 share out, +8 shares in" is two directions of one wording.
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

/// How far `amount` may sit from quantity × price before it is worth saying so. A broker prints
/// the unit price rounded, and that rounding multiplies by the quantity — which is why the
/// allowance is mostly per unit rather than a flat percentage: a flat 1 % hides a hundred euros
/// on a ten-thousand-euro trade, and a flat cent flags every thousand-unit crypto order.
const AMOUNT_TOLERANCE_PER_UNIT: Decimal = dec!(0.005);
const AMOUNT_TOLERANCE_RATIO: Decimal = dec!(0.002);
const AMOUNT_TOLERANCE_ABSOLUTE: Decimal = dec!(0.01);

fn amount_tolerance(quantity: Decimal, expected: Decimal) -> Decimal {
    AMOUNT_TOLERANCE_ABSOLUTE
        + quantity.abs() * AMOUNT_TOLERANCE_PER_UNIT
        + expected.abs() * AMOUNT_TOLERANCE_RATIO
}

/// Emits non-blocking plausibility diagnostics for one draft.
pub fn check_row(number: usize, draft: &TransactionDraft, context: &CheckContext<'_>) -> Vec<ImportProblem> {
    let mut out = Vec::new();

    if draft.kind.affects_quantity()
        && !draft.quantity.is_zero()
        && !draft.price.is_zero()
        && !draft.amount.is_zero()
    {
        let expected = draft.quantity * draft.price;
        if (draft.amount - expected).abs() > amount_tolerance(draft.quantity, expected) {
            out.push(
                ImportProblem::row(
                    ProblemCode::AmountVsQuantityPrice,
                    number,
                    format!(
                        "the amount {} does not match quantity × price ({} × {} = {}). \
                         Check the columns and the decimal separator",
                        draft.amount,
                        draft.quantity,
                        draft.price,
                        expected.round_dp(4)
                    ),
                )
                .with("amount", draft.amount)
                .with("quantity", draft.quantity)
                .with("price", draft.price)
                .with("expected", expected.round_dp(4))
                .warn(),
            );
        }
    }

    let charges = draft.fees + draft.taxes;
    if !draft.amount.is_zero() && charges > draft.amount {
        out.push(
            ImportProblem::row(
                ProblemCode::FeeExceedsAmount,
                number,
                format!(
                    "commission and tax ({charges}) exceed the transaction amount ({}) — \
                     the columns look swapped",
                    draft.amount
                ),
            )
            .with("charges", charges)
            .with("amount", draft.amount)
            .warn(),
        );
    }

    if let (Some(_), Some(base)) = (draft.fx_rate_to_base, context.base_currency)
        && draft.currency == base
    {
        out.push(
            ImportProblem::row(
                ProblemCode::FxRateOnBaseCurrency,
                number,
                format!(
                    "an FX rate is given while the transaction currency {base} equals the base \
                     currency: it is not applied. This column usually holds the source currency's rate"
                ),
            )
            .with("base", base)
            .warn(),
        );
    }

    if draft.kind.cash_sign() != 0 && draft.amount.is_zero() && draft.fees.is_zero() && draft.taxes.is_zero()
    {
        out.push(
            ImportProblem::row(
                ProblemCode::ZeroAmount,
                number,
                "a transaction for zero: it will not move any balance",
            )
            .warn(),
        );
    }

    if draft.currency.len() != 3 || !draft.currency.chars().all(|c| c.is_ascii_alphabetic()) {
        out.push(
            ImportProblem::row(
                ProblemCode::SuspiciousCurrency,
                number,
                format!("{:?} does not look like a currency code", draft.currency),
            )
            .with("currency", &draft.currency)
            .warn(),
        );
    }

    // Shares crossing the boundary with no money named: the lot's cost basis becomes zero and
    // the whole position reads as profit. The commonest cause is moving a portfolio between
    // brokers, where the receiving statement states quantities and never what they cost.
    if draft.kind.affects_quantity()
        && !draft.quantity.is_zero()
        && draft.price.is_zero()
        && draft.amount.is_zero()
    {
        out.push(
            ImportProblem::row(
                ProblemCode::DeliveryWithoutCost,
                number,
                format!(
                    "{} shares move with no value given: the lot enters at a cost of zero and the \
                     whole holding will read as profit. Enter the price paid, or the total, on this row",
                    draft.quantity
                ),
            )
            .with("quantity", draft.quantity)
            .with("symbol", draft.symbol.as_deref().unwrap_or("-"))
            .warn(),
        );
    }

    if let Some(account) = context.account_currency
        && !draft.currency.eq_ignore_ascii_case(account)
    {
        out.push(
            ImportProblem::row(
                ProblemCode::AccountCurrencyMismatch,
                number,
                format!(
                    "the row is in {} and the account it lands on keeps {account}: correct if the \
                     currency column was read wrong, ignore if the account really holds both",
                    draft.currency
                ),
            )
            .with("currency", &draft.currency)
            .with("account", account)
            .warn(),
        );
    }

    if let Some(today) = context.today
        && draft.date > today
    {
        out.push(
            ImportProblem::row(
                ProblemCode::FutureDate,
                number,
                format!(
                    "the date {} lies in the future — check the date format",
                    draft.date
                ),
            )
            .with("date", draft.date)
            .warn(),
        );
    }

    out
}

const MAX_SPAN_YEARS: i32 = 50;

const SINGLE_KIND_MIN_ROWS: usize = 20;

/// Smallest price step read as a split rather than as a market move, and how close to a whole
/// number the step has to be. A stock really can double between two trades, so this is a warning
/// and never a refusal — it says "check this", not "this is wrong".
const SPLIT_MIN_RATIO: f64 = 1.8;
const SPLIT_MAX_RATIO: f64 = 20.0;
/// A split's factor is *exact*; a market move that happens to land near a whole number is not.
/// Two trades a year apart in an instrument that doubled give 2.03, and calling that a split
/// once teaches the user to ignore the notice when it is real.
const SPLIT_ROUNDNESS: f64 = 0.01;
/// And the market has to have had no time to blur that factor. Over a quarter its contribution
/// is small enough that an exact whole number means something; over a year it is the whole
/// signal. The price series a provider sends is the reliable route to a split — it reports the
/// event itself — so this stays the narrow case that route cannot cover.
const SPLIT_MAX_DAYS: i64 = 90;

/// Prices of one instrument stepping by a whole factor between two adjacent trades: the broker
/// applied a split part-way through the statement. Quantities then refer to two different
/// shares, and the stored quotes are adjusted throughout, so the average cost comes out wrong
/// while the holding still adds up.
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
mod tests {
    use super::*;

    fn vote_of(rows: &[(TransactionKind, Decimal)]) -> SignVote {
        let mut vote = SignVote::default();
        for (kind, amount) in rows {
            count_vote(&mut vote, Some(*kind), *amount);
        }
        vote
    }

    #[test]
    fn a_share_movement_is_signed_on_the_quantity_not_on_the_amount() {
        // A split is printed as two legs of one wording: "-1 share out, +8 shares in". No
        // money moves, so the amount cannot carry the direction — the quantity does.
        assert_eq!(
            resolve_direction(TransactionKind::DeliveryInbound, dec!(-1), AmountSign::Signed),
            Direction::Flipped(TransactionKind::DeliveryOutbound)
        );
        assert_eq!(
            resolve_direction(TransactionKind::DeliveryInbound, dec!(8), AmountSign::Signed),
            Direction::Keep
        );
        // A sale is signed too, but its direction is already in the kind: Buy and Sell never
        // turn into each other.
        assert_eq!(
            resolve_direction(TransactionKind::Sell, dec!(-100), AmountSign::Signed),
            Direction::Conflict
        );
        // Nothing is read into a file that does not sign its amounts.
        assert_eq!(
            resolve_direction(TransactionKind::DeliveryInbound, dec!(-1), AmountSign::Unsigned),
            Direction::Keep
        );
    }

    #[test]
    fn conversions_do_not_vote_against_the_file_they_belong_to() {
        // A crypto file is half currency conversions, and one leg of every pair is negative
        // by construction. Counting those legs would drag the agreement under the threshold
        // and leave both legs credited — money out of nothing.
        let mut rows = vec![(TransactionKind::Deposit, dec!(100)); 5];
        rows.push((TransactionKind::Withdrawal, dec!(-40)));
        for _ in 0..9 {
            rows.push((TransactionKind::TransferIn, dec!(81.16)));
            rows.push((TransactionKind::TransferIn, dec!(-81.16)));
        }

        let vote = vote_of(&rows);
        assert_eq!(vote.votes, 6, "only the six rows whose direction is their own");
        let (sign, problem) = decide_amount_sign(vote);
        assert_eq!(sign, AmountSign::Signed);
        assert!(problem.is_none());
        // Not voting is not the same as not flipping: the negative leg still turns around.
        assert_eq!(
            resolve_direction(TransactionKind::TransferIn, dec!(-81.16), sign),
            Direction::Flipped(TransactionKind::TransferOut)
        );
    }

    #[test]
    fn one_refund_among_many_charges_makes_the_file_signed() {
        let mut rows = vec![(TransactionKind::Withdrawal, dec!(-10)); 199];
        rows.push((TransactionKind::Withdrawal, dec!(36.64)));
        let (sign, problem) = decide_amount_sign(vote_of(&rows));
        assert_eq!(sign, AmountSign::Signed);
        assert!(problem.is_none());
        assert_eq!(
            resolve_direction(TransactionKind::Withdrawal, dec!(36.64), sign),
            Direction::Flipped(TransactionKind::Deposit)
        );
    }

    #[test]
    fn all_positive_amounts_mean_the_sign_carries_nothing() {
        let rows = vec![
            (TransactionKind::Deposit, dec!(100)),
            (TransactionKind::Withdrawal, dec!(50)),
            (TransactionKind::Deposit, dec!(100)),
            (TransactionKind::Withdrawal, dec!(50)),
            (TransactionKind::Deposit, dec!(100)),
            (TransactionKind::Withdrawal, dec!(50)),
        ];
        let (sign, _) = decide_amount_sign(vote_of(&rows));
        assert_eq!(sign, AmountSign::Unsigned);
        assert_eq!(
            resolve_direction(TransactionKind::Withdrawal, dec!(50), sign),
            Direction::Keep
        );
    }

    #[test]
    fn half_disagreeing_is_reported_and_changes_nothing() {
        let mut rows = vec![(TransactionKind::Deposit, dec!(-100)); 5];
        rows.extend(vec![(TransactionKind::Deposit, dec!(100)); 5]);
        let (sign, problem) = decide_amount_sign(vote_of(&rows));
        assert_eq!(sign, AmountSign::Unsigned);
        assert_eq!(problem.unwrap().code, ProblemCode::AmountSignAmbiguous);
    }

    #[test]
    fn a_short_file_is_never_declared_signed() {
        let rows = vec![
            (TransactionKind::Withdrawal, dec!(-10)),
            (TransactionKind::Deposit, dec!(10)),
        ];
        assert_eq!(decide_amount_sign(vote_of(&rows)).0, AmountSign::Unsigned);
    }

    #[test]
    fn a_trade_is_reported_never_flipped() {
        assert_eq!(
            resolve_direction(TransactionKind::Buy, dec!(200), AmountSign::Signed),
            Direction::Conflict
        );
    }

    #[test]
    fn amount_is_checked_against_quantity_times_price() {
        let mut draft = draft_of(TransactionKind::Buy, dec!(199.99));
        draft.quantity = dec!(0.361938);
        draft.price = dec!(552.56);
        assert!(check_row(1, &draft, &CheckContext::default()).is_empty());

        draft.amount = dec!(19.99);
        let problems = check_row(1, &draft, &CheckContext::default());
        assert_eq!(problems[0].code, ProblemCode::AmountVsQuantityPrice);
        assert!(!problems[0].is_error());
    }

    #[test]
    fn fx_rate_on_the_base_currency_is_reported() {
        let mut draft = draft_of(TransactionKind::Dividend, dec!(6.05));
        draft.fx_rate_to_base = Some(dec!(0.8539));
        let context = CheckContext {
            base_currency: Some("EUR"),
            today: None,
            account_currency: None,
        };
        let codes: Vec<_> = check_row(1, &draft, &context).iter().map(|p| p.code).collect();
        assert!(codes.contains(&ProblemCode::FxRateOnBaseCurrency));
    }

    fn draft_of(kind: TransactionKind, amount: Decimal) -> TransactionDraft {
        TransactionDraft {
            account_id: "acc".into(),
            kind,
            date: "2024-01-02".parse().unwrap(),
            symbol: None,
            isin: None,
            security_name: None,
            security_id: None,
            quantity: Decimal::ZERO,
            price: Decimal::ZERO,
            amount,
            fees: Decimal::ZERO,
            taxes: Decimal::ZERO,
            currency: "EUR".into(),
            fee_currency: None,
            tax_currency: None,
            fx_rate_to_base: None,
            link_id: None,
            external_id: None,
            replaces: None,
            note: None,
        }
    }
}
