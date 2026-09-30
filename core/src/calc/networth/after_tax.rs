//! Net worth after the tax selling would cost, on one rate the owner states (ADR-0093).
//!
//! A second view, never the total: it is exact about the portfolio's unrealized gain, silent
//! about everything owned outside it — an asset has no purchase price by design (ADR-0092) — and
//! it carries the value it could not speak about so the figure cannot be read as "after all tax".

use crate::error::{Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// What would be owed if the portfolio were sold today, and what net worth is left after it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AfterTax {
    /// The rate as a fraction, echoed back so a tile can say what it assumed.
    #[serde(with = "rust_decimal::serde::str")]
    pub rate: Decimal,
    /// What the rate was applied to: the portfolio's unrealized gain, never below zero.
    #[serde(with = "rust_decimal::serde::str")]
    pub taxable_gain_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub tax_base: Decimal,
    /// Net worth minus that tax.
    #[serde(with = "rust_decimal::serde::str")]
    pub net_after_tax_base: Decimal,
    /// What this reading says nothing about: everything owned outside the portfolio, which has no
    /// purchase price and therefore no gain to tax.
    #[serde(with = "rust_decimal::serde::str")]
    pub outside_base: Decimal,
}

/// `unrealized_gain_base` is the portfolio's own unrealized gain; `outside_base` is what is owned
/// beside it. `rate` is a fraction: `0.26` is 26%.
pub fn after_tax(
    net_base: Decimal,
    unrealized_gain_base: Decimal,
    outside_base: Decimal,
    rate: Decimal,
) -> Result<AfterTax> {
    if rate < Decimal::ZERO || rate > Decimal::ONE {
        return Err(Error::Invalid(format!(
            "a tax rate of {rate} is not between 0 and 100%"
        )));
    }
    // A loss is not a refund, and carrying it forward is a tax rule — which is the one thing
    // this reading refuses to model.
    let taxable_gain_base = unrealized_gain_base.max(Decimal::ZERO);
    let tax_base = taxable_gain_base * rate;

    Ok(AfterTax {
        rate,
        taxable_gain_base,
        tax_base,
        net_after_tax_base: net_base - tax_base,
        outside_base,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// 180 000 of net worth, 12 000 of it an unrealized gain, taxed at 26%:
    ///   12 000 x 0.26 = 3 120, so 176 880 is left.
    #[test]
    fn the_tax_is_the_rate_over_the_unrealized_gain() {
        let view = after_tax(dec!(180000), dec!(12000), dec!(400000), dec!(0.26)).unwrap();

        assert_eq!(view.taxable_gain_base, dec!(12000));
        assert_eq!(view.tax_base, dec!(3120));
        assert_eq!(view.net_after_tax_base, dec!(176880));
        // And it says out loud what it could not speak about.
        assert_eq!(view.outside_base, dec!(400000));
    }

    /// A portfolio under water owes nothing, and the reading is still an answer.
    #[test]
    fn a_loss_is_taxed_at_nothing_rather_than_refunded() {
        let view = after_tax(dec!(50000), dec!(-8000), Decimal::ZERO, dec!(0.26)).unwrap();

        assert_eq!(view.taxable_gain_base, Decimal::ZERO);
        assert_eq!(view.tax_base, Decimal::ZERO);
        assert_eq!(view.net_after_tax_base, dec!(50000));
    }

    /// Zero is a rate somebody may genuinely have; it is not the same as asking nothing.
    #[test]
    fn a_zero_rate_answers_zero_tax() {
        let view = after_tax(dec!(180000), dec!(12000), Decimal::ZERO, Decimal::ZERO).unwrap();

        assert_eq!(view.tax_base, Decimal::ZERO);
        assert_eq!(view.net_after_tax_base, dec!(180000));
    }

    /// Refused, not clamped: a rate of 260% is a typo, and quietly reading it as 100% would hide
    /// the typo behind a plausible figure.
    #[test]
    fn a_rate_outside_the_range_is_refused() {
        assert!(after_tax(dec!(180000), dec!(12000), Decimal::ZERO, dec!(2.6)).is_err());
        assert!(after_tax(dec!(180000), dec!(12000), Decimal::ZERO, dec!(-0.1)).is_err());
    }

    /// Debt can put net worth under water; the tax on the gain is still owed on top of it.
    #[test]
    fn a_negative_net_worth_still_carries_the_tax() {
        let view = after_tax(dec!(-20000), dec!(5000), Decimal::ZERO, dec!(0.2)).unwrap();

        assert_eq!(view.tax_base, dec!(1000));
        assert_eq!(view.net_after_tax_base, dec!(-21000));
    }
}
