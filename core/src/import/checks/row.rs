//! Plausibility of one row on its own. Each check is independent and says at most one thing.

use super::CheckContext;
use crate::import::parse::{ImportProblem, ProblemCode};
use crate::import::preview::TransactionDraft;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Allowance per unit, because the printed unit price is rounded and that rounding scales with quantity.
const AMOUNT_TOLERANCE_PER_UNIT: Decimal = dec!(0.005);
const AMOUNT_TOLERANCE_RATIO: Decimal = dec!(0.002);
const AMOUNT_TOLERANCE_ABSOLUTE: Decimal = dec!(0.01);

type RowCheck = fn(usize, &TransactionDraft, &CheckContext<'_>) -> Option<ImportProblem>;

/// In the order the problems are listed.
const ROW_CHECKS: &[RowCheck] = &[
    amount_vs_quantity_price,
    fee_exceeds_amount,
    fx_rate_on_base_currency,
    zero_amount,
    suspicious_currency,
    delivery_without_cost,
    account_currency_mismatch,
    future_date,
];

/// Emits non-blocking plausibility diagnostics for one draft.
pub fn check_row(number: usize, draft: &TransactionDraft, context: &CheckContext<'_>) -> Vec<ImportProblem> {
    ROW_CHECKS
        .iter()
        .filter_map(|check| check(number, draft, context))
        .collect()
}

fn amount_tolerance(quantity: Decimal, expected: Decimal) -> Decimal {
    AMOUNT_TOLERANCE_ABSOLUTE
        + quantity.abs() * AMOUNT_TOLERANCE_PER_UNIT
        + expected.abs() * AMOUNT_TOLERANCE_RATIO
}

fn amount_vs_quantity_price(
    number: usize,
    draft: &TransactionDraft,
    _: &CheckContext<'_>,
) -> Option<ImportProblem> {
    if !draft.kind.affects_quantity()
        || draft.quantity.is_zero()
        || draft.price.is_zero()
        || draft.amount.is_zero()
    {
        return None;
    }
    // In range individually, out of range multiplied: the row is already refused for its size,
    // so this check has nothing left to say about it.
    let expected = draft.quantity.checked_mul(draft.price)?;
    if (draft.amount - expected).abs() <= amount_tolerance(draft.quantity, expected) {
        return None;
    }
    Some(
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
    )
}

fn fee_exceeds_amount(
    number: usize,
    draft: &TransactionDraft,
    _: &CheckContext<'_>,
) -> Option<ImportProblem> {
    let charges = draft.fees + draft.taxes;
    if draft.amount.is_zero() || charges <= draft.amount {
        return None;
    }
    Some(
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
    )
}

fn fx_rate_on_base_currency(
    number: usize,
    draft: &TransactionDraft,
    context: &CheckContext<'_>,
) -> Option<ImportProblem> {
    let base = context.base_currency?;
    if draft.fx_rate_to_base.is_none() || draft.currency != base {
        return None;
    }
    Some(
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
    )
}

fn zero_amount(number: usize, draft: &TransactionDraft, _: &CheckContext<'_>) -> Option<ImportProblem> {
    let moves_nothing = draft.amount.is_zero() && draft.fees.is_zero() && draft.taxes.is_zero();
    (draft.kind.cash_sign() != 0 && moves_nothing).then(|| {
        ImportProblem::row(
            ProblemCode::ZeroAmount,
            number,
            "a transaction for zero: it will not move any balance",
        )
        .warn()
    })
}

fn suspicious_currency(
    number: usize,
    draft: &TransactionDraft,
    _: &CheckContext<'_>,
) -> Option<ImportProblem> {
    let looks_like_code =
        draft.currency.len() == 3 && draft.currency.chars().all(|c| c.is_ascii_alphabetic());
    (!looks_like_code).then(|| {
        ImportProblem::row(
            ProblemCode::SuspiciousCurrency,
            number,
            format!("{:?} does not look like a currency code", draft.currency),
        )
        .with("currency", &draft.currency)
        .warn()
    })
}

/// Shares crossing the boundary at zero cost make the whole position read as profit.
fn delivery_without_cost(
    number: usize,
    draft: &TransactionDraft,
    _: &CheckContext<'_>,
) -> Option<ImportProblem> {
    let without_value = draft.price.is_zero() && draft.amount.is_zero();
    (draft.kind.affects_quantity() && !draft.quantity.is_zero() && without_value).then(|| {
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
        .warn()
    })
}

fn account_currency_mismatch(
    number: usize,
    draft: &TransactionDraft,
    context: &CheckContext<'_>,
) -> Option<ImportProblem> {
    let account = context.account_currency?;
    (!draft.currency.eq_ignore_ascii_case(account)).then(|| {
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
        .warn()
    })
}

fn future_date(number: usize, draft: &TransactionDraft, context: &CheckContext<'_>) -> Option<ImportProblem> {
    let today = context.today?;
    (draft.date > today).then(|| {
        ImportProblem::row(
            ProblemCode::FutureDate,
            number,
            format!(
                "the date {} lies in the future — check the date format",
                draft.date
            ),
        )
        .with("date", draft.date)
        .warn()
    })
}
