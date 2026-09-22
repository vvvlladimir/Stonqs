use super::mapping::AmountSign;
use super::parse::{ImportProblem, ProblemCode};
use super::preview::{ImportRow, KindMapping, TransactionDraft};
use crate::model::TransactionKind;
use chrono::{Datelike, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Optional context for plausibility checks.
#[derive(Debug, Clone, Copy, Default)]
pub struct CheckContext<'a> {
    pub base_currency: Option<&'a str>,

    pub today: Option<NaiveDate>,
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

const AMOUNT_TOLERANCE_RATIO: Decimal = dec!(0.01);
const AMOUNT_TOLERANCE_ABSOLUTE: Decimal = dec!(0.01);

/// Emits non-blocking plausibility diagnostics for one draft.
pub fn check_row(number: usize, draft: &TransactionDraft, context: &CheckContext<'_>) -> Vec<ImportProblem> {
    let mut out = Vec::new();

    if draft.kind.affects_quantity()
        && !draft.quantity.is_zero()
        && !draft.price.is_zero()
        && !draft.amount.is_zero()
    {
        let expected = draft.quantity * draft.price;
        let tolerance = expected * AMOUNT_TOLERANCE_RATIO + AMOUNT_TOLERANCE_ABSOLUTE;
        if (draft.amount - expected).abs() > tolerance {
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

pub fn check_file(rows: &[ImportRow], kinds: &[KindMapping]) -> Vec<ImportProblem> {
    let mut out = Vec::new();

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
            note: None,
        }
    }
}
