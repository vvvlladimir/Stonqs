use super::*;
use crate::model::TransactionKind;
use chrono::NaiveDate;
use rust_decimal_macros::dec;

/// Missing lookup: the transaction must still supply a usable rate.
struct NoRates;
impl RateLookup for NoRates {
    fn rate_as_of(&self, _: &str, _: &str, _: NaiveDate) -> Result<Option<Decimal>> {
        Ok(None)
    }
}

/// A base-currency dividend must ignore a misleading broker FX column.
#[test]
fn a_transaction_in_the_base_currency_is_never_converted() {
    let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
    let mut t = Transaction::cash("acc", TransactionKind::Dividend, date, dec!(0.22), "EUR");
    t.fx_rate_to_base = Some(dec!(0.853898));

    assert_eq!(resolve_rate(&t, "EUR", &NoRates).unwrap(), Decimal::ONE);
}

/// For foreign currency, the transaction's recorded rate overrides the lookup.
#[test]
fn the_rate_recorded_in_the_trade_still_wins_for_other_currencies() {
    let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
    let mut t = Transaction::cash("acc", TransactionKind::Dividend, date, dec!(0.26), "USD");
    t.fx_rate_to_base = Some(dec!(0.853898));

    assert_eq!(resolve_rate(&t, "EUR", &NoRates).unwrap(), dec!(0.853898));
}
