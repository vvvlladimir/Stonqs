//! Dividing one node's drift across the subjects filed under it.
//!
//! A security becomes a trade floored to its observed step; a cash subject becomes a deposit,
//! because a balance is paid in rather than bought. A subject that cannot afford its first step
//! is still kept as a candidate, so leftover cash has somewhere to go.

use super::{CashDeposit, Prices, RebalanceTrade};
use crate::calc::Assignment;
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
    let Prices {
        securities,
        unit_price,
        value_of,
        step: observed_step,
        cash: cash_subjects,
    } = prices;
    if drift.is_zero() {
        return NodeMoves::default();
    }
    // Subject weight in the node; BTreeMap keeps report order stable.
    let mut weight_in_node: BTreeMap<&str, Decimal> = BTreeMap::new();
    for a in assignments {
        if !subtree.contains(&a.node_id) {
            continue;
        }
        // Securities without a price cannot produce a trade; cash uses its own path.
        let known =
            unit_price.contains_key(a.subject_id.as_str()) && securities.contains_key(a.subject_id.as_str());
        if !known && !cash_subjects.contains_key(a.subject_id.as_str()) {
            continue;
        }
        let key = securities
            .get_key_value(a.subject_id.as_str())
            .map(|(id, _)| *id)
            .or_else(|| {
                cash_subjects
                    .get_key_value(a.subject_id.as_str())
                    .map(|(id, _)| *id)
            })
            .expect("the key was checked above");
        *weight_in_node.entry(key).or_default() += a.weight;
    }
    if weight_in_node.is_empty() {
        return NodeMoves::default();
    }

    // Split drift by current contribution, or evenly when the node is empty.
    let contributions: BTreeMap<&str, Decimal> = weight_in_node
        .iter()
        .map(|(id, w)| {
            let value = value_of
                .get(*id)
                .copied()
                .or_else(|| cash_subjects.get(*id).map(|s| s.value_base))
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
        if let Some(subject) = cash_subjects.get(*id) {
            // Parse cash keys here so the UI need not know their internal format.
            let (account_id, currency) =
                parse_cash_subject(&subject.key).unwrap_or((subject.key.as_str(), ""));
            moves.deposits.push(CashDeposit {
                subject_id: subject.key.clone(),
                account_id: account_id.to_string(),
                currency: currency.to_string(),
                label: subject.name.clone(),
                current_base: subject.value_base,
                amount_base: share,
                // Final account weight is filled after all deposits are aggregated.
                weight_after: Decimal::ZERO,
            });
            continue;
        }
        let price = unit_price[*id];
        if price.is_zero() {
            continue;
        }
        moves.tradable_share += share.max(Decimal::ZERO);
        let security = securities[*id];
        let step = security.quantity_step_for(observed_step.get(*id).copied().flatten());
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
            continue;
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
    moves
}
