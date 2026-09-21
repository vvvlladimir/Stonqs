//! The readings: value series, risk, allocation, benchmark and rebalance over the demo world.

use crate::args::parse_range;
use crate::demo::{day, demo_world};
use crate::fmt::{money, percent, percent_f64, print_buckets};
use rust_decimal::Decimal;
use sq_core::prelude::*;
use std::str::FromStr;

/// Print the daily valuation series as monthly summaries.
pub fn series(args: &[String]) -> Result<()> {
    let world = demo_world()?;
    let range = parse_range(args, world.default_range())?;
    let series = world.analytics()?.series(range)?;

    println!(
        "days in the series: {}, base {}",
        series.len(),
        series.base_currency
    );
    println!("{}", "-".repeat(58));
    println!("{:<12} {:>16} {:>16}", "month end", "value", "flows in the month");
    for month in Period::Month.split(range) {
        let Some(value) = series.value_on(month.to) else {
            continue;
        };
        let flow: Decimal = series
            .dates
            .iter()
            .zip(&series.external_flow_base)
            .filter(|(date, _)| **date >= month.from && **date <= month.to)
            .map(|(_, flow)| *flow)
            .sum();
        println!("{:<12} {:>16} {:>16}", month.to, money(value), money(flow));
    }
    println!("{}", "-".repeat(58));
    println!("TWR over the period: {}", percent(series.twr()?));
    Ok(())
}

/// Print risk metrics for a date range.
pub fn risk(args: &[String]) -> Result<()> {
    let world = demo_world()?;
    let range = parse_range(args, world.default_range())?;
    // The demo has no user context, so its risk-free rate is a fixed example.
    let risk_free = 0.03;
    let m = world.analytics()?.risk(range, risk_free)?;

    println!("period {} — {}, business days {}", range.from, range.to, m.days);
    println!("{}", "-".repeat(58));
    println!("annualized return       {:>10}", percent_f64(m.annualized_return));
    println!("volatility (annual)     {:>10}", percent_f64(m.volatility));
    println!("semi-deviation (annual) {:>10}", percent_f64(m.semi_deviation));
    match m.sharpe {
        Some(s) => println!("Sharpe (rf {:.0}%)          {:>10.2}", risk_free * 100.0, s),
        None => println!("Sharpe                       none (zero volatility)"),
    }
    println!(
        "winning days            {:>10}",
        percent_f64(m.positive_days_share)
    );
    if let Some((date, r)) = m.best_day {
        println!("best day      {date}  {:>10}", percent_f64(r));
    }
    if let Some((date, r)) = m.worst_day {
        println!("worst day     {date}  {:>10}", percent_f64(r));
    }
    match m.max_drawdown {
        Some(dd) => println!(
            "drawdown {:>10}: peak {}, trough {}, recovery {}",
            percent_f64(dd.depth),
            dd.peak,
            dd.trough,
            dd.recovered
                .map(|d| d.to_string())
                .unwrap_or_else(|| "not yet".into()),
        ),
        None => println!("no drawdowns"),
    }
    Ok(())
}

/// Print the current valuation grouped by taxonomy, currency, account, or security.
pub fn allocation(args: &[String]) -> Result<()> {
    let world = demo_world()?;
    let analytics = world.analytics()?;
    let date = day(12, 31);
    let cut = args.first().map(String::as_str).unwrap_or("taxonomy");

    let allocation = match cut {
        "taxonomy" => analytics.allocation_by_taxonomy(&world.taxonomy_id, date)?,
        "currency" => analytics.allocation_by_currency(date)?,
        "account" => analytics.allocation_by_account(date)?,
        "security" => analytics.allocation_by_security(date)?,
        other => {
            return Err(Error::Invalid(format!(
                "unknown allocation cut {other:?}: taxonomy|currency|account|security"
            )));
        }
    };

    println!(
        "breakdown \"{cut}\" on {date}, total {}",
        money(allocation.total_base)
    );
    println!("{}", "-".repeat(58));
    print_buckets(&allocation.buckets, 0);
    println!("{}", "-".repeat(58));
    println!("shares sum to {}", percent(allocation.total_weight()));
    Ok(())
}

/// Compare the portfolio with a benchmark security.
pub fn benchmark(args: &[String]) -> Result<()> {
    let world = demo_world()?;
    let range = parse_range(args, world.default_range())?;
    let c = world
        .analytics()?
        .benchmark(&world.benchmark.id, range.from, range.to)?;

    println!("period {} — {}", c.from, c.to);
    println!("portfolio {:>10}", percent(c.portfolio_twr));
    println!("{:<9} {:>10}", world.benchmark.symbol, percent(c.benchmark_twr));
    println!(
        "delta     {:>10}  ({})",
        percent(c.excess),
        if c.excess.is_sign_negative() {
            "the benchmark is ahead"
        } else {
            "the portfolio is ahead"
        }
    );
    println!();
    println!("the benchmark's dividends are not in its return: the provider gives prices, not total return");
    Ok(())
}

/// Print a rebalancing plan, optionally using new cash and buy-only mode.
pub fn rebalance(args: &[String]) -> Result<()> {
    let world = demo_world()?;
    let date = day(12, 31);
    let options = RebalanceOptions {
        cash_to_invest: args
            .iter()
            .find_map(|a| Decimal::from_str(a).ok())
            .unwrap_or(Decimal::ZERO),
        allow_sell: !args.iter().any(|a| a == "--buy-only"),
    };
    let plan = world.analytics()?.rebalance(&world.target, date, options)?;

    println!(
        "target \"{}\" on {date}, portfolio {}",
        world.target.name,
        money(plan.total_base)
    );
    if !options.cash_to_invest.is_zero() {
        println!(
            "new money {}, of which allocated {}",
            money(options.cash_to_invest),
            money(plan.cash_used_base)
        );
    }
    println!("{}", "-".repeat(78));
    println!(
        "{:<12} {:>12} {:>8} {:>8} {:>12}",
        "node", "now", "weight", "target", "drift"
    );
    for item in &plan.items {
        println!(
            "{:<12} {:>12} {:>8} {:>8} {:>12}",
            item.label,
            money(item.current_base),
            percent(item.current_weight),
            percent(item.target_weight),
            money(item.drift_base),
        );
        for trade in &item.trades {
            let verb = if trade.quantity.is_sign_negative() {
                "sell"
            } else {
                "buy"
            };
            println!(
                "    {verb} {} × {} ≈ {}",
                trade.quantity.abs(),
                trade.symbol,
                money(trade.estimated_base.abs())
            );
        }
    }
    println!("{}", "-".repeat(78));
    println!(
        "off target (cash and unclassified): {}",
        money(plan.off_target_base)
    );
    Ok(())
}
