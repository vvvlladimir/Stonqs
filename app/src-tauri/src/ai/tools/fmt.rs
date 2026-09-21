//! Rounding, where the tool answers. Full ledger precision handed to a model comes back
//! quoted at 26 digits, and an answer is not truer for being long.

use serde_json::Value;

/// Rounded to the currency's own scale before it is handed over. Full `Decimal` precision is
/// what the *ledger* needs; a model given `40.71258117880362826842458338` will quote it back at
/// that length, and an answer is not more true for being 26 digits long. Rounding here rather
/// than asking the prompt nicely is what makes that impossible rather than unlikely.
pub(super) fn money(value: rust_decimal::Decimal) -> Value {
    Value::String(value.round_dp(2).normalize().to_string())
}

/// A price carries more scale than a balance: sub-cent quotes are real, 26 digits are not.
pub(super) fn price(value: rust_decimal::Decimal) -> Value {
    Value::String(value.round_dp(6).normalize().to_string())
}

/// Already multiplied by 100 — two decimals is what a percentage is ever read to.
pub(super) fn percent(value: rust_decimal::Decimal) -> Value {
    Value::String(
        (value * rust_decimal::Decimal::ONE_HUNDRED)
            .round_dp(2)
            .normalize()
            .to_string(),
    )
}

/// The same for the statistics that are genuinely `f64` (volatility, Sharpe) — see
/// `.claude/rules/money-and-fx.md` for why those are not `Decimal`.
pub(super) fn ratio(value: f64, places: i32) -> Value {
    let factor = 10f64.powi(places);
    Value::from((value * factor).round() / factor)
}

pub(super) fn quantity(value: rust_decimal::Decimal) -> Value {
    Value::String(value.normalize().to_string())
}

// --- bodies -----------------------------------------------------------------------------
