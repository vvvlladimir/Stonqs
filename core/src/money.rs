//! Money, currencies and small supporting arithmetic.

use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;

/// ISO 4217 code; normalised by [`normalize_currency`].
pub type Currency = String;

/// Canonicalizes a currency code (uppercase, trimmed) so sources spelling it
/// differently (`usd`, `USD `, `Usd`) don't create duplicate currencies.
pub fn normalize_currency(code: &str) -> Currency {
    code.trim().to_ascii_uppercase()
}

/// Whether a code can be a currency at all: two to six letters or digits, at least one of them a
/// letter. Not a list — a crypto ticker is nobody's registry — but a shape, so a broker's
/// "1 000,50" or "DOLLARS AND CENTS" is not written as money nothing will ever find a rate for.
pub fn is_currency_code(code: &str) -> bool {
    let code = code.trim();
    (2..=6).contains(&code.chars().count())
        && code.chars().all(|c| c.is_ascii_alphanumeric())
        && code.chars().any(|c| c.is_ascii_alphabetic())
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

/// Closed list on purpose: guessing from the code's shape risks a 100x error.
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

/// Minor unit folded to its major one with the multiplier (`("GBP", 0.01)` for pence).
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

/// An amount with its currency, for the UI boundary.
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

/// 2dp half-up, for final totals only, never mid-calculation.
pub fn round_money(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

/// Rounds a share quantity (8dp — enough for fractional shares and crypto).
pub fn round_quantity(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(8, RoundingStrategy::MidpointAwayFromZero)
}

/// Largest magnitude a ledger figure (amount, quantity, price, charge) may carry. `Decimal`
/// itself holds 7.9e28, but two such numbers multiplied overflow and `Decimal`'s operators
/// panic rather than saturate, so the headroom above this is left to intermediate arithmetic.
pub const MAX_MAGNITUDE: Decimal = dec!(1_000_000_000_000_000);

/// Whether a figure is small enough to be stored and multiplied without overflowing.
pub fn in_range(v: Decimal) -> bool {
    v.abs() <= MAX_MAGNITUDE
}

/// Decimals a split leaves a quantity with. `Decimal` carries 28 significant digits, and both
/// problems a split brings come from spending all of them: a sum of full-precision parts rounds,
/// so the lots and the accounts of a position stop adding up to it, and a factor applied and then
/// undone (7/3 then 3/7) never lands back on the number it started from. A scale of 18 is finer
/// than any instrument is traded in — it is the smallest unit of ether — and rounds back onto
/// itself.
const QUANTITY_SCALE: u32 = 18;

/// Rounds a split-adjusted quantity to [`QUANTITY_SCALE`], or to whatever a large quantity leaves
/// room for: the digits before the point and the digits after it share the same 28.
pub fn fit_quantity(v: Decimal) -> Decimal {
    // `trunc` keeps the scale it was given, so the zeros it left behind are not digits.
    let integer_part = v.trunc().normalize().mantissa().unsigned_abs();
    let integer_digits = integer_part.checked_ilog10().map_or(0, |log| log + 1);
    v.round_dp(QUANTITY_SCALE.min(28_u32.saturating_sub(integer_digits)))
}

/// Division by zero is routine in return calculations (empty portfolio, zero
/// base) — `None` lets callers handle it explicitly instead of panicking.
pub fn checked_div(a: Decimal, b: Decimal) -> Option<Decimal> {
    if b.is_zero() { None } else { Some(a / b) }
}

#[cfg(test)]
mod tests {
    use super::*;

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
