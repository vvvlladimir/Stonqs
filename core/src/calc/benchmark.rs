use super::GrowthSeries;
use crate::error::{Error, Result};
use crate::fx::RateLookup;
use crate::market::PriceLookup;
use crate::money::normalize_currency;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Portfolio vs. a benchmark over the same period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    pub from: NaiveDate,
    pub to: NaiveDate,
    /// Portfolio TWR as a fraction — `0.1` = +10%.
    #[serde(with = "rust_decimal::serde::str")]
    pub portfolio_twr: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub benchmark_twr: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub excess: Decimal,
}

/// Benchmark price return in base currency; no dividends are invented because providers
/// expose prices, and benchmarks have no external flows to split.
pub fn benchmark_return(
    security_id: &str,
    base: &str,
    from: NaiveDate,
    to: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
) -> Result<Decimal> {
    let base = normalize_currency(base);
    let start = price_in_base(security_id, &base, from, prices, rates)?;
    if start.is_zero() {
        return Err(Error::Math(format!(
            "benchmark {security_id} is worth zero on {from}"
        )));
    }
    Ok(price_in_base(security_id, &base, to, prices, rates)? / start - Decimal::ONE)
}

/// Benchmark price on a date, converted to base. Currency comes from the quote itself, not
/// the security record — a listing switch can leave those disagreeing.
fn price_in_base(
    security_id: &str,
    base: &str,
    date: NaiveDate,
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
) -> Result<Decimal> {
    let price = prices
        .price_as_of(security_id, date)?
        .ok_or_else(|| Error::MissingMarketData {
            kind: "price",
            key: security_id.to_string(),
            date,
        })?;
    let rate = rates
        .rate_as_of(&price.currency, base, date)?
        .ok_or_else(|| Error::MissingMarketData {
            kind: "fx rate",
            key: format!("{}/{base}", price.currency),
            date,
        })?;
    Ok(price.close * rate)
}

/// The first of `dates` the benchmark has a price for, `None` if it has none at all.
///
/// `price_as_of` is forward-fill, so "has a price" only ever turns on as the date moves
/// forward: the day can be bisected instead of scanned.
pub fn benchmark_start(
    security_id: &str,
    dates: &[NaiveDate],
    prices: &dyn PriceLookup,
) -> Result<Option<NaiveDate>> {
    let Some(&last) = dates.last() else {
        return Ok(None);
    };
    if prices.price_as_of(security_id, last)?.is_none() {
        return Ok(None);
    }
    let (mut low, mut high) = (0, dates.len() - 1);
    while low < high {
        let mid = low + (high - low) / 2;
        if prices.price_as_of(security_id, dates[mid])?.is_some() {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    Ok(Some(dates[low]))
}

/// Growth of one benchmark unit on the portfolio's date grid; forward-filled prices cover
/// days when the benchmark does not trade.
///
/// A benchmark younger than the period is not missing data — it did not exist yet. The series
/// then starts on the first day it has a price for and is based at that price, so the caller
/// draws a line that begins later rather than no line at all. It is aligned by date, not by
/// index: the two series need not be the same length. A benchmark with no price anywhere in
/// the window is still `MissingMarketData` — that one really is a gap.
pub fn benchmark_series(
    security_id: &str,
    base: &str,
    dates: &[NaiveDate],
    prices: &dyn PriceLookup,
    rates: &dyn RateLookup,
) -> Result<GrowthSeries> {
    let base = normalize_currency(base);
    let mut out = GrowthSeries::default();
    let Some(&last) = dates.last() else {
        return Ok(out);
    };
    let first = benchmark_start(security_id, dates, prices)?.ok_or(Error::MissingMarketData {
        kind: "price",
        key: security_id.to_string(),
        date: last,
    })?;
    let start = price_in_base(security_id, &base, first, prices, rates)?;
    if start.is_zero() {
        return Err(Error::Math(format!(
            "benchmark {security_id} is worth zero on {first}"
        )));
    }
    for date in dates.iter().skip_while(|date| **date < first) {
        out.push(
            *date,
            price_in_base(security_id, &base, *date, prices, rates)? / start,
        );
    }
    Ok(out)
}

/// Combines portfolio and benchmark returns; excess is their arithmetic difference.
pub fn compare_to_benchmark(
    from: NaiveDate,
    to: NaiveDate,
    portfolio_twr: Decimal,
    benchmark_twr: Decimal,
) -> BenchmarkComparison {
    BenchmarkComparison {
        from,
        to,
        portfolio_twr,
        benchmark_twr,
        excess: portfolio_twr - benchmark_twr,
    }
}
