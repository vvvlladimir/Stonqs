//! Applying events one at a time, so a daily series stays linear.

use super::{Holdings, HoldingsOptions, paired_links};
use crate::error::{Error, Result};
use crate::fx::RateLookup;
use crate::model::{CorporateAction, Lot, Transaction, TransactionKind};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use std::collections::{HashMap, HashSet};

/// Incremental holdings builder; applies events once so daily valuation stays linear.
pub(crate) struct HoldingsBuilder<'a> {
    pub(super) holdings: Holdings,
    pub(super) base: Currency,
    pub(super) rates: &'a dyn RateLookup,
    pub(super) options: HoldingsOptions<'a>,
    /// Lots removed by a transfer-out and waiting for the linked transfer-in.
    pub(super) in_transit: HashMap<String, Vec<Lot>>,
    /// Links with two legs present, resolved once over the whole set — a single event cannot
    /// see whether its counterpart exists. See [`paired_links`].
    pub(super) paired: HashSet<String>,
}

impl<'a> HoldingsBuilder<'a> {
    /// The transaction set is taken whole because a `link_id` only means something against it.
    pub(crate) fn new(
        transactions: &[Transaction],
        base: &str,
        rates: &'a dyn RateLookup,
        options: HoldingsOptions<'a>,
    ) -> Self {
        HoldingsBuilder {
            holdings: Holdings::default(),
            base: normalize_currency(base),
            rates,
            options,
            in_transit: HashMap::new(),
            paired: paired_links(transactions).into_iter().map(String::from).collect(),
        }
    }

    pub(crate) fn apply(&mut self, event: Event<'_>) -> Result<()> {
        match event {
            Event::Action(action) => super::apply::corporate_action(&mut self.holdings, action),
            Event::Tx(t) => self.transaction(t),
        }
    }

    /// Holdings snapshot at the current event.
    pub(crate) fn holdings(&self) -> &Holdings {
        &self.holdings
    }

    pub(crate) fn finish(self) -> Result<Holdings> {
        if let Some(link) = self.in_transit.keys().next() {
            return Err(Error::Invalid(format!(
                "security transfer {link} has an outgoing side but no incoming one"
            )));
        }
        Ok(self.holdings)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Event<'a> {
    Action(&'a CorporateAction),
    Tx(&'a Transaction),
}

impl Event<'_> {
    pub(crate) fn date(&self) -> NaiveDate {
        match self {
            Event::Action(a) => a.date,
            Event::Tx(t) => t.date,
        }
    }
}

/// Chronological; within a day: action, acquisition, transfer-out, transfer-in, the rest.
/// Shares arrive before they can leave, since storage order within a day is random (ADR-0089).
pub(crate) fn ordered_events<'a>(
    transactions: &'a [Transaction],
    actions: &'a [CorporateAction],
) -> Vec<Event<'a>> {
    fn phase(kind: TransactionKind) -> u8 {
        match kind {
            TransactionKind::Buy | TransactionKind::DeliveryInbound => 1,
            TransactionKind::SecurityTransferOut => 2,
            TransactionKind::SecurityTransferIn => 3,
            _ => 4,
        }
    }

    let mut events: Vec<(NaiveDate, u8, usize, Event<'a>)> =
        Vec::with_capacity(transactions.len() + actions.len());
    for (i, a) in actions.iter().enumerate() {
        events.push((a.date, 0, i, Event::Action(a)));
    }
    for (i, t) in transactions.iter().enumerate() {
        events.push((t.date, phase(t.kind), i, Event::Tx(t)));
    }
    // Original index preserves storage order within the same date and phase.
    events.sort_by_key(|(date, phase, index, _)| (*date, *phase, *index));
    events.into_iter().map(|(_, _, _, e)| e).collect()
}
