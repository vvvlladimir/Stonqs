//! A trade is a position's life: it opens with a purchase, grows with later ones, and one
//! disposal ends it. The numbers come from the lots a disposal consumed — see ADR-0027.

use super::{CashFlow, Holdings, PortfolioValuation, RealizedGain, xirr};
use crate::error::Result;
use crate::fx::RateLookup;
use crate::model::{Lot, Transaction, TransactionKind};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};

/// One trade: the purchases behind a quantity and what leaving it brought in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trade {
    pub security_id: String,
    /// Purchase date of the oldest lot in the trade.
    pub opened_at: NaiveDate,
    /// The disposal that ended it; `None` while the shares are still held.
    pub closed_at: Option<NaiveDate>,
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    /// What entering cost, purchase commissions and taxes included.
    #[serde(with = "rust_decimal::serde::str")]
    pub entry_value_base: Decimal,
    /// Sale proceeds net of its costs, or — while open — today's market value.
    #[serde(with = "rust_decimal::serde::str")]
    pub exit_value_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub pnl_base: Decimal,
    /// Days held, weighted by quantity: two purchases have two ages and one trade.
    pub holding_days: i64,
    /// `exit / entry - 1`; `None` when there was no cost to divide by.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub return_pct: Option<Decimal>,
    /// The same result as a yearly rate, from each lot's own date; `None` when unsolvable.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub irr: Option<Decimal>,
}

impl Trade {
    pub fn is_open(&self) -> bool {
        self.closed_at.is_none()
    }
}

/// Builds one trade from the lots behind it and the value of leaving on `exit_date`.
/// `closed` separates a sale that happened from a position marked to market today.
fn trade(
    security_id: &str,
    lots: &[Lot],
    exit_value_base: Decimal,
    exit_date: NaiveDate,
    closed: bool,
) -> Option<Trade> {
    let quantity: Decimal = lots.iter().map(|l| l.quantity).sum();
    if quantity.is_zero() {
        return None;
    }
    let entry_value_base: Decimal = lots.iter().map(|l| l.quantity * l.cost_per_unit_base).sum();
    let opened_at = lots.iter().map(|l| l.acquired_at).min()?;

    // Each lot contributes its own age in proportion to the shares it holds.
    let weighted: Decimal = lots
        .iter()
        .map(|l| l.quantity * Decimal::from((exit_date - l.acquired_at).num_days()))
        .sum();
    let holding_days = (weighted / quantity).round().to_i64().unwrap_or_default();

    let mut flows: Vec<CashFlow> = lots
        .iter()
        .map(|l| CashFlow {
            date: l.acquired_at,
            amount_base: -(l.quantity * l.cost_per_unit_base),
        })
        .collect();
    flows.push(CashFlow {
        date: exit_date,
        amount_base: exit_value_base,
    });

    Some(Trade {
        security_id: security_id.to_string(),
        opened_at,
        closed_at: closed.then_some(exit_date),
        quantity,
        entry_value_base,
        exit_value_base,
        pnl_base: exit_value_base - entry_value_base,
        holding_days,
        return_pct: (!entry_value_base.is_zero()).then(|| exit_value_base / entry_value_base - Decimal::ONE),
        // An unsolvable rate is an empty cell, not a failure: a same-day trade has no root.
        irr: xirr(&flows).ok(),
    })
}

/// One trade per disposal, in disposal order. An outbound delivery ends a trade too — it
/// realizes a result, and dropping it would lose everything the shares earned before leaving.
pub fn closed_trades(realized: &[RealizedGain]) -> Vec<Trade> {
    realized
        .iter()
        .filter_map(|g| trade(&g.security_id, &g.lots, g.net_proceeds_base(), g.date, true))
        .collect()
}

/// One trade per open position, marked to market at the valuation date. Under average cost
/// a position has a single merged lot, so its trade opens at the earliest purchase.
pub fn open_trades(holdings: &Holdings, valuation: &PortfolioValuation) -> Vec<Trade> {
    valuation
        .positions
        .iter()
        .filter_map(|p| {
            let position = holdings.positions.get(&p.security_id)?;
            trade(
                &p.security_id,
                &position.lots,
                p.market_value_base,
                valuation.date,
                false,
            )
        })
        .collect()
}

/// Both sides of the trade ledger from one holdings pass: what is still held and what was closed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeBook {
    pub open: Vec<Trade>,
    pub closed: Vec<Trade>,
}

/// A group of trades as the dashboard counts them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradeStats {
    pub trades: usize,
    pub winners: usize,
    pub losers: usize,
    #[serde(with = "rust_decimal::serde::str")]
    pub pnl_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub entry_value_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub exit_value_base: Decimal,
    /// Holding period weighted by what each trade had invested — an afternoon in a tiny
    /// position should not shorten the average as much as a decade in the largest one.
    pub average_holding_days: i64,
    /// Winners over trades; `None` when there are none to divide.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub win_rate: Option<Decimal>,
}

pub fn trade_stats(trades: &[Trade]) -> TradeStats {
    let mut stats = TradeStats {
        trades: trades.len(),
        ..TradeStats::default()
    };
    let mut weighted = Decimal::ZERO;
    for t in trades {
        if t.pnl_base.is_sign_negative() {
            stats.losers += 1;
        } else {
            stats.winners += 1;
        }
        stats.pnl_base += t.pnl_base;
        stats.entry_value_base += t.entry_value_base;
        stats.exit_value_base += t.exit_value_base;
        weighted += t.entry_value_base * Decimal::from(t.holding_days);
    }
    if stats.trades > 0 {
        stats.win_rate = Some(Decimal::from(stats.winners) / Decimal::from(stats.trades));
        stats.average_holding_days = if stats.entry_value_base.is_zero() {
            // Nothing to weight by: fall back to a plain mean rather than reporting no age.
            (trades.iter().map(|t| t.holding_days).sum::<i64>()) / stats.trades as i64
        } else {
            (weighted / stats.entry_value_base)
                .round()
                .to_i64()
                .unwrap_or_default()
        };
    }
    stats
}

/// What the portfolio's own trading moved over a period, the numerator of a turnover rate.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TradingVolume {
    #[serde(with = "rust_decimal::serde::str")]
    pub bought_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub sold_base: Decimal,
    /// Purchases plus sales: every euro that changed shape.
    #[serde(with = "rust_decimal::serde::str")]
    pub volume_base: Decimal,
    pub trades: usize,
}

/// Traded volume in `[from, to]`. A genuine delivery is excluded on purpose: shares arriving from
/// another broker cost no commission and are not a decision this portfolio made. A trade a scope
/// turned into one is not that — the depot bought, and only the cash leg fell outside the lens.
pub fn trading_volume(
    transactions: &[Transaction],
    base: &str,
    from: NaiveDate,
    to: NaiveDate,
    rates: &dyn RateLookup,
) -> Result<TradingVolume> {
    let mut out = TradingVolume::default();
    for t in transactions {
        if t.date < from || t.date > to {
            continue;
        }
        let traded = t.scoped_from.unwrap_or(t.kind);
        if !matches!(traded, TransactionKind::Buy | TransactionKind::Sell) {
            continue;
        }
        // A rewritten buy is priced by the same formula as the buy it was, so the gross is read
        // off the row as it stands and only the direction comes from the original kind.
        let amount = t.gross_in_transaction_currency() * super::resolve_rate(t, base, rates)?;
        match traded {
            TransactionKind::Buy => out.bought_base += amount,
            _ => out.sold_base += amount,
        }
        out.trades += 1;
    }
    out.volume_base = out.bought_base + out.sold_base;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn lot(acquired: NaiveDate, quantity: Decimal, cost: Decimal) -> Lot {
        Lot {
            acquired_at: acquired,
            quantity,
            cost_per_unit: cost,
            cost_per_unit_base: cost,
        }
    }

    /// 10 shares at 100 bought 2024-01-01, 10 at 120 bought 2024-07-01, all sold
    /// 2025-01-01 for 2500 net. Entry 10*100 + 10*120 = 2200, result 300.
    /// Ages are 366 and 184 days, so the weighted period is
    /// (10*366 + 10*184) / 20 = 275 days.
    #[test]
    fn two_purchases_make_one_trade_with_a_weighted_age() {
        let lots = [
            lot(day(2024, 1, 1), dec!(10), dec!(100)),
            lot(day(2024, 7, 1), dec!(10), dec!(120)),
        ];
        let t = trade("VWCE", &lots, dec!(2500), day(2025, 1, 1), true).unwrap();

        assert_eq!(t.quantity, dec!(20));
        assert_eq!(t.entry_value_base, dec!(2200));
        assert_eq!(t.pnl_base, dec!(300));
        assert_eq!(t.holding_days, 275);
        assert_eq!(t.opened_at, day(2024, 1, 1));
        assert_eq!(t.closed_at, Some(day(2025, 1, 1)));
    }

    /// 2200 in and 2500 out is 2500/2200 - 1 = 0.13636...
    #[test]
    fn the_return_is_what_came_out_over_what_went_in() {
        let lots = [lot(day(2024, 1, 1), dec!(20), dec!(110))];
        let t = trade("VWCE", &lots, dec!(2500), day(2025, 1, 1), true).unwrap();

        assert_eq!(t.return_pct.unwrap().round_dp(5), dec!(0.13636));
    }

    /// A position still held has no closing date, and its exit value is the market value.
    #[test]
    fn an_open_trade_is_marked_to_market() {
        let lots = [lot(day(2024, 1, 1), dec!(5), dec!(100))];
        let t = trade("VWCE", &lots, dec!(700), day(2024, 6, 1), false).unwrap();

        assert!(t.is_open());
        assert_eq!(t.pnl_base, dec!(200));
        assert_eq!(t.holding_days, 152);
    }

    /// 1000 held 100 days and 100 held 900 days: the average leans to the money,
    /// (1000*100 + 100*900) / 1100 = 190000/1100 = 172.7 -> 173 days.
    #[test]
    fn the_average_age_follows_the_money_not_the_count() {
        let trades = vec![
            trade(
                "A",
                &[lot(day(2024, 1, 1), dec!(10), dec!(100))],
                dec!(1100),
                day(2024, 4, 10),
                true,
            )
            .unwrap(),
            trade(
                "B",
                &[lot(day(2022, 1, 1), dec!(1), dec!(100))],
                dec!(90),
                day(2024, 6, 19),
                true,
            )
            .unwrap(),
        ];
        let stats = trade_stats(&trades);

        assert_eq!(stats.trades, 2);
        assert_eq!(stats.winners, 1);
        assert_eq!(stats.losers, 1);
        assert_eq!(stats.win_rate, Some(dec!(0.5)));
        assert_eq!(stats.average_holding_days, 173);
    }
}
