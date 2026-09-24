//! Everything the demo needs besides the ledger: classifications and a target, a savings goal
//! and an allowance, two plans, a watchlist, alerts, instrument events and attributes — so every
//! screen and every dashboard widget has real rows to draw rather than an empty state.

use super::World;
use chrono::{Datelike, Days, Months};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sq_core::model::{CashClassification, SecurityClassification, TaxonomyNode};
use sq_core::prelude::*;
use std::collections::BTreeMap;

/// The trees seeded by the migration; their ids are fixed on purpose, so the demo classifies into
/// what the user already has instead of inventing a second "Asset class".
const ASSET_CLASS: &str = "tax-asset-class";

pub fn write(store: &Store, portfolio: &Portfolio, world: &World) -> Result<()> {
    let cash_node = cash_node(store)?;
    classify(store, world, &cash_node)?;
    target(store, portfolio, &cash_node)?;
    goals(store, portfolio, world)?;
    plans(store, portfolio, world)?;
    watchlist(store, world)?;
    alerts(store, world)?;
    events(store, world)?;
    attributes(store, world)?;
    Ok(())
}

/// The shipped asset-class tree leaves cash out, and a target that ignores the cash a portfolio
/// holds drifts by definition. The demo adds the node rather than the migration: an existing
/// database must keep the tree its owner arranged.
fn cash_node(store: &Store) -> Result<TaxonomyNode> {
    let node = TaxonomyNode {
        color: Some(8),
        ..TaxonomyNode::root(ASSET_CLASS, "Cash").with_rank(5)
    };
    store.save_taxonomy_node(&node)?;
    Ok(node)
}

fn classify(store: &Store, world: &World, cash_node: &TaxonomyNode) -> Result<()> {
    let s = &world.instruments;
    let splits: [(&str, &[(&str, Decimal)]); 7] = [
        (
            &s.allworld.id,
            &[("tn-ac-eq-dev", dec!(0.88)), ("tn-ac-eq-em", dec!(0.12))],
        ),
        (&s.core_world.id, &[("tn-ac-eq-dev", Decimal::ONE)]),
        (&s.apple.id, &[("tn-ac-eq-single", Decimal::ONE)]),
        (&s.microsoft.id, &[("tn-ac-eq-single", Decimal::ONE)]),
        (&s.netflix.id, &[("tn-ac-eq-single", Decimal::ONE)]),
        (
            &s.bonds.id,
            &[("tn-ac-bond-gov", dec!(0.6)), ("tn-ac-bond-corp", dec!(0.4))],
        ),
        (&s.bitcoin.id, &[("tn-ac-crypto", Decimal::ONE)]),
    ];
    // Region deliberately says nothing about bitcoin: an unclassified bucket is part of what the
    // allocation screen has to show.
    let regions: [(&str, &[(&str, Decimal)]); 6] = [
        (
            &s.allworld.id,
            &[
                ("tn-rg-us", dec!(0.62)),
                ("tn-rg-ez", dec!(0.11)),
                ("tn-rg-eu-ex", dec!(0.07)),
                ("tn-rg-jp", dec!(0.06)),
                ("tn-rg-apac", dec!(0.04)),
                ("tn-rg-em-asia", dec!(0.08)),
                ("tn-rg-em-other", dec!(0.02)),
            ],
        ),
        (
            &s.core_world.id,
            &[
                ("tn-rg-us", dec!(0.71)),
                ("tn-rg-ez", dec!(0.10)),
                ("tn-rg-eu-ex", dec!(0.07)),
                ("tn-rg-jp", dec!(0.06)),
                ("tn-rg-apac", dec!(0.06)),
            ],
        ),
        (&s.bonds.id, &[("tn-rg-us", dec!(0.45)), ("tn-rg-ez", dec!(0.55))]),
        (&s.apple.id, &[("tn-rg-us", Decimal::ONE)]),
        (&s.microsoft.id, &[("tn-rg-us", Decimal::ONE)]),
        (&s.netflix.id, &[("tn-rg-us", Decimal::ONE)]),
    ];
    let sectors: [(&str, &str); 3] = [
        (&s.apple.id, "tn-sc-hardware"),
        (&s.microsoft.id, "tn-sc-software"),
        (&s.netflix.id, "tn-sc-consumer"),
    ];

    for (security_id, nodes) in splits.into_iter().chain(regions) {
        for (node_id, weight) in nodes {
            store.save_classification(&SecurityClassification::new(security_id, node_id, *weight))?;
        }
    }
    for (security_id, node_id) in sectors {
        store.save_classification(&SecurityClassification::new(security_id, node_id, Decimal::ONE))?;
    }

    for (account, currency) in [
        (&world.accounts.euro_cash, "EUR"),
        (&world.accounts.usd_cash, "USD"),
        (&world.accounts.savings, "EUR"),
    ] {
        store.save_cash_classification(&CashClassification::new(
            &account.id,
            currency,
            &cash_node.id,
            Decimal::ONE,
        ))?;
    }
    Ok(())
}

/// Weights are shares of the parent, so these four top-level nodes add up to one and the rest of
/// the tree is simply not part of the plan.
fn target(store: &Store, portfolio: &Portfolio, cash_node: &TaxonomyNode) -> Result<()> {
    let target = AllocationTarget::new(&portfolio.id, ASSET_CLASS, "Long-term plan")
        .with_weight("tn-ac-equity", dec!(0.70))
        .with_weight("tn-ac-bonds", dec!(0.20))
        .with_weight("tn-ac-crypto", dec!(0.05))
        .with_weight(&cash_node.id, dec!(0.05));
    store.save_target(&target)
}

fn goals(store: &Store, portfolio: &Portfolio, world: &World) -> Result<()> {
    let opened = world.days[0];
    let apartment = Goal {
        target_date: world.today.checked_add_months(Months::new(48)),
        monthly_amount: Some(dec!(700)),
        expected_return: dec!(0.05),
        note: Some("Deposit on a flat, the whole portfolio counts towards it".into()),
        ..Goal::new("Apartment deposit", dec!(120000), "EUR", opened)
    };
    let emergency = Goal {
        monthly_amount: Some(dec!(150)),
        expected_return: dec!(0.026),
        accounts: vec![world.accounts.savings.id.clone()],
        ..Goal::new("Emergency fund", dec!(15000), "EUR", opened)
    };
    store.save_goal(&portfolio.id, &apartment)?;
    store.save_goal(&portfolio.id, &emergency)?;

    // An allowance is measured, never enforced: nothing refuses a deposit over it.
    store.save_limit(&ContributionLimit {
        note: Some("Yearly allowance on the broker account".into()),
        ..ContributionLimit::new(
            &world.accounts.euro_cash.id,
            "Tax-free allowance",
            dec!(9000),
            "EUR",
        )
    })?;
    store.save_limit(&ContributionLimit {
        year_starts_on: "04-06".into(),
        withdrawals_restore: true,
        ..ContributionLimit::new(&world.accounts.savings.id, "Savings allowance", dec!(4000), "EUR")
    })?;
    Ok(())
}

/// Both plans start next month: a plan is an intention about what comes, and one dated into the
/// past would open the app with three years of occurrences waiting to be committed.
fn plans(store: &Store, portfolio: &Portfolio, world: &World) -> Result<()> {
    let start = world
        .today
        .checked_add_months(Months::new(1))
        .and_then(|d| d.with_day(5))
        .unwrap_or(world.today);
    let core = InvestmentPlan::new(
        &portfolio.id,
        &world.accounts.euro_depot.id,
        "Monthly savings plan",
        dec!(650),
        "EUR",
        Schedule::monthly(start),
    )
    .with_leg(&world.instruments.allworld.id, dec!(0.7))
    .with_leg(&world.instruments.bonds.id, dec!(0.3))
    .with_costs(dec!(1), Decimal::ZERO);
    let crypto = InvestmentPlan::new(
        &portfolio.id,
        &world.accounts.euro_depot.id,
        "Crypto, every quarter",
        dec!(150),
        "EUR",
        Schedule::monthly(start).every(3, Interval::Month),
    )
    .with_leg(&world.instruments.bitcoin.id, Decimal::ONE)
    .with_costs(dec!(1), Decimal::ZERO);
    store.save_plan(&core)?;
    store.save_plan(&crypto)
}

fn watchlist(store: &Store, world: &World) -> Result<()> {
    let list = Watchlist::new("Ideas")
        .with(&world.instruments.nvidia.id)
        .with(&world.instruments.sp500.id)
        .with(&world.instruments.netflix.id);
    store.save_watchlist(&list)
}

/// Levels sit where the series has already been, so the first check finds crossings to log and
/// the alert screen is not an empty list on a portfolio three years long.
/// A level sits where the series has already been, so the first check finds a crossing or two to
/// log — but only over a few months: every crossing is an announcement, and a demo that greets
/// its user with twenty notifications has misread what a demo is for.
fn alerts(store: &Store, world: &World) -> Result<()> {
    let months_ago = |months: u32| {
        world
            .today
            .checked_sub_months(Months::new(months))
            .unwrap_or(world.today)
    };

    let days_ago = |days: u64| {
        world
            .today
            .checked_sub_days(Days::new(days))
            .unwrap_or(world.today)
    };
    let apple_level = world.price(&world.instruments.apple, world.index_on(days_ago(21)));
    store.save_alert(
        &SecurityAlert::price(&world.instruments.apple.id, apple_level, "USD", days_ago(42))
            .with_direction(AlertDirection::Up)
            .with_note("Where I would add to the position"),
    )?;

    // Nothing has crossed this one yet: a rule that is simply waiting is the usual case.
    let last = world.days.len() - 1;
    let floor = (world.price(&world.instruments.allworld, last) * dec!(0.92)).round_dp(2);
    store.save_alert(
        &SecurityAlert::price(
            &world.instruments.allworld.id,
            floor,
            "EUR",
            world.today.checked_sub_days(Days::new(30)).unwrap_or(world.today),
        )
        .with_direction(AlertDirection::Down)
        .with_note("An 8% dip is a buying month"),
    )?;

    let review = world.today.checked_sub_days(Days::new(3)).unwrap_or(world.today);
    store.save_alert(
        &SecurityAlert::date_reached(&world.instruments.bitcoin.id, review, months_ago(2))
            .with_note("Review how large the crypto sleeve has grown"),
    )
}

fn events(store: &Store, world: &World) -> Result<()> {
    let ago = |months: u32| {
        world
            .today
            .checked_sub_months(Months::new(months))
            .unwrap_or(world.today)
    };
    for note in [
        (
            &world.instruments.apple.id,
            ago(2),
            "Quarterly results beat guidance",
        ),
        (
            &world.instruments.bitcoin.id,
            ago(7),
            "Halving; the sleeve was not topped up",
        ),
        (
            &world.instruments.allworld.id,
            ago(14),
            "Switched the monthly plan onto this fund",
        ),
    ] {
        store.save_security_event(&SecurityEvent::note(note.0, note.1, note.2))?;
    }

    // What a provider would have reported beside the quotes, so the events list is not only notes.
    let reported = [
        SecurityEvent::dividend(&world.instruments.apple.id, ago(1), dec!(0.25), "USD", "manual"),
        SecurityEvent::dividend(&world.instruments.apple.id, ago(4), dec!(0.25), "USD", "manual"),
        SecurityEvent::dividend(
            &world.instruments.microsoft.id,
            ago(3),
            dec!(0.83),
            "USD",
            "manual",
        ),
        SecurityEvent::dividend(&world.instruments.bonds.id, ago(2), dec!(0.032), "EUR", "manual"),
    ];
    store.save_provider_events(&reported)?;
    Ok(())
}

/// Two attributes: one a number the instruments screen can sort on, one a text column a
/// classification tree can be filled from.
fn attributes(store: &Store, world: &World) -> Result<()> {
    let yield_def = SecurityAttributeDef::new("Dividend yield", AttributeKind::Number).with_unit("%");
    let strategy = SecurityAttributeDef::new("Strategy", AttributeKind::Text);
    store.save_attribute_def(&yield_def)?;
    store.save_attribute_def(&strategy)?;

    let s = &world.instruments;
    for (security_id, dividend_yield, role) in [
        (&s.allworld.id, "1.80", "Core"),
        (&s.core_world.id, "1.50", "Core"),
        (&s.bonds.id, "3.40", "Defensive"),
        (&s.apple.id, "0.55", "Satellite"),
        (&s.microsoft.id, "0.75", "Satellite"),
        (&s.netflix.id, "0.00", "Satellite"),
        (&s.bitcoin.id, "0.00", "Speculative"),
    ] {
        let mut values = BTreeMap::new();
        values.insert(yield_def.id.clone(), dividend_yield.to_string());
        values.insert(strategy.id.clone(), role.to_string());
        store.set_security_attributes(security_id, &values)?;
    }
    Ok(())
}
