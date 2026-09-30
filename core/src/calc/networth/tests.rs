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

/// The line skips the per-reading detail, so every point has to answer with what a full reading
/// of that same day answers: the line is the same figures, drawn cheaper.
#[test]
fn every_point_of_the_line_agrees_with_a_reading_of_that_day() {
    let series = ValueSeries {
        base_currency: "EUR".into(),
        dates: vec![d(2026, 1, 1), d(2026, 3, 1), d(2026, 6, 1)],
        total_value_base: vec![dec!(50000), dec!(55000), dec!(60000)],
        external_flow_base: vec![Decimal::ZERO, Decimal::ZERO, Decimal::ZERO],
    };
    let mut closed = Asset::new("Car", AssetKind::Vehicle, "CHF");
    closed.id = "car".into();
    closed.closed_on = Some(d(2026, 5, 1));
    let assets = vec![house(), mortgage(), closed];
    let values = vec![
        AssetValue::new("house", d(2026, 2, 1), dec!(400000)),
        AssetValue::new("house", d(2026, 5, 1), dec!(410000)),
        AssetValue::new("debt", d(2026, 1, 15), dec!(250000)),
        AssetValue::new("car", d(2026, 1, 15), dec!(20000)),
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

    assert!(!line.points.is_empty());
    for point in &line.points {
        let reading = net_worth(
            point.investments_base,
            &assets,
            &values,
            "EUR",
            point.date,
            &Rates,
        )
        .unwrap();
        assert_eq!(point.owned_base, reading.owned_base, "owned on {}", point.date);
        assert_eq!(point.owed_base, reading.owed_base, "owed on {}", point.date);
        assert_eq!(point.net_base, reading.net_base, "net on {}", point.date);
    }
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

/// 400 000 owned with 250 000 owed on it leaves 150 000 of equity, and the flat alone carries
/// that figure — the debt does not repeat it.
#[test]
fn a_debt_leaves_equity_on_the_thing_it_is_secured_on() {
    let assets = vec![house(), mortgage()];
    let values = vec![
        AssetValue::new("house", d(2026, 1, 1), dec!(400000)),
        AssetValue::new("debt", d(2026, 1, 1), dec!(250000)),
    ];

    let reading = net_worth(Decimal::ZERO, &assets, &values, "EUR", d(2026, 6, 30), &Rates).unwrap();

    let flat = reading.holdings.iter().find(|h| h.asset_id == "house").unwrap();
    assert_eq!(flat.secured_debt_base, Some(dec!(250000)));
    assert_eq!(flat.equity_base, Some(dec!(150000)));
    let debt = reading.holdings.iter().find(|h| h.asset_id == "debt").unwrap();
    assert_eq!(debt.equity_base, None);
}

/// Two debts on one flat are summed: 400 000 − 250 000 − 30 000 = 120 000 left.
#[test]
fn two_debts_on_one_asset_are_summed() {
    let mut renovation = Asset::new("Renovation loan", AssetKind::Loan, "EUR");
    renovation.id = "loan".into();
    renovation.secured_by = Some("house".into());
    let assets = vec![house(), mortgage(), renovation];
    let values = vec![
        AssetValue::new("house", d(2026, 1, 1), dec!(400000)),
        AssetValue::new("debt", d(2026, 1, 1), dec!(250000)),
        AssetValue::new("loan", d(2026, 1, 1), dec!(30000)),
    ];

    let reading = net_worth(Decimal::ZERO, &assets, &values, "EUR", d(2026, 6, 30), &Rates).unwrap();

    let flat = reading.holdings.iter().find(|h| h.asset_id == "house").unwrap();
    assert_eq!(flat.equity_base, Some(dec!(120000)));
}

/// Nothing owed on it means no equity figure: repeating the value under a second name would
/// invite adding the two together.
#[test]
fn an_asset_with_no_debt_on_it_carries_no_equity_figure() {
    let values = vec![AssetValue::new("house", d(2026, 1, 1), dec!(400000))];
    let reading = net_worth(Decimal::ZERO, &[house()], &values, "EUR", d(2026, 6, 30), &Rates).unwrap();

    assert_eq!(reading.holdings[0].secured_debt_base, None);
    assert_eq!(reading.holdings[0].equity_base, None);
}

/// The change is against the figure written before the one in force, not against the reading
/// date, and it names the day it measures from.
#[test]
fn a_change_is_measured_between_two_valuations() {
    let values = vec![
        AssetValue::new("house", d(2026, 1, 1), dec!(400000)),
        AssetValue::new("house", d(2026, 4, 1), dec!(415000)),
    ];
    let reading = net_worth(Decimal::ZERO, &[house()], &values, "EUR", d(2026, 6, 30), &Rates).unwrap();

    assert_eq!(reading.holdings[0].change_base, Some(dec!(15000)));
    assert_eq!(reading.holdings[0].changed_since, Some(d(2026, 1, 1)));

    // One figure has nothing to be compared with, and zero would be a claim.
    let single = net_worth(
        Decimal::ZERO,
        &[house()],
        &values[..1],
        "EUR",
        d(2026, 6, 30),
        &Rates,
    )
    .unwrap();
    assert_eq!(single.holdings[0].change_base, None);
}

/// A foreign asset's change is the revaluation alone: both figures are converted at the reading
/// date's rate, so a currency that moved in between does not show up as a gain here.
#[test]
fn a_change_in_a_foreign_currency_is_not_a_currency_move() {
    let mut chalet = Asset::new("Chalet", AssetKind::Property, "CHF");
    chalet.id = "chf".into();
    let values = vec![
        AssetValue::new("chf", d(2026, 1, 1), dec!(200000)),
        AssetValue::new("chf", d(2026, 4, 1), dec!(210000)),
    ];

    let reading = net_worth(Decimal::ZERO, &[chalet], &values, "EUR", d(2026, 6, 30), &Rates).unwrap();

    // 10 000 CHF more, at 1.05: 10 500 EUR, and not a cent of rate difference.
    assert_eq!(reading.holdings[0].change_base, Some(dec!(10500)));
}

/// Age is counted to the reading date, and past the fuse it is worth saying out loud.
#[test]
fn an_old_figure_is_counted_and_named() {
    let values = vec![AssetValue::new("house", d(2026, 1, 1), dec!(400000))];

    let fresh = net_worth(Decimal::ZERO, &[house()], &values, "EUR", d(2026, 3, 1), &Rates).unwrap();
    assert_eq!(fresh.holdings[0].days_old, 59);
    assert!(!fresh.holdings[0].stale);
    assert_eq!(fresh.stale_count, 0);

    let old = net_worth(Decimal::ZERO, &[house()], &values, "EUR", d(2026, 12, 31), &Rates).unwrap();
    assert_eq!(old.holdings[0].days_old, 364);
    assert!(old.holdings[0].stale);
    assert_eq!(old.stale_count, 1);
}

/// 250 000 owed against 60 000 invested plus 400 000 owned: 250 000 / 460 000 of the assets.
#[test]
fn debt_is_measured_against_everything_owned_investments_included() {
    let assets = vec![house(), mortgage()];
    let values = vec![
        AssetValue::new("house", d(2026, 1, 1), dec!(400000)),
        AssetValue::new("debt", d(2026, 1, 1), dec!(250000)),
    ];

    let reading = net_worth(dec!(60000), &assets, &values, "EUR", d(2026, 6, 30), &Rates).unwrap();

    assert_eq!(
        reading.debt_to_assets.map(|r| r.round_dp(6)),
        Some((dec!(250000) / dec!(460000)).round_dp(6))
    );

    // A debt and nothing owned is not a ratio.
    let only_debt = net_worth(
        Decimal::ZERO,
        &[mortgage()],
        &values[1..],
        "EUR",
        d(2026, 6, 30),
        &Rates,
    )
    .unwrap();
    assert_eq!(only_debt.debt_to_assets, None);
}

/// A schedule on a debt answers when it ends; a thing owned never carries one.
#[test]
fn only_a_debt_with_a_schedule_gets_a_payoff() {
    let mut debt = mortgage();
    debt.schedule = Some(crate::model::Amortization {
        rate: dec!(0.12),
        monthly_payment: dec!(400),
        ends_on: None,
    });
    let values = vec![
        AssetValue::new("house", d(2026, 1, 1), dec!(400000)),
        AssetValue::new("debt", d(2026, 1, 1), dec!(1000)),
    ];

    let reading = net_worth(
        Decimal::ZERO,
        &[house(), debt],
        &values,
        "EUR",
        d(2026, 6, 30),
        &Rates,
    )
    .unwrap();

    let flat = reading.holdings.iter().find(|h| h.asset_id == "house").unwrap();
    assert!(flat.payoff.is_none());
    let owed = reading.holdings.iter().find(|h| h.asset_id == "debt").unwrap();
    let payoff = owed.payoff.as_ref().unwrap();
    // The same three months `payoff::tests` works out longhand, from the reading date.
    assert_eq!(payoff.months_left, Some(3));
    assert_eq!(payoff.payoff_on, Some(d(2026, 9, 30)));
}

/// One figure is not progress: a debt valued once reports no share paid, rather than a 0% that
/// stands still until a second figure exists.
#[test]
fn a_debt_valued_once_reports_no_progress() {
    let mut debt = mortgage();
    debt.schedule = Some(crate::model::Amortization {
        rate: dec!(0.12),
        monthly_payment: dec!(400),
        ends_on: None,
    });
    let once = vec![AssetValue::new("debt", d(2026, 1, 1), dec!(1000))];

    let reading = net_worth(
        Decimal::ZERO,
        &[debt.clone()],
        &once,
        "EUR",
        d(2026, 6, 30),
        &Rates,
    )
    .unwrap();
    let payoff = reading.holdings[0].payoff.as_ref().unwrap();
    assert_eq!(payoff.paid_share, None);
    // The rest of the forward reading is still there: it needs today's figure, not a history.
    assert_eq!(payoff.months_left, Some(3));

    // A second figure makes it measurable: 1 200 down to 1 000 is a sixth of it gone.
    let twice = vec![
        AssetValue::new("debt", d(2026, 1, 1), dec!(1200)),
        AssetValue::new("debt", d(2026, 4, 1), dec!(1000)),
    ];
    let reading = net_worth(Decimal::ZERO, &[debt], &twice, "EUR", d(2026, 6, 30), &Rates).unwrap();
    let payoff = reading.holdings[0].payoff.as_ref().unwrap();
    assert_eq!(
        payoff.paid_share.map(|s| s.round_dp(6)),
        Some((dec!(200) / dec!(1200)).round_dp(6))
    );
}
