//! An instrument's series: its own source first, others only for a gap it failed (ADR-0052), and
//! its latest-close source for the unpublished tail (ADR-0079).

use super::{MarketDataService, OVERLAP_DAYS, RESTS_AFTER};
use crate::error::{Error, Result};
use crate::market::DateRange;
use crate::model::Security;
use crate::storage::Store;
use chrono::Duration;

impl MarketDataService {
    /// One call to a source through the retry policy and its breaker: a rejected key rests the
    /// source at once, a transient failure counts towards `RESTS_AFTER`, a success clears both.
    pub(super) fn attempt<T>(
        &self,
        store: &Store,
        id: &'static str,
        call: impl FnMut() -> Result<T>,
    ) -> Result<T> {
        if self
            .failures
            .lock()
            .map(|f| f.get(id).copied().unwrap_or(0))
            .unwrap_or(0)
            >= RESTS_AFTER
        {
            return Err(Error::Unavailable(format!(
                "{id} is resting after repeated failures"
            )));
        }
        self.budgets.spend(store, id)?;
        let out = self.policy.run(call);
        if let Ok(mut failures) = self.failures.lock() {
            match &out {
                Ok(_) => {
                    failures.remove(id);
                }
                Err(Error::Unauthorized(_)) => {
                    failures.insert(id, RESTS_AFTER);
                }
                Err(e) if e.is_transient() => *failures.entry(id).or_default() += 1,
                Err(_) => {}
            }
        }
        out
    }

    /// One gap from another source that knows the instrument, if `guard::check` accepts it. Fills
    /// only empty days and extends no coverage, so the own source overwrites later.
    fn fill_from_fallbacks(
        &self,
        store: &Store,
        security: &Security,
        gap: DateRange,
    ) -> Result<Option<usize>> {
        let own = security.data_source.as_deref();
        let asked = DateRange::new(gap.from - Duration::days(OVERLAP_DAYS), gap.to);
        let currency = store
            .latest_quote_currency(&security.id)?
            .unwrap_or_else(|| security.currency.clone());
        for id in self.order.iter().copied().filter(|id| Some(*id) != own) {
            let provider = &self.providers[id];
            if !provider.covers(security) {
                continue;
            }
            let Some(symbol) = store.symbol_at(security, id)? else {
                continue;
            };
            let alias = security.clone().with_source(id, &symbol);
            let Ok(quotes) = self.attempt(store, id, || provider.fetch(&alias, asked)) else {
                continue;
            };
            let stored = store.quotes_in_range(&security.id, asked)?;
            if crate::market::guard::check(id, &currency, &stored, &quotes).is_err() {
                continue;
            }
            return Ok(Some(store.fill_quotes(&quotes)?));
        }
        Ok(None)
    }

    pub fn ensure_history(&self, store: &Store, security: &Security, range: DateRange) -> Result<usize> {
        self.ensure_history_through(store, security, range, range.to)
    }

    pub fn ensure_history_through(
        &self,
        store: &Store,
        security: &Security,
        range: DateRange,
        settled_through: chrono::NaiveDate,
    ) -> Result<usize> {
        let Some(source) = security.data_source.as_deref() else {
            return Ok(0);
        };
        if !security.is_quotable() {
            return Err(Error::Invalid(format!(
                "{}: the ticker field holds an ISIN, not a provider symbol — identify the instrument in the directory",
                security.provider_symbol()
            )));
        }
        let provider = self
            .providers
            .get(source)
            .ok_or_else(|| Error::NotFound(format!("quote provider {source}")))?;

        // Events ride on the quote request, so a range is covered only once both were asked for.
        let covered = overlap(
            store.quote_coverage(&security.id)?,
            store.event_coverage(&security.id)?,
        );
        let mut saved = 0;
        for gap in missing_ranges(covered, range) {
            let history = match self.attempt(store, provider.id(), || provider.fetch_history(security, gap)) {
                Ok(history) => history,
                Err(e) => match self.fill_from_fallbacks(store, security, gap)? {
                    Some(filled) => {
                        saved += filled;
                        continue;
                    }
                    None => return Err(e),
                },
            };
            saved += store.save_quotes(&history.quotes)?;
            store.save_provider_events(&history.events)?;
            // Coverage records the request, even when the source returned no rows.
            let covered_to = gap.to.min(settled_through);
            if covered_to >= gap.from {
                let asked = DateRange::new(gap.from, covered_to);
                store.extend_quote_coverage(&security.id, asked)?;
                store.extend_event_coverage(&security.id, asked)?;
            }
        }
        Ok(saved)
    }

    /// The days after the stored series ends, from the latest-close source (ADR-0079). Guarded
    /// like a fallback, fills only empty days and extends no coverage.
    pub fn ensure_latest(
        &self,
        store: &Store,
        security: &Security,
        through: chrono::NaiveDate,
    ) -> Result<usize> {
        let Some((source, symbol)) = store.latest_symbol(&security.id)? else {
            return Ok(0);
        };
        if security.data_source.as_deref() == Some(source.as_str()) {
            return Ok(0);
        }
        // Nothing to extend, and nothing to check a stranger's closes against.
        let Some(last) = store.latest_quote_date(&security.id)? else {
            return Ok(0);
        };
        if last >= through {
            return Ok(0);
        }
        // A source switched off is not registered: ask nothing rather than fail.
        let Some((&id, provider)) = self.providers.get_key_value(source.as_str()) else {
            return Ok(0);
        };
        if !provider.covers(security) {
            return Ok(0);
        }
        let asked = DateRange::new(last - Duration::days(OVERLAP_DAYS), through);
        let alias = security.clone().with_source(id, &symbol);
        let quotes = self.attempt(store, id, || provider.fetch(&alias, asked))?;
        let currency = store
            .latest_quote_currency(&security.id)?
            .unwrap_or_else(|| security.currency.clone());
        crate::market::guard::check(
            id,
            &currency,
            &store.quotes_in_range(&security.id, asked)?,
            &quotes,
        )?;
        let tail: Vec<_> = quotes.into_iter().filter(|q| q.date > last).collect();
        store.fill_quotes(&tail)
    }
}

pub(super) fn overlap(a: Option<DateRange>, b: Option<DateRange>) -> Option<DateRange> {
    let (a, b) = (a?, b?);
    let range = DateRange::new(a.from.max(b.from), a.to.min(b.to));
    (range.from <= range.to).then_some(range)
}

pub(super) fn missing_ranges(covered: Option<DateRange>, wanted: DateRange) -> Vec<DateRange> {
    let Some(covered) = covered else {
        return vec![wanted];
    };
    let mut gaps = Vec::new();
    if wanted.from < covered.from {
        gaps.push(DateRange::new(wanted.from, covered.from - Duration::days(1)));
    }
    if wanted.to > covered.to {
        gaps.push(DateRange::new(covered.to + Duration::days(1), wanted.to));
    }
    gaps
}
