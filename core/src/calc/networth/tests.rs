use super::*;
use crate::model::AssetKind;
use rust_decimal_macros::dec;

fn d(y: i32, m: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, day).unwrap()
}

/// One rate, so a conversion either finds it or is a bug in the caller.
struct Rates;

impl RateLookup for Rates {
    fn rate_as_of(&self, from: &str, to: &str, _date: NaiveDate) -> Result<Option<Decimal>> {
        Ok(match (from, to) {
            ("EUR", "EUR") => Some(Decimal::ONE),
            ("CHF", "EUR") => Some(dec!(1.05)),
            _ => None,
        })
    }
}

fn house() -> Asset {
    let mut asset = Asset::new("Flat", AssetKind::Property, "EUR");
    asset.id = "house".into();
    asset
}

fn mortgage() -> Asset {
    let mut asset = Asset::new("Mortgage", AssetKind::Mortgage, "EUR");
    asset.id = "debt".into();
    asset.secured_by = Some("house".into());
    asset
}

#[test]
fn a_debt_is_subtracted_and_a_thing_owned_is_added() {
    let assets = vec![house(), mortgage()];
    let values = vec![
        AssetValue::new("house", d(2026, 1, 1), dec!(400000)),
        AssetValue::new("debt", d(2026, 1, 1), dec!(250000)),
    ];

    let reading = net_worth(dec!(60000), &assets, &values, "EUR", d(2026, 6, 30), &Rates).unwrap();

    // 60 000 invested + 400 000 owned - 250 000 owed = 210 000.
    assert_eq!(reading.owned_base, dec!(400000));
    assert_eq!(reading.owed_base, dec!(250000));
    assert_eq!(reading.net_base, dec!(210000));
    // The house is the biggest thing owned, so it leads; the debt is last.
    assert_eq!(reading.holdings[0].asset_id, "house");
    assert_eq!(reading.holdings[1].side, AssetSide::Owed);
}

/// The figure in force is the last one written, not the one nearest the reading date.
#[test]
fn a_valuation_holds_until_the_next_one() {
    let values = vec![
        AssetValue::new("house", d(2026, 1, 1), dec!(400000)),
        AssetValue::new("house", d(2026, 7, 1), dec!(430000)),
    ];
    let assets = vec![house()];

    let june = net_worth(Decimal::ZERO, &assets, &values, "EUR", d(2026, 6, 30), &Rates).unwrap();
    let july = net_worth(Decimal::ZERO, &assets, &values, "EUR", d(2026, 7, 1), &Rates).unwrap();

    assert_eq!(june.owned_base, dec!(400000));
    assert_eq!(june.holdings[0].valued_on, d(2026, 1, 1));
    assert_eq!(july.owned_base, dec!(430000));
}

#[test]
fn a_thing_is_absent_before_its_first_valuation_and_from_its_closing_day() {
    let mut sold = house();
    sold.closed_on = Some(d(2026, 3, 10));
    let assets = vec![sold];
    let values = vec![AssetValue::new("house", d(2026, 2, 1), dec!(400000))];

    let before = net_worth(Decimal::ZERO, &assets, &values, "EUR", d(2026, 1, 31), &Rates).unwrap();
    let during = net_worth(Decimal::ZERO, &assets, &values, "EUR", d(2026, 3, 9), &Rates).unwrap();
    let after = net_worth(Decimal::ZERO, &assets, &values, "EUR", d(2026, 3, 10), &Rates).unwrap();

    assert_eq!(before.owned_base, Decimal::ZERO);
    assert_eq!(before.not_valued_yet, vec!["house".to_string()]);
    assert_eq!(during.owned_base, dec!(400000));
    assert_eq!(after.owned_base, Decimal::ZERO);
    assert!(after.holdings.is_empty());
    // Closed is not the same as never valued: nothing is waiting for a figure.
    assert!(after.not_valued_yet.is_empty());
}

#[test]
fn a_foreign_currency_asset_is_converted_at_the_reading_dates_rate() {
    let mut chalet = Asset::new("Chalet", AssetKind::Property, "CHF");
    chalet.id = "chf".into();
    let values = vec![AssetValue::new("chf", d(2026, 1, 1), dec!(200000))];

    let reading = net_worth(Decimal::ZERO, &[chalet], &values, "EUR", d(2026, 6, 30), &Rates).unwrap();

    // 200 000 CHF x 1.05 = 210 000 EUR.
    assert_eq!(reading.owned_base, dec!(210000));
    assert_eq!(reading.holdings[0].amount, dec!(200000));
}

#[test]
fn a_share_of_a_negative_net_worth_is_not_reported() {
    let assets = vec![mortgage()];
    let values = vec![AssetValue::new("debt", d(2026, 1, 1), dec!(250000))];

    let reading = net_worth(dec!(10000), &assets, &values, "EUR", d(2026, 6, 30), &Rates).unwrap();

    assert_eq!(reading.net_base, dec!(-240000));
    assert_eq!(reading.invested_share, None);
}

/// The line moves where something was measured: a valuation, a closing, or a portfolio day.
#[test]
fn the_series_has_a_point_where_the_sum_can_move() {
    let series = ValueSeries {
        base_currency: "EUR".into(),
        dates: vec![d(2026, 1, 1), d(2026, 4, 1)],
        total_value_base: vec![dec!(50000), dec!(55000)],
        external_flow_base: vec![Decimal::ZERO, Decimal::ZERO],
    };
    let assets = vec![house()];
    let values = vec![
        AssetValue::new("house", d(2026, 2, 1), dec!(400000)),
        AssetValue::new("house", d(2026, 5, 1), dec!(410000)),
    ];

    let line = net_worth_series(
        &series,
        &assets,
        &values,
        "EUR",
        d(2026, 1, 1),
        d(2026, 6, 30),
        &Rates,
    )
    .unwrap();

    let dates: Vec<NaiveDate> = line.points.iter().map(|p| p.date).collect();
    assert_eq!(
        dates,
        vec![
            d(2026, 1, 1),
            d(2026, 2, 1),
            d(2026, 4, 1),
            d(2026, 5, 1),
            d(2026, 6, 30)
        ]
    );
    // 1 January: invested only, the house is not valued yet.
    assert_eq!(line.points[0].net_base, dec!(50000));
    // 1 February: 50 000 + 400 000.
    assert_eq!(line.points[1].net_base, dec!(450000));
    // 1 April: the portfolio moved to 55 000, the house is unchanged.
    assert_eq!(line.points[2].net_base, dec!(455000));
    // 30 June: the last portfolio day still holds, the house is worth 410 000.
    assert_eq!(line.points[4].net_base, dec!(465000));
}

#[test]
fn a_missing_rate_fails_rather_than_counting_zero() {
    let mut gold = Asset::new("Coins", AssetKind::Collectible, "USD");
    gold.id = "usd".into();
    let values = vec![AssetValue::new("usd", d(2026, 1, 1), dec!(5000))];

    let error = net_worth(Decimal::ZERO, &[gold], &values, "EUR", d(2026, 6, 30), &Rates);
    assert!(error.is_err());
}
