use super::{CashFlow, GrowthSeries, annualize, xirr};
use crate::error::{Error, Result};
use crate::inflation::IndexLookup;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A nominal return restated in the money of the period's first day.
///
/// The window is reported rather than assumed: a month's index is published weeks after the
/// month ends, so a period running to today is deflated only through the last published month.
/// `factor` is what one unit of money at `from` costs at `to` — `1.023` is 2.3% of inflation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealReturn {
    pub region: String,
    pub from: NaiveDate,
    /// The last day the index covers, never later than the period's end.
    pub to: NaiveDate,
    #[serde(with = "rust_decimal::serde::str")]
    pub factor: Decimal,
    /// The same thing as a rate — `factor - 1`. Carried rather than left to the reader: the
    /// frontend does no `Decimal` arithmetic, and a float subtraction there would round.
    #[serde(with = "rust_decimal::serde::str")]
    pub inflation: Decimal,
    /// The return as given, before the money it is measured in lost value.
    #[serde(with = "rust_decimal::serde::str")]
    pub nominal: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub real: Decimal,
    /// `real` as a yearly rate, over the window the *return* covers rather than the shorter
    /// one `to` reports; `None` for a window shorter than a day.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub real_annualized: Option<Decimal>,
}

/// What one unit of money at `from` costs at `to`: `I(to) / I(from)`.
///
/// Both levels come from one region's stored series, which is written by a single source, so
/// the ratio measures prices rather than a change of index base. `MissingMarketData` when the
/// region has nothing published on or before `from` — that is a gap, unlike the tail of the
/// window, which is merely not published yet.
pub fn inflation_factor(
    region: &str,
    from: NaiveDate,
    to: NaiveDate,
    index: &dyn IndexLookup,
) -> Result<Decimal> {
    let level = |date: NaiveDate| -> Result<Decimal> {
        index
            .index_as_of(region, date)?
            .ok_or_else(|| Error::MissingMarketData {
                kind: "price index",
                key: region.to_string(),
                date,
            })
    };
    let start = level(from)?;
    if start <= Decimal::ZERO {
        return Err(Error::Math(format!("price index for {region} is zero on {from}")));
    }
    Ok(level(to)? / start)
}

/// The last day of a period that the region's index actually covers.
///
/// An index level published for a month holds to the end of that month, so a series running
/// through June covers the 30th. `None` when the region has no series at all.
pub fn deflation_end(region: &str, to: NaiveDate, index: &dyn IndexLookup) -> Result<Option<NaiveDate>> {
    let Some(month) = index.index_through(region)? else {
        return Ok(None);
    };
    let last_day = crate::inflation::first_of_month(month)
        .checked_add_months(chrono::Months::new(1))
        .and_then(|next| next.pred_opt())
        .unwrap_or(month);
    Ok(Some(to.min(last_day)))
}

/// Takes inflation out of a return: `(1 + nominal) / factor - 1`.
///
/// Not `nominal - inflation`. Money earned is spent at the later price level, so the two
/// compound rather than add; at the rates a quiet year produces the difference is small, and
/// over a decade or in a high-inflation economy it is not.
pub fn real_return(nominal: Decimal, factor: Decimal) -> Result<Decimal> {
    if factor <= Decimal::ZERO {
        return Err(Error::Math("inflation factor is not positive".into()));
    }
    Ok((Decimal::ONE + nominal) / factor - Decimal::ONE)
}

/// Restates a period's nominal return in the money of its first day.
///
/// The period is narrowed to what the index covers, and the nominal return is *not* recomputed
/// for that shorter window: the caller passes the return of the window it reports, and the
/// figure says which window the deflation was measured over. That understates real return by
/// whatever the unpublished tail earned, which is the honest direction to be wrong in.
pub fn real_period_return(
    region: &str,
    from: NaiveDate,
    to: NaiveDate,
    nominal: Decimal,
    index: &dyn IndexLookup,
) -> Result<RealReturn> {
    let deflated_to = deflation_end(region, to, index)?.ok_or_else(|| Error::MissingMarketData {
        kind: "price index",
        key: region.to_string(),
        date: to,
    })?;
    let factor = inflation_factor(region, from, deflated_to, index)?;
    let real = real_return(nominal, factor)?;
    Ok(RealReturn {
        region: crate::model::normalize_region(region),
        from,
        to: deflated_to,
        factor,
        inflation: factor - Decimal::ONE,
        nominal,
        real,
        // Over the window the *return* covers, not the shorter one the index does. `real` is
        // the caller's nominal return with what inflation is published taken out of it, so
        // dividing it by the published months alone would scale a full period's earnings by a
        // part of it — overstating the yearly rate, in the opposite direction to the understated
        // `real` this deliberately accepts.
        real_annualized: annualize(real, from, to),
    })
}

/// The internal rate of return earned on flows restated in the money of `base_date`.
///
/// Each flow is deflated at *its own* date rather than the result being deflated once: a
/// payment made three years in was made in cheaper money, and discounting the answer instead
/// would credit every flow with the same purchasing power.
pub fn real_xirr(
    region: &str,
    base_date: NaiveDate,
    flows: &[CashFlow],
    index: &dyn IndexLookup,
) -> Result<Decimal> {
    let mut deflated = Vec::with_capacity(flows.len());
    for flow in flows {
        let factor = inflation_factor(region, base_date, flow.date, index)?;
        deflated.push(CashFlow {
            date: flow.date,
            amount_base: flow.amount_base / factor,
        });
    }
    xirr(&deflated)
}

/// Growth of one unit of money's *cost* on the portfolio's date grid, based at the first day —
/// the same shape a benchmark has, so a chart draws it as one more line.
///
/// It steps: a monthly level holds until the next is published, and interpolating it would
/// draw a daily inflation rate nobody measured. Days past the last published month are left
/// out rather than flattened, so the line stops where the data does.
pub fn inflation_series(region: &str, dates: &[NaiveDate], index: &dyn IndexLookup) -> Result<GrowthSeries> {
    let mut out = GrowthSeries::default();
    let (Some(&first), Some(&last)) = (dates.first(), dates.last()) else {
        return Ok(out);
    };
    let through = deflation_end(region, last, index)?.ok_or_else(|| Error::MissingMarketData {
        kind: "price index",
        key: region.to_string(),
        date: last,
    })?;
    for date in dates.iter().take_while(|date| **date <= through) {
        out.push(*date, inflation_factor(region, first, *date, index)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inflation::IndexPoint;
    use crate::market::DateRange;
    use crate::storage::Store;
    use rust_decimal_macros::dec;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    /// Twelve months at 2015 = 100, ending June 2024 — July is not published yet.
    fn index() -> Store {
        let store = Store::open_in_memory().unwrap();
        let levels = [
            ("2023-07-01", dec!(120.0)),
            ("2023-10-01", dec!(121.2)),
            ("2024-01-01", dec!(122.4)),
            ("2024-04-01", dec!(123.6)),
            ("2024-06-01", dec!(124.8)),
        ];
        let points: Vec<IndexPoint> = levels
            .iter()
            .map(|(month, value)| IndexPoint::new("DE", d(month), *value))
            .collect();
        store
            .save_index_from(
                "DE",
                &points,
                "eurostat",
                DateRange::new(d("2023-07-01"), d("2024-07-31")),
            )
            .unwrap();
        store
    }

    #[test]
    fn a_factor_is_the_ratio_of_two_published_levels() {
        // July 2023 = 120.0, June 2024 = 124.8.
        // 124.8 / 120.0 = 1.04 — prices are 4% higher.
        let store = index();
        let factor = inflation_factor("DE", d("2023-07-15"), d("2024-06-30"), &store).unwrap();
        assert_eq!(factor, dec!(1.04));
    }

    #[test]
    fn a_level_holds_for_its_whole_month() {
        // April's 123.6 is the last level published on or before 2024-05-31, so both days
        // give the same factor against July 2023: 123.6 / 120.0 = 1.03.
        let store = index();
        assert_eq!(
            inflation_factor("DE", d("2023-07-01"), d("2024-04-02"), &store).unwrap(),
            dec!(1.03)
        );
        assert_eq!(
            inflation_factor("DE", d("2023-07-01"), d("2024-05-31"), &store).unwrap(),
            dec!(1.03)
        );
    }

    #[test]
    fn real_return_divides_rather_than_subtracts() {
        // Nominal +9%, prices +4%: (1 + 0.09) / 1.04 - 1 = 1.09 / 1.04 - 1
        //   = 1.0480769230769230769230769231 - 1 = 0.0480769230769230769230769231,
        // which is 4.8077%, not the 5.0000% subtraction would report.
        let real = real_return(dec!(0.09), dec!(1.04)).unwrap();
        assert_eq!(real.round_dp(6), dec!(0.048077));
        assert_ne!(real.round_dp(6), dec!(0.050000));
    }

    #[test]
    fn a_period_is_narrowed_to_what_the_index_covers() {
        // The period runs to 2024-07-20, the series ends with June, whose level holds to
        // 2024-06-30. Nominal +9% over the asked period, deflated by 124.8 / 120.0 = 1.04.
        let store = index();
        let out = real_period_return("DE", d("2023-07-01"), d("2024-07-20"), dec!(0.09), &store).unwrap();
        assert_eq!(out.to, d("2024-06-30"));
        assert_eq!(out.factor, dec!(1.04));
        assert_eq!(out.real.round_dp(6), dec!(0.048077));
        // The yearly rate spans the window the return covers — 2023-07-01 to 2024-07-20, 385
        // days — not the shorter one the index covers: 1.048077^(365/385) - 1 = 0.045523.
        // Over the index's own 365 days it would read 4.8077%, crediting a full period's
        // earnings to eleven and a half months of it.
        assert_eq!(out.real_annualized.unwrap().round_dp(6), dec!(0.045523));
    }

    #[test]
    fn each_flow_is_deflated_at_its_own_date() {
        // Paid 1000 on 2023-07-01 (index 120.0), worth 1040 on 2024-06-30 (index 124.8).
        // In July 2023 money the proceeds are 1040 / 1.04 = 1000, so the real IRR is 0%
        // even though the nominal one is +4% over the year.
        let store = index();
        let flows = vec![
            CashFlow {
                date: d("2023-07-01"),
                amount_base: dec!(-1000),
            },
            CashFlow {
                date: d("2024-06-30"),
                amount_base: dec!(1040),
            },
        ];
        assert!(xirr(&flows).unwrap() > dec!(0.039));
        assert_eq!(
            real_xirr("DE", d("2023-07-01"), &flows, &store)
                .unwrap()
                .round_dp(6),
            dec!(0)
        );
    }

    #[test]
    fn the_series_steps_and_stops_where_the_data_does() {
        let store = index();
        let dates: Vec<NaiveDate> = [
            "2023-07-01",
            "2023-07-31",
            "2023-10-01",
            "2024-06-30",
            "2024-07-01",
        ]
        .iter()
        .map(|s| d(s))
        .collect();
        let series = inflation_series("DE", &dates, &store).unwrap();
        // July's level holds all month, so the first two days share a value of 1.
        assert_eq!(series.values, vec![dec!(1), dec!(1), dec!(1.01), dec!(1.04)]);
        assert_eq!(series.dates.last(), Some(&d("2024-06-30")));
    }

    #[test]
    fn a_region_with_no_series_is_missing_data_rather_than_zero_inflation() {
        let store = index();
        assert!(matches!(
            real_period_return("FR", d("2023-07-01"), d("2024-06-30"), dec!(0.09), &store),
            Err(Error::MissingMarketData { .. })
        ));
    }
}
