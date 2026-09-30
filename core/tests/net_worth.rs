//! Net worth over a real store: a flat, the mortgage on it, and the promise that neither one
//! reaches a single figure of the portfolio. See ADR-0092.

mod support;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sq_core::calc::PortfolioAnalytics;
use sq_core::fx::FxRate;
use sq_core::market::Quote;
use sq_core::model::{
    Account, Amortization, Asset, AssetKind, AssetValue, Portfolio, Security, SecurityKind, Transaction,
    TransactionKind,
};
use sq_core::storage::Store;
use support::d;

struct World {
    store: Store,
    portfolio: Portfolio,
    house: Asset,
    mortgage: Asset,
}

/// A portfolio worth 30 000 on 30 June 2026 — 20 000 of deposits, 100 shares at 200, so 10 000
/// of cash left — beside a flat valued at 400 000 and a mortgage of 250 000 still owed.
fn seeded() -> World {
    let store = Store::open_in_memory().unwrap();

    let cash = Account::deposit("Bank", "EUR");
    store.save_account(&cash).unwrap();
    let depot = Account::securities("Broker", "EUR", &cash.id);
    store.save_account(&depot).unwrap();

    let etf = Security::new("VWCE", "FTSE All-World", "EUR", SecurityKind::Etf);
    store.save_security(&etf).unwrap();
    store
        .save_quotes(&[Quote {
            security_id: etf.id.clone(),
            date: d(2026, 1, 2),
            close: dec!(200),
            currency: "EUR".into(),
            source: "manual".into(),
        }])
        .unwrap();

    for t in [
        Transaction::cash(
            &cash.id,
            TransactionKind::Deposit,
            d(2026, 1, 1),
            dec!(20000),
            "EUR",
        ),
        Transaction::buy(&depot.id, &etf.id, d(2026, 1, 2), dec!(100), dec!(100), "EUR"),
    ] {
        store.save_transaction(&t).unwrap();
    }

    let portfolio = Portfolio::new("Main", "EUR").with_accounts([cash.id.clone(), depot.id.clone()]);
    store.save_portfolio(&portfolio).unwrap();

    let house = Asset::new("Flat", AssetKind::Property, "EUR");
    store.save_asset(&portfolio.id, &house).unwrap();
    let mut mortgage = Asset::new("Mortgage", AssetKind::Mortgage, "EUR");
    mortgage.secured_by = Some(house.id.clone());
    mortgage.schedule = Some(Amortization {
        rate: dec!(0.0345),
        monthly_payment: dec!(1100),
        ends_on: Some(d(2049, 5, 1)),
    });
    store.save_asset(&portfolio.id, &mortgage).unwrap();

    store
        .save_asset_value(&AssetValue::new(&house.id, d(2026, 1, 15), dec!(400000)))
        .unwrap();
    store
        .save_asset_value(&AssetValue::new(&mortgage.id, d(2026, 1, 15), dec!(250000)))
        .unwrap();

    World {
        store,
        portfolio,
        house,
        mortgage,
    }
}

/// 30 000 invested + 400 000 owned − 250 000 owed = 180 000; invested is 30 000 / 180 000 of it.
#[test]
fn net_worth_adds_what_is_owned_and_subtracts_what_is_owed() {
    let world = seeded();
    let analytics = PortfolioAnalytics::new(&world.store, &world.portfolio).unwrap();

    let reading = analytics.net_worth(d(2026, 6, 30)).unwrap();

    assert_eq!(reading.investments_base, dec!(30000));
    assert_eq!(reading.owned_base, dec!(400000));
    assert_eq!(reading.owed_base, dec!(250000));
    assert_eq!(reading.net_base, dec!(180000));
    assert_eq!(
        reading.invested_share.unwrap().round_dp(6),
        (dec!(30000) / dec!(180000)).round_dp(6)
    );
    // The debt names what it is secured by, and nothing about the flat depends on that.
    let debt = reading
        .holdings
        .iter()
        .find(|h| h.asset_id == world.mortgage.id)
        .unwrap();
    assert_eq!(debt.secured_by.as_deref(), Some(world.house.id.as_str()));
}

/// The whole point of the decision: assets are a second total, so the portfolio's own figures are
/// the same with them and without them, down to the last cent.
#[test]
fn a_flat_and_a_mortgage_move_no_figure_of_the_portfolio() {
    let world = seeded();
    let analytics = PortfolioAnalytics::new(&world.store, &world.portfolio).unwrap();

    let with = analytics.valuation_at(d(2026, 6, 30)).unwrap();
    let twr_with = analytics.twr(d(2026, 1, 1), d(2026, 6, 30)).unwrap();
    let allocation_with = analytics.allocation_by_account(d(2026, 6, 30)).unwrap();

    world.store.delete_asset(&world.house.id).unwrap();
    world.store.delete_asset(&world.mortgage.id).unwrap();
    let without = analytics.valuation_at(d(2026, 6, 30)).unwrap();

    assert_eq!(with, without);
    assert_eq!(twr_with, analytics.twr(d(2026, 1, 1), d(2026, 6, 30)).unwrap());
    assert_eq!(
        allocation_with.total_base,
        analytics
            .allocation_by_account(d(2026, 6, 30))
            .unwrap()
            .total_base
    );
    // And with both gone, net worth is the portfolio again.
    let reading = analytics.net_worth(d(2026, 6, 30)).unwrap();
    assert_eq!(reading.net_base, dec!(30000));
    assert!(reading.holdings.is_empty());
}

/// A valuation is one figure per day: answering the same day again replaces that answer, and the
/// series keeps every other day it was asked.
#[test]
fn re_valuing_a_day_replaces_that_days_figure() {
    let world = seeded();

    world
        .store
        .save_asset_value(&AssetValue::new(&world.house.id, d(2026, 1, 15), dec!(415000)))
        .unwrap();
    world
        .store
        .save_asset_value(&AssetValue::new(&world.house.id, d(2026, 7, 1), dec!(430000)))
        .unwrap();

    let history = world.store.asset_values(&world.house.id).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].amount, dec!(415000));
    assert_eq!(history[1].date, d(2026, 7, 1));

    let analytics = PortfolioAnalytics::new(&world.store, &world.portfolio).unwrap();
    // 30 June still reads January's figure; 1 July reads July's.
    assert_eq!(
        analytics.net_worth(d(2026, 6, 30)).unwrap().owned_base,
        dec!(415000)
    );
    assert_eq!(
        analytics.net_worth(d(2026, 7, 1)).unwrap().owned_base,
        dec!(430000)
    );
}

/// The saved asset comes back as it went in, schedule included, and deleting it takes its
/// valuations with it while the flat it was secured on stays.
#[test]
fn an_asset_survives_a_round_trip_and_its_valuations_follow_it() {
    let world = seeded();

    let stored = world.store.get_asset(&world.mortgage.id).unwrap();
    assert_eq!(stored, world.mortgage);
    assert_eq!(stored.schedule.unwrap().ends_on, Some(d(2049, 5, 1)));
    assert_eq!(world.store.list_assets(&world.portfolio.id).unwrap().len(), 2);

    world.store.delete_asset(&world.mortgage.id).unwrap();
    assert!(world.store.asset_values(&world.mortgage.id).unwrap().is_empty());
    let left = world.store.list_assets(&world.portfolio.id).unwrap();
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].id, world.house.id);
}

/// Sold on a day, gone from that day: the flat leaves net worth and its history stays readable.
#[test]
fn a_closed_asset_leaves_net_worth_and_keeps_its_history() {
    let world = seeded();
    let mut house = world.house.clone();
    house.closed_on = Some(d(2026, 4, 1));
    world.store.save_asset(&world.portfolio.id, &house).unwrap();

    let analytics = PortfolioAnalytics::new(&world.store, &world.portfolio).unwrap();
    assert_eq!(
        analytics.net_worth(d(2026, 3, 31)).unwrap().owned_base,
        dec!(400000)
    );
    assert_eq!(
        analytics.net_worth(d(2026, 4, 1)).unwrap().owned_base,
        Decimal::ZERO
    );
    assert_eq!(world.store.asset_values(&house.id).unwrap().len(), 1);
}

/// A foreign-currency asset is converted at the reading date's rate, like every other figure
/// shown in the base currency.
#[test]
fn a_foreign_asset_needs_a_rate_and_uses_the_reading_dates_one() {
    let world = seeded();
    let mut coins = Asset::new("Gold coins", AssetKind::Collectible, "CHF");
    coins.id = "coins".into();
    world.store.save_asset(&world.portfolio.id, &coins).unwrap();
    world
        .store
        .save_asset_value(&AssetValue::new(&coins.id, d(2026, 2, 1), dec!(20000)))
        .unwrap();

    let analytics = PortfolioAnalytics::new(&world.store, &world.portfolio).unwrap();
    // No CHF rate stored yet, so the reading fails instead of counting the coins as nothing.
    assert!(analytics.net_worth(d(2026, 6, 30)).is_err());

    world
        .store
        .save_fx_rates(&[
            FxRate::new("CHF", "EUR", d(2026, 2, 1), dec!(1.02)),
            FxRate::new("CHF", "EUR", d(2026, 6, 30), dec!(1.05)),
        ])
        .unwrap();

    // 20 000 CHF at June's 1.05, not February's 1.02: 21 000 EUR on top of the flat.
    let reading = analytics.net_worth(d(2026, 6, 30)).unwrap();
    assert_eq!(reading.owned_base, dec!(400000) + dec!(21000));
}

/// The line has a point where the sum can move and nowhere else, and it ends on the day asked for.
#[test]
fn the_net_worth_line_steps_between_valuations() {
    let world = seeded();
    world
        .store
        .save_asset_value(&AssetValue::new(&world.house.id, d(2026, 4, 1), dec!(420000)))
        .unwrap();

    let analytics = PortfolioAnalytics::new(&world.store, &world.portfolio).unwrap();
    let line = analytics.net_worth_series(d(2026, 1, 1), d(2026, 6, 30)).unwrap();

    let first = line.points.first().unwrap();
    let last = line.points.last().unwrap();
    // 1 January: the deposit has not landed in a valuation of anything but the portfolio, and
    // the flat has no figure yet, so net worth is the portfolio alone.
    assert_eq!(first.date, d(2026, 1, 1));
    assert_eq!(first.owned_base, Decimal::ZERO);
    // 30 June: 30 000 invested + 420 000 - 250 000 = 200 000.
    assert_eq!(last.date, d(2026, 6, 30));
    assert_eq!(last.net_base, dec!(200000));
    // 15 January and 1 April are the valuation days, and both are on the line.
    let dates: Vec<_> = line.points.iter().map(|p| p.date).collect();
    assert!(dates.contains(&d(2026, 1, 15)));
    assert!(dates.contains(&d(2026, 4, 1)));
}
