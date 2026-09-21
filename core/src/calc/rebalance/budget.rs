//! Spending what the proportional pass left over.
//!
//! One tradable step at a time, largest remaining shortfall first, and the least-overweight node
//! once no shortfall remains. A node with nothing to buy is never touched — a met `Cash 10%`
//! target keeps its share — and nothing is ever sold to make room.

use super::moves::NodeBudget;
use super::{RebalanceItem, RebalanceTrade};
use rust_decimal::Decimal;

/// Uses leftover cash one trading step at a time, prioritizing the largest remaining shortfall.
/// Cash-only nodes are not touched; unspent amounts remain cash.
pub(super) fn spend_leftover(
    items: &mut [RebalanceItem],
    budgets: &mut [NodeBudget],
    cash_to_invest: Decimal,
) {
    let mut pool: Decimal = budgets
        .iter()
        .map(|b| (b.budget - b.spent).max(Decimal::ZERO))
        .sum();

    // Cap by funds actually available after planned purchases and sales.
    let mut available = cash_to_invest;
    for item in items.iter() {
        for trade in &item.trades {
            available -= trade.estimated_base;
        }
    }
    pool = pool.min(available);

    // Hard cap protects against invalid zero-step or zero-price data.
    for _ in 0..10_000 {
        if pool <= Decimal::ZERO {
            return;
        }
        // Prioritize largest remaining shortfall; overweights come last.
        let mut order: Vec<usize> = (0..budgets.len()).collect();
        order.sort_by(|a, b| {
            let left = budgets[*b].shortfall - budgets[*b].spent;
            let right = budgets[*a].shortfall - budgets[*a].spent;
            left.cmp(&right)
        });

        let mut chosen: Option<(usize, usize, Decimal)> = None;
        for node in order {
            let mut best: Option<(usize, Decimal)> = None;
            for (index, candidate) in budgets[node].candidates.iter().enumerate() {
                if candidate.step.is_zero() || !candidate.price.is_sign_positive() {
                    continue;
                }
                let cost = candidate.step * candidate.price;
                if cost > pool {
                    continue;
                }
                let underfunded = candidate.wanted - candidate.spent;
                match best {
                    Some((_, top)) if top >= underfunded => {}
                    _ => best = Some((index, underfunded)),
                }
            }
            if let Some((index, _)) = best {
                let cost = budgets[node].candidates[index].step * budgets[node].candidates[index].price;
                chosen = Some((node, index, cost));
                break;
            }
        }

        let Some((node, index, cost)) = chosen else {
            return;
        };
        let budget = &mut budgets[node];
        let candidate = &mut budget.candidates[index];
        candidate.spent += cost;
        let step = candidate.step;
        let price = candidate.price;
        let security_id = candidate.security_id.clone();
        let symbol = candidate.symbol.clone();
        budget.spent += cost;
        pool -= cost;

        let trades = &mut items[budget.item].trades;
        match trades.iter_mut().find(|t| t.security_id == security_id) {
            Some(trade) => {
                trade.quantity += step;
                trade.estimated_base += cost;
            }
            None => trades.push(RebalanceTrade {
                security_id,
                symbol,
                quantity: step,
                estimated_base: cost,
                price_base: price,
                weight_after: Decimal::ZERO,
            }),
        }
    }
}
