//! One node's drift across its subjects: a security becomes a trade floored to its step, cash
//! becomes a deposit. A subject too poor for one step stays a candidate for leftover cash.

use super::{CashDeposit, Prices, RebalanceTrade};
use crate::calc::{Assignment, TaxonomySubject};
use crate::model::{floor_to_step, parse_cash_subject};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Security candidate that can receive one more trading step.
pub(super) struct Candidate {
    pub security_id: String,
    pub symbol: String,
    /// Base-currency unit price.
    pub price: Decimal,
    /// Quantity bought per trading step.
    pub step: Decimal,
    /// Proportional amount assigned to the security.
    pub wanted: Decimal,
    /// Amount already planned.
    pub spent: Decimal,
}

/// Node budget for purchases.
pub(super) struct NodeBudget {
    /// Plan row receiving trades.
    pub item: usize,
    /// Purchase budget after scaling.
    pub budget: Decimal,
    /// Full node shortfall, used to prioritize top-ups.
    pub shortfall: Decimal,
    pub spent: Decimal,
    pub candidates: Vec<Candidate>,
}

/// Planned trades, deposits, and top-up candidates for a node.
#[derive(Default)]
pub(super) struct NodeMoves {
    pub trades: Vec<RebalanceTrade>,
    pub deposits: Vec<CashDeposit>,
    pub candidates: Vec<Candidate>,
    /// Node share assigned to tradable subjects.
    pub tradable_share: Decimal,
}

pub(super) fn plan_moves(
    subtree: &[String],
    drift: Decimal,
    assignments: &[Assignment],
    prices: &Prices<'_>,
) -> NodeMoves {
    if drift.is_zero() {
        return NodeMoves::default();
    }
    let weight_in_node = weights_in_node(subtree, assignments, prices);
    if weight_in_node.is_empty() {
        return NodeMoves::default();
    }

    // Split drift by current contribution, or evenly when the node is empty.
    let contributions: BTreeMap<&str, Decimal> = weight_in_node
        .iter()
        .map(|(id, w)| {
            let value = prices
                .value_of
                .get(*id)
                .copied()
                .or_else(|| prices.cash.get(*id).map(|s| s.value_base))
                .unwrap_or(Decimal::ZERO);
            (*id, value * *w)
        })
        .collect();
    let contributed: Decimal = contributions.values().sum();
    let count = Decimal::from(contributions.len());

    let mut moves = NodeMoves::default();
    for (id, contribution) in &contributions {
        let share = if contributed.is_zero() {
            drift / count
        } else {
            drift * (*contribution / contributed)
        };
        if let Some(subject) = prices.cash.get(*id) {
            moves.deposits.push(cash_deposit(subject, share));
        } else {
            security_move(&mut moves, id, share, prices);
        }
    }
    moves
}

/// Each priced or cash subject's weight in the node; a BTreeMap keeps report order stable.
fn weights_in_node<'a>(
    subtree: &[String],
    assignments: &[Assignment],
    prices: &Prices<'a>,
) -> BTreeMap<&'a str, Decimal> {
    let Prices {
        securities,
        unit_price,
        cash,
        ..
    } = prices;
    let mut weights: BTreeMap<&str, Decimal> = BTreeMap::new();
    for a in assignments {
        if !subtree.contains(&a.node_id) {
            continue;
        }
        let id = a.subject_id.as_str();
        // Securities without a price cannot produce a trade; cash uses its own path.
        let key = if unit_price.contains_key(id) {
            securities.get_key_value(id).map(|(key, _)| *key)
        } else {
            None
        }
        .or_else(|| cash.get_key_value(id).map(|(key, _)| *key));
        if let Some(key) = key {
            *weights.entry(key).or_default() += a.weight;
        }
    }
    weights
}

fn cash_deposit(subject: &TaxonomySubject, share: Decimal) -> CashDeposit {
    // Parse cash keys here so the UI need not know their internal format.
    let (account_id, currency) = parse_cash_subject(&subject.key).unwrap_or((subject.key.as_str(), ""));
    CashDeposit {
        subject_id: subject.key.clone(),
        account_id: account_id.to_string(),
        currency: currency.to_string(),
        label: subject.name.clone(),
        current_base: subject.value_base,
        amount_base: share,
        // Final account weight is filled after all deposits are aggregated.
        weight_after: Decimal::ZERO,
    }
}

/// A security's share of the drift, floored to its trading step.
fn security_move(moves: &mut NodeMoves, id: &str, share: Decimal, prices: &Prices<'_>) {
    let price = prices.unit_price[id];
    if price.is_zero() {
        return;
    }
    moves.tradable_share += share.max(Decimal::ZERO);
    let security = prices.securities[id];
    let step = security.quantity_step_for(prices.step.get(id).copied().flatten());
    let rounded = floor_to_step((share / price).abs(), step);
    let quantity = if share.is_sign_negative() {
        -rounded
    } else {
        rounded
    };
    let estimated = quantity * price;
    // Keep candidates that could not afford their first step for leftover allocation.
    if share.is_sign_positive() {
        moves.candidates.push(Candidate {
            security_id: security.id.clone(),
            symbol: security.symbol.clone(),
            price,
            step,
            wanted: share,
            spent: estimated,
        });
    }
    if rounded.is_zero() {
        return;
    }
    moves.trades.push(RebalanceTrade {
        security_id: security.id.clone(),
        symbol: security.symbol.clone(),
        quantity,
        estimated_base: estimated,
        price_base: price,
        // Final security weight is filled after all node trades are aggregated.
        weight_after: Decimal::ZERO,
    });
}
