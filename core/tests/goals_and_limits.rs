//! Savings goals and contribution limits over a real store. See ADR-0068.

mod support;

use rust_decimal_macros::dec;
use sq_core::calc::PortfolioAnalytics;
use sq_core::market::Quote;
use sq_core::model::{
    Account, ContributionLimit, Goal, Portfolio, Security, SecurityKind, Transaction, TransactionKind,
};
use sq_core::storage::Store;
use support::d;

struct World {
    store: Store,
    portfolio: Portfolio,
    isa: Account,
    depot: Account,
    other: Account,
}

/// Two cash accounts: an ISA holding 12 000 of deposits and 100 shares at 100, and an ordinary
/// one holding 5 000. The ISA is therefore worth 22 000 on its own.
fn seeded() -> World {
    let store = Store::open_in_memory().unwrap();

    let isa = Account::deposit("ISA cash", "GBP");
    store.save_account(&isa).unwrap();
    let depot = Account::securities("ISA depot", "GBP", &isa.id);
    store.save_account(&depot).unwrap();
    let other = Account::deposit("Current", "GBP");
    store.save_account(&other).unwrap();

    let etf = Security::new("VWRL", "FTSE All-World", "GBP", SecurityKind::Etf);
    store.save_security(&etf).unwrap();
    store
        .save_quotes(&[Quote {
            security_id: etf.id.clone(),
            date: d(2025, 6, 30),
            close: dec!(100),
            currency: "GBP".into(),
            source: "manual".into(),
        }])
        .unwrap();

    for t in [
        Transaction::cash(
            &isa.id,
            TransactionKind::Deposit,
            d(2025, 5, 1),
            dec!(12000),
            "GBP",
        ),
        Transaction::buy(&depot.id, &etf.id, d(2025, 5, 2), dec!(100), dec!(100), "GBP"),
        Transaction::cash(
            &other.id,
            TransactionKind::Deposit,
            d(2025, 5, 1),
            dec!(5000),
            "GBP",
        ),
    ] {
        store.save_transaction(&t).unwrap();
    }

    let portfolio =
        Portfolio::new("Main", "GBP").with_accounts([isa.id.clone(), depot.id.clone(), other.id.clone()]);
    store.save_portfolio(&portfolio).unwrap();

    World {
        store,
        portfolio,
        isa,
        depot,
        other,
    }
}

/// A goal reads the accounts it names, not the whole portfolio and not the picker's.
#[test]
fn a_goal_counts_only_the_accounts_it_names() {
    let world = seeded();
    let analytics = PortfolioAnalytics::new(&world.store, &world.portfolio).unwrap();

    let mut goal = Goal::new("House", dec!(40000), "GBP", d(2025, 1, 1));
    goal.accounts = vec![world.isa.id.clone()];
    world.store.save_goal(&world.portfolio.id, &goal).unwrap();

    // The ISA cash account alone: 12 000 paid in, 10 000 of it spent on shares.
    let progress = analytics.goal_progress(&goal, d(2025, 6, 30)).unwrap();
    assert_eq!(progress.current_base, dec!(2000));

    // Naming no account is the whole portfolio: 2 000 + 10 000 of shares + 5 000 elsewhere.
    let mut whole = goal.clone();
    whole.accounts.clear();
    let progress = analytics.goal_progress(&whole, d(2025, 6, 30)).unwrap();
    assert_eq!(progress.current_base, dec!(17000));
    assert_eq!(progress.missing_base, dec!(23000));
}

/// A goal survives being re-read: its accounts come back with it.
#[test]
fn a_saved_goal_keeps_its_accounts() {
    let world = seeded();
    let mut goal = Goal::new("House", dec!(40000), "GBP", d(2025, 1, 1));
    goal.accounts = vec![world.isa.id.clone(), world.other.id.clone()];
    goal.target_date = Some(d(2027, 1, 1));
    world.store.save_goal(&world.portfolio.id, &goal).unwrap();

    let stored = world.store.get_goal(&goal.id).unwrap();
    assert_eq!(stored.accounts.len(), 2);
    assert_eq!(stored.target_date, Some(d(2027, 1, 1)));
    assert_eq!(world.store.list_goals(&world.portfolio.id).unwrap().len(), 1);

    // Deleting an account takes it off the goal and leaves the goal standing.
    world.store.delete_account(&world.other.id).unwrap();
    assert_eq!(
        world.store.get_goal(&goal.id).unwrap().accounts,
        vec![world.isa.id]
    );
}

/// A limit counts what was paid into its own account over its own year, and nothing else.
#[test]
fn a_limit_counts_its_accounts_own_year() {
    let world = seeded();
    let analytics = PortfolioAnalytics::new(&world.store, &world.portfolio).unwrap();

    let mut limit = ContributionLimit::new(&world.isa.id, "ISA", dec!(20000), "GBP");
    limit.year_starts_on = "04-06".into();
    world.store.save_limit(&limit).unwrap();

    let usage = analytics.limit_usage(&limit, d(2025, 6, 30)).unwrap();
    assert_eq!(usage.from, d(2025, 4, 6));
    assert_eq!(usage.to, d(2026, 4, 5));
    // The 12 000 landed on the ISA; the 5 000 on the other account is another allowance's.
    assert_eq!(usage.used, dec!(12000));
    assert_eq!(usage.remaining, dec!(8000));

    // The year before it opened saw nothing.
    let earlier = analytics.limit_usage(&limit, d(2025, 4, 1)).unwrap();
    assert_eq!(earlier.used, dec!(0));
}

/// Buying shares with money already inside the account is not a contribution.
#[test]
fn spending_inside_the_account_eats_no_allowance() {
    let world = seeded();
    let analytics = PortfolioAnalytics::new(&world.store, &world.portfolio).unwrap();
    let limit = ContributionLimit::new(&world.isa.id, "ISA", dec!(20000), "GBP");

    let before = analytics.limit_usage(&limit, d(2025, 6, 30)).unwrap().used;
    let buy = Transaction::buy(
        &world.depot.id,
        &world.store.list_securities().unwrap()[0].id,
        d(2025, 6, 1),
        dec!(10),
        dec!(100),
        "GBP",
    );
    world.store.save_transaction(&buy).unwrap();

    let after = analytics.limit_usage(&limit, d(2025, 6, 30)).unwrap().used;
    assert_eq!(before, after);
}
