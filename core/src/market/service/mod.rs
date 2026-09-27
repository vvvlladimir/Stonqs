use super::{
    DateRange, FetchPolicy, Listing, ListingDirectory, QuoteProvider, SecurityMatch, SecuritySearch, mic,
};
use crate::error::{Error, Result};
use crate::model::{Security, is_isin};
use crate::money::normalize_currency;
use crate::storage::Store;
use chrono::Duration;
use std::collections::HashMap;
use std::sync::Mutex;
const MAX_PROFILE_PROBES: usize = 5;
/// Consecutive failures after which a source is skipped for the rest of this service's life (one
/// refresh), so a source that is down costs three requests rather than one per instrument.
const RESTS_AFTER: u32 = 3;
/// How far before a gap a fallback is asked, so its closes overlap the stored series and can be
/// checked against it.
const OVERLAP_DAYS: i64 = 14;

fn rank(found: &SecurityMatch) -> u8 {
    u8::from(found.has_history == Some(true)) * 2 + u8::from(found.currency.is_some())
}

/// The ticker without the venue suffix a provider spells into its symbol.
fn base_ticker(symbol: &str) -> String {
    let symbol = symbol.trim().to_uppercase();
    match symbol.find('.') {
        Some(dot) => symbol[..dot].to_string(),
        None => symbol,
    }
}

fn looks_like_a_placeholder(symbol: &str, query: &str) -> u8 {
    let root = symbol.split('.').next().unwrap_or(symbol);
    u8::from(is_isin(root) || root.eq_ignore_ascii_case(query))
}

/// Coordinates quote, search, and listing sources with retry and persistence.
#[derive(Default)]
pub struct MarketDataService {
    providers: HashMap<&'static str, Box<dyn QuoteProvider>>,
    /// Registration order, which is the order fallbacks are tried in.
    order: Vec<&'static str>,
    failures: Mutex<HashMap<&'static str, u32>>,
    budgets: super::Budgets,
    searches: HashMap<&'static str, Box<dyn SecuritySearch>>,
    directories: HashMap<&'static str, Box<dyn ListingDirectory>>,
    policy: FetchPolicy,
}

impl MarketDataService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: FetchPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Caps a source at `per_day` requests, counted in the store the requests are saved to.
    pub fn set_budget(&mut self, source: &'static str, per_day: u32) {
        self.budgets.set(source, per_day);
    }

    pub fn register(&mut self, provider: Box<dyn QuoteProvider>) {
        if !self.order.contains(&provider.id()) {
            self.order.push(provider.id());
        }
        self.providers.insert(provider.id(), provider);
    }

    /// One call to a source through the retry policy and its breaker: a rejected key rests the
    /// source at once, a transient failure counts towards `RESTS_AFTER`, a success clears both.
    fn attempt<T>(&self, store: &Store, id: &'static str, call: impl FnMut() -> Result<T>) -> Result<T> {
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

    /// Asks the other sources that know this instrument for one gap its own source could not
    /// fill. What one returns joins the series only if `guard::check` accepts it, and only on
    /// days nothing is stored for; coverage is not extended, so the own source is asked again
    /// next time and overwrites. `None` when no fallback answered.
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
            if super::guard::check(id, &currency, &stored, &quotes).is_err() {
                continue;
            }
            return Ok(Some(store.fill_quotes(&quotes)?));
        }
        Ok(None)
    }

    pub fn with(mut self, provider: Box<dyn QuoteProvider>) -> Self {
        self.register(provider);
        self
    }

    pub fn provider(&self, id: &str) -> Option<&dyn QuoteProvider> {
        self.providers.get(id).map(|b| b.as_ref())
    }

    pub fn register_search(&mut self, search: Box<dyn SecuritySearch>) {
        self.searches.insert(search.id(), search);
    }

    pub fn with_search(mut self, search: Box<dyn SecuritySearch>) -> Self {
        self.register_search(search);
        self
    }

    pub fn register_directory(&mut self, directory: Box<dyn ListingDirectory>) {
        self.directories.insert(directory.id(), directory);
    }

    pub fn with_directory(mut self, directory: Box<dyn ListingDirectory>) -> Self {
        self.register_directory(directory);
        self
    }

    pub fn listings(&self, isin: &str) -> Result<Vec<Listing>> {
        let mut ids: Vec<&&'static str> = self.directories.keys().collect();
        ids.sort_unstable();

        let mut out: Vec<Listing> = Vec::new();
        let mut last_error = None;
        for id in ids {
            match self.policy.run(|| self.directories[*id].listings(isin)) {
                Ok(found) => out.extend(found),
                Err(e) => last_error = Some(e),
            }
        }
        if out.is_empty() {
            return match last_error {
                Some(e) => Err(e),
                None => Ok(Vec::new()),
            };
        }

        for listing in &mut out {
            listing.symbol = self
                .searches
                .values()
                .find_map(|s| s.symbol_for(&listing.ticker, &listing.mic));
        }
        out.retain(|l| l.symbol.is_some());
        out.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        out.dedup_by(|a, b| a.symbol == b.symbol);
        Ok(out)
    }

    /// Venues of an instrument with no ISIN. The directory cannot answer — it is keyed by ISIN —
    /// so the quote sources are asked for the bare ticker instead and every match they can place
    /// on a supported venue becomes a listing. `Listing::isin` is empty: there is none to record,
    /// which is also why the caller must not cache the result.
    pub fn listings_by_symbol(&self, symbol: &str) -> Result<Vec<Listing>> {
        let ticker = base_ticker(symbol);
        if ticker.is_empty() {
            return Ok(Vec::new());
        }
        let mut out: Vec<Listing> = Vec::new();
        for found in self.search(&ticker)? {
            // A search for "AWK" also returns unrelated instruments; only the same ticker on
            // another venue is the same instrument.
            let Some(mic) = found.mic.filter(|_| base_ticker(&found.symbol) == ticker) else {
                continue;
            };
            if out.iter().any(|l| l.mic == mic) {
                continue;
            }
            out.push(Listing {
                isin: String::new(),
                exchange: found
                    .exchange
                    .or_else(|| mic::market_name(&mic).map(str::to_string)),
                mic,
                ticker: ticker.clone(),
                name: Some(found.name),
                symbol: Some(found.symbol),
                currency: found.currency,
                has_history: found.has_history,
                last_close: found.last_close,
                source: found.source,
            });
        }
        out.sort_by(|a, b| a.symbol.cmp(&b.symbol));
        Ok(out)
    }

    pub fn probe_listings(&self, listings: Vec<Listing>, limit: usize) -> Vec<Listing> {
        let mut out = Vec::with_capacity(listings.len());
        let mut probed = 0usize;
        for mut listing in listings {
            let symbol = listing.symbol.clone();
            match symbol {
                Some(symbol) if probed < limit => {
                    probed += 1;
                    if let Some(profile) = self
                        .searches
                        .values()
                        .find_map(|s| self.policy.run(|| s.profile(&symbol)).ok().flatten())
                    {
                        listing.currency = profile.currency;
                        listing.has_history = profile.has_history;
                        listing.last_close = profile.last_close;
                        if listing.name.is_none() {
                            listing.name = Some(profile.name);
                        }
                    } else {
                        listing.has_history = Some(false);
                    }
                }
                _ => {}
            }
            out.push(listing);
        }
        out
    }

    pub fn best_listing(&self, isin: &str, preferred: Option<&str>) -> Result<Option<Listing>> {
        Ok(self.pick_usable(self.listings(isin)?, preferred))
    }

    /// The same choice for an instrument with no ISIN, where the venues come from the bare
    /// ticker rather than from the directory ([`Self::listings_by_symbol`]).
    pub fn best_listing_by_symbol(&self, symbol: &str, preferred: Option<&str>) -> Result<Option<Listing>> {
        Ok(self.pick_usable(self.listings_by_symbol(symbol)?, preferred))
    }

    /// First candidate that actually has candles, preferring the given currency. Probed one at a
    /// time so a list of venues costs only the requests it takes to find a usable one.
    fn pick_usable(&self, candidates: Vec<Listing>, preferred: Option<&str>) -> Option<Listing> {
        let preferred = preferred.map(normalize_currency);
        let mut best: Option<Listing> = None;
        for listing in candidates.into_iter().take(MAX_PROFILE_PROBES) {
            let checked = self.probe_listings(vec![listing], 1).remove(0);
            if !checked.is_usable() {
                continue;
            }
            if preferred.is_none() || checked.currency == preferred {
                return Some(checked);
            }
            if best.is_none() {
                best = Some(checked);
            }
        }
        best
    }

    pub fn search(&self, query: &str) -> Result<Vec<SecurityMatch>> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let mut ids: Vec<&&'static str> = self.searches.keys().collect();
        ids.sort_unstable();

        let mut out = Vec::new();
        let mut last_error = None;
        for id in ids {
            match self.policy.run(|| self.searches[*id].search(query)) {
                Ok(found) => out.extend(found),
                Err(e) => last_error = Some(e),
            }
        }
        match last_error {
            Some(e) if out.is_empty() => Err(e),
            _ => Ok(out),
        }
    }

    pub fn resolve(&self, query: &str) -> Result<Option<SecurityMatch>> {
        self.resolve_preferring(query, None)
    }

    /// The profile of one exact symbol at one search source. Unlike `resolve`, it never swaps in
    /// another listing; `None` when the source is unknown or does not know the symbol.
    pub fn profile(&self, source: &str, symbol: &str) -> Result<Option<SecurityMatch>> {
        let Some(search) = self.searches.get(source) else {
            return Ok(None);
        };
        self.policy.run(|| search.profile(symbol))
    }

    pub fn resolve_preferring(
        &self,
        query: &str,
        preferred_currency: Option<&str>,
    ) -> Result<Option<SecurityMatch>> {
        let query = query.trim();
        let preferred = preferred_currency.map(normalize_currency);
        let mut candidates = self.search(query)?;
        if candidates.is_empty() {
            return Ok(None);
        }
        candidates.sort_by_key(|c| looks_like_a_placeholder(&c.symbol, query));

        let mut best: Option<SecurityMatch> = None;
        for candidate in candidates.into_iter().take(MAX_PROFILE_PROBES) {
            let Some(search) = self.searches.get(candidate.source.as_str()) else {
                continue;
            };
            let profile = match self.policy.run(|| search.profile(&candidate.symbol)) {
                Ok(Some(profile)) => SecurityMatch {
                    exchange: profile.exchange.or(candidate.exchange),
                    mic: profile.mic.or(candidate.mic),
                    ..profile
                },
                _ => continue,
            };
            let matches_currency = preferred.is_some() && profile.currency == preferred;
            let usable = profile.has_history == Some(true);

            if usable && (matches_currency || preferred.is_none()) {
                return Ok(Some(self.with_isin(profile, query)));
            }
            if best.as_ref().is_none_or(|b| rank(b) < rank(&profile)) {
                best = Some(profile);
            }
        }
        Ok(best.map(|f| self.with_isin(f, query)))
    }

    fn with_isin(&self, mut found: SecurityMatch, query: &str) -> SecurityMatch {
        if is_isin(query) {
            found.isin = Some(query.to_uppercase());
        }
        found
    }

    pub fn provider_ids(&self) -> Vec<&'static str> {
        let mut ids: Vec<&'static str> = self.providers.keys().copied().collect();
        ids.sort_unstable();
        ids
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

    /// Asks the instrument's latest-close source (ADR-0079) for the days after its stored series
    /// ends, through `through`. Only that tail: a hole inside the history is the fallbacks'
    /// business. Checked against the stored overlap like a fallback, written only on days nothing
    /// is stored for, and extending no coverage: a day the own source is later asked for and
    /// answers replaces the close written here.
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
        // A source switched off is not registered: the role stays on the instrument and simply
        // asks nothing until it is back on, rather than failing every refresh.
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
        super::guard::check(
            id,
            &currency,
            &store.quotes_in_range(&security.id, asked)?,
            &quotes,
        )?;
        let tail: Vec<_> = quotes.into_iter().filter(|q| q.date > last).collect();
        store.fill_quotes(&tail)
    }
}

fn overlap(a: Option<DateRange>, b: Option<DateRange>) -> Option<DateRange> {
    let (a, b) = (a?, b?);
    let range = DateRange::new(a.from.max(b.from), a.to.min(b.to));
    (range.from <= range.to).then_some(range)
}

fn missing_ranges(covered: Option<DateRange>, wanted: DateRange) -> Vec<DateRange> {
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

#[cfg(test)]
mod chain_tests;
#[cfg(test)]
mod listing_tests;
#[cfg(test)]
mod resolve_tests;
#[cfg(test)]
mod tests;
