//! Money, currencies and small supporting arithmetic.

use rust_decimal::Decimal;
use rust_decimal::prelude::*;

/// ISO 4217 currency code (`"EUR"`, `"USD"`).
///
/// Plain `String`, not a newtype — the DB stores text anyway, so a wrapper
/// would only add ceremony. Normalization lives in [`normalize_currency`].
pub type Currency = String;

/// Canonicalizes a currency code (uppercase, trimmed) so sources spelling it
/// differently (`usd`, `USD `, `Usd`) don't create duplicate currencies.
pub fn normalize_currency(code: &str) -> Currency {
    code.trim().to_ascii_uppercase()
}

/// A minor unit as a source spells it, and its ratio to the major currency.
struct MinorUnit {
    code: &'static str,
    major: &'static str,
    /// 2 = hundredth (pence), 3 = thousandth (fils).
    exponent: u32,
    /// `GBp` vs `GBP` are different money; `usd` vs `USD` are the same — so
    /// case sensitivity is per unit, not global.
    exact_case: bool,
}

/// Minor units seen in quotes: pence (London), cents (Johannesburg), agorot
/// (Tel Aviv), fils (Kuwait). List is closed on purpose — guessing from the
/// code's shape risks a 100x error.
const MINOR_UNITS: &[MinorUnit] = &[
    // London: Yahoo emits `GBp`, Stooq and broker exports emit `GBX`.
    MinorUnit {
        code: "GBp",
        major: "GBP",
        exponent: 2,
        exact_case: true,
    },
    MinorUnit {
        code: "GBX",
        major: "GBP",
        exponent: 2,
        exact_case: false,
    },
    // Johannesburg; `ZAC` isn't taken by any currency, so case doesn't matter.
    MinorUnit {
        code: "ZAc",
        major: "ZAR",
        exponent: 2,
        exact_case: false,
    },
    // Tel Aviv, agorot.
    MinorUnit {
        code: "ILA",
        major: "ILS",
        exponent: 2,
        exact_case: false,
    },
    // Kuwait, fils — a thousandth of the dinar, not a hundredth.
    MinorUnit {
        code: "KWF",
        major: "KWD",
        exponent: 3,
        exact_case: false,
    },
];

/// Source currency folded to its major unit, with the multiplier to reach it
/// (e.g. `("GBP", 0.01)` for pence). See `money-and-fx.md` for why this
/// isn't part of [`normalize_currency`] and why GBX isn't kept as its own
/// currency with a synthetic FX rate.
pub fn major_currency(code: &str) -> (Currency, Decimal) {
    let raw = code.trim();
    for unit in MINOR_UNITS {
        let hit = if unit.exact_case {
            raw == unit.code
        } else {
            raw.eq_ignore_ascii_case(unit.code)
        };
        if hit {
            return (unit.major.to_string(), Decimal::new(1, unit.exponent));
        }
    }
    (normalize_currency(raw), Decimal::ONE)
}

/// An amount with its currency attached, used at the UI boundary so a
/// report never reads as a bare `12345`. Calc code uses plain [`Decimal`]
/// internally, where the currency is already fixed by context.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Money {
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    pub currency: Currency,
}

impl Money {
    pub fn new(amount: Decimal, currency: impl AsRef<str>) -> Self {
        Money {
            amount,
            currency: normalize_currency(currency.as_ref()),
        }
    }
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2} {}", self.amount, self.currency)
    }
}

/// Rounds to currency cents (2dp, half-up). Only for display/storage of a
/// final total — never mid-calculation, where `Decimal`'s 28 digits should
/// be left alone to avoid accumulating rounding error.
pub fn round_money(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

/// Rounds a share quantity (8dp — enough for fractional shares and crypto).
pub fn round_quantity(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(8, RoundingStrategy::MidpointAwayFromZero)
}

/// Division by zero is routine in return calculations (empty portfolio, zero
/// base) — `None` lets callers handle it explicitly instead of panicking.
pub fn checked_div(a: Decimal, b: Decimal) -> Option<Decimal> {
    if b.is_zero() { None } else { Some(a / b) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn an_ordinary_currency_passes_through_unchanged() {
        assert_eq!(major_currency(" usd "), ("USD".to_string(), Decimal::ONE));
    }

    #[test]
    fn pence_become_pounds_divided_by_a_hundred() {
        // SWDA.L quotes at 10953.0 GBp -> 10953.0 x 0.01 = 109.53 GBP.
        let (currency, factor) = major_currency("GBp");
        assert_eq!(currency, "GBP");
        assert_eq!(dec!(10953.0) * factor, dec!(109.5300));
    }

    #[test]
    fn the_uppercase_spelling_of_pence_is_a_separate_code() {
        // `GBX` is a distinct code, not a spelling of `GBP` — case-insensitive.
        assert_eq!(major_currency("GBX"), ("GBP".to_string(), dec!(0.01)));
        assert_eq!(major_currency("gbx"), ("GBP".to_string(), dec!(0.01)));
    }

    #[test]
    fn pounds_are_not_mistaken_for_pence() {
        // The guard hinges on the last letter's case; check the spelling that never occurs as a minor unit too.
        assert_eq!(major_currency("GBP"), ("GBP".to_string(), Decimal::ONE));
        assert_eq!(major_currency("gbp"), ("GBP".to_string(), Decimal::ONE));
    }

    #[test]
    fn fils_is_a_thousandth_not_a_hundredth() {
        assert_eq!(major_currency("KWF"), ("KWD".to_string(), dec!(0.001)));
    }
}
