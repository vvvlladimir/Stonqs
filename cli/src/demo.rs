//! The scenario every other subcommand reads: one in-memory portfolio with enough history to
//! make a series, a benchmark and a rebalance say something.

use crate::fmt::{money, percent};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sq_core::model::{SecurityClassification, TaxonomyNode};
use sq_core::prelude::*;

/// Shared in-memory data for commands that compare portfolio calculations.
pub struct DemoWorld {
    pub store: Store,
    pub portfolio: Portfolio,
    /// Accounts are creation-ordered; the first brokerage account is USD-based.
    pub accounts: Vec<Account>,
    pub apple: Security,
    pub world: Security,
    pub benchmark: Security,
    pub taxonomy_id: String,
    pub target: AllocationTarget,
}

impl DemoWorld {
    pub fn analytics(&self) -> Result<PortfolioAnalytics<'_>> {
        PortfolioAnalytics::new(&self.store, &self.portfolio)
    }

    /// Default date range covering the demo year.
    pub fn default_range(&self) -> DateRange {
        DateRange::new(day(6, 1), day(12, 31))
    }
}

pub fn day(month: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2024, month, d).expect("valid demo date")
}

pub fn demo_world() -> Result<DemoWorld> {
    let store = Store::open_in_memory()?;

    // Securities and cash use separate accounts.
    let ib_cash = Account::deposit("IBKR · cash", "USD");
    let tr_cash = Account::deposit("Trade Republic · cash", "EUR");
    store.save_account(&ib_cash)?;
    store.save_account(&tr_cash)?;
    let ib = Account::securities("Interactive Brokers", "USD", &ib_cash.id);
    let tr = Account::securities("Trade Republic", "EUR", &tr_cash.id);
    store.save_account(&ib)?;
    store.save_account(&tr)?;

    let apple = Security::new("AAPL", "Apple Inc.", "USD", SecurityKind::Stock).with_source("yahoo", "AAPL");
    // Fractional trading is a broker property, so the quantity step belongs here.
    let world = Security::new("IWDA", "iShares Core MSCI World", "EUR", SecurityKind::Etf)
        .with_quantity_step(dec!(0.001));
    let benchmark = Security::new("SPX", "S&P 500", "USD", SecurityKind::Other);
    store.save_security(&apple)?;
    store.save_security(&world)?;
    store.save_security(&benchmark)?;

    let portfolio = Portfolio::new("Main", "EUR").with_accounts([
        ib_cash.id.clone(),
        tr_cash.id.clone(),
        ib.id.clone(),
        tr.id.clone(),
    ]);
    store.save_portfolio(&portfolio)?;

    for tx in [
        Transaction::cash(
            &ib_cash.id,
            TransactionKind::Deposit,
            day(6, 1),
            dec!(2000),
            "USD",
        )
        .with_fx_rate(dec!(0.92))
        .with_note("account top-up"),
        Transaction::buy(&ib.id, &apple.id, day(6, 5), dec!(5), dec!(195), "USD")
            .with_fees(dec!(1))
            .with_fx_rate(dec!(0.92)),
        Transaction::buy(&ib.id, &apple.id, day(8, 12), dec!(5), dec!(216), "USD")
            .with_fees(dec!(1))
            .with_fx_rate(dec!(0.91)),
        Transaction::dividend(&ib.id, &apple.id, day(8, 15), dec!(12.50), "USD")
            .with_taxes(dec!(1.88))
            .with_fx_rate(dec!(0.91)),
        Transaction::sell(&ib.id, &apple.id, day(11, 4), dec!(4), dec!(222), "USD")
            .with_fees(dec!(1))
            .with_fx_rate(dec!(0.92)),
        Transaction::cash(
            &tr_cash.id,
            TransactionKind::Deposit,
            day(6, 1),
            dec!(3000),
            "EUR",
        ),
        Transaction::buy(&tr.id, &world.id, day(6, 10), dec!(30), dec!(85), "EUR").with_fees(dec!(1)),
    ] {
        store.save_transaction(&tx)?;
    }

    // Seed market data manually so the demo does not require a network.
    let mut quotes = Vec::new();
    for (security, currency, series) in [
        (
            &apple,
            "USD",
            vec![
                (day(6, 5), dec!(195)),
                (day(8, 12), dec!(216)),
                (day(11, 4), dec!(222)),
                (day(12, 31), dec!(250)),
            ],
        ),
        (
            &world,
            "EUR",
            vec![
                (day(6, 10), dec!(85)),
                (day(8, 12), dec!(90)),
                (day(11, 4), dec!(95)),
                (day(12, 31), dec!(100)),
            ],
        ),
        (
            &benchmark,
            "USD",
            vec![(day(6, 1), dec!(5300)), (day(12, 31), dec!(5900))],
        ),
    ] {
        for (date, close) in series {
            quotes.push(Quote {
                security_id: security.id.clone(),
                date,
                close,
                currency: currency.into(),
                source: "manual".into(),
            });
        }
    }
    store.save_quotes(&quotes)?;

    store.save_fx_rates(&[
        FxRate::new("USD", "EUR", day(6, 1), dec!(0.92)),
        FxRate::new("USD", "EUR", day(8, 12), dec!(0.91)),
        FxRate::new("USD", "EUR", day(11, 4), dec!(0.92)),
        FxRate::new("USD", "EUR", day(12, 31), dec!(0.95)),
    ])?;

    // The broad fund is split by regional weights; Apple is entirely US.
    let taxonomy = Taxonomy::new("Regions", TaxonomyKind::Region);
    store.save_taxonomy(&taxonomy)?;
    let us = TaxonomyNode::root(&taxonomy.id, "United States");
    let eu = TaxonomyNode::root(&taxonomy.id, "Europe").with_rank(1);
    store.save_taxonomy_node(&us)?;
    store.save_taxonomy_node(&eu)?;
    for c in [
        SecurityClassification::new(&apple.id, &us.id, Decimal::ONE),
        SecurityClassification::new(&world.id, &us.id, dec!(0.6)),
        SecurityClassification::new(&world.id, &eu.id, dec!(0.4)),
    ] {
        store.save_classification(&c)?;
    }

    let target = AllocationTarget::new(&portfolio.id, &taxonomy.id, "60/40")
        .with_weight(&us.id, dec!(0.6))
        .with_weight(&eu.id, dec!(0.4));
    store.save_target(&target)?;

    Ok(DemoWorld {
        store,
        portfolio,
        accounts: vec![ib, tr],
        apple,
        world,
        benchmark,
        taxonomy_id: taxonomy.id,
        target,
    })
}

/// Run the end-to-end valuation and return metrics demo.
pub fn run() -> Result<()> {
    let world = demo_world()?;
    let analytics = world.analytics()?;
    let as_of = day(12, 31);
    let v = analytics.valuation_at(as_of)?;

    println!(
        "Portfolio \"{}\" on {}, base {}",
        world.portfolio.name, v.date, v.base_currency
    );
    println!("{}", "-".repeat(78));
    println!(
        "{:<10} {:>10} {:>10} {:>8} {:>14} {:>14}",
        "instrument", "qty", "price", "fx", "value", "unreal."
    );
    for p in &v.positions {
        let security = world.store.get_security(&p.security_id)?;
        println!(
            "{:<10} {:>10} {:>10} {:>8} {:>14} {:>14}",
            security.symbol,
            p.quantity,
            money(p.price),
            p.fx_rate,
            money(p.market_value_base),
            money(p.unrealized_pnl_base),
        );
    }
    println!("{}", "-".repeat(78));
    println!(
        "securities        {:>14} {}",
        money(v.securities_value_base),
        v.base_currency
    );
    println!("cash              {:>14} {}", money(v.cash_base), v.base_currency);
    println!(
        "total             {:>14} {}",
        money(v.total_value_base),
        v.base_currency
    );
    println!();
    println!("cost basis        {:>14}", money(v.cost_basis_base));
    println!("unrealized        {:>14}", money(v.unrealized_pnl_base));
    println!("realized          {:>14}", money(v.realized_pnl_base));
    println!("dividends         {:>14}", money(v.dividends_base));
    println!("total P/L         {:>14}", money(v.total_pnl_base()));
    println!();

    let from = day(6, 1);
    println!(
        "TWR  (portfolio return)      {}",
        percent(analytics.twr(from, as_of)?)
    );
    println!("XIRR (investor return)       {}", percent(analytics.xirr(as_of)?));
    println!(
        "TWR  of AAPL                 {}",
        percent(analytics.position_twr(&world.apple.id, from, as_of)?)
    );
    println!(
        "IRR  of IWDA                 {}",
        percent(analytics.position_xirr(&world.world.id, as_of)?)
    );

    println!();
    for (year, gains) in analytics.capital_gains(as_of)? {
        println!(
            "realized in {year}: {} (proceeds {}, cost basis {}, fees {})",
            money(gains.gain_base),
            money(gains.proceeds_base),
            money(gains.cost_base),
            money(gains.fees_base),
        );
    }
    for (year, dividends) in analytics.dividends(as_of)? {
        println!(
            "dividends in {year}: {} net (accrued {}, tax {})",
            money(dividends.net_base),
            money(dividends.gross_base),
            money(dividends.taxes_base),
        );
    }

    // Demonstrate as-of lookup returning the last known price on a closed day.
    let ny = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    println!();
    println!(
        "AAPL price on {} (market closed) = {} — falling back to the last known",
        ny,
        world
            .store
            .price_as_of(&world.apple.id, ny)?
            .map(|p| format!("{} {}", money(p.close), p.currency))
            .unwrap_or_else(|| "none".into())
    );

    Ok(())
}
