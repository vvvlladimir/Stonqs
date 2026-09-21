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
        let preferred = preferred.map(normalize_currency);
        let candidates = self.listings(isin)?;

        let mut best: Option<Listing> = None;
        for listing in candidates.into_iter().take(MAX_PROFILE_PROBES) {
            let checked = self.probe_listings(vec![listing], 1).remove(0);
            if !checked.is_usable() {
                continue;
            }
            if preferred.is_none() || checked.currency == preferred {
                return Ok(Some(checked));
            }
            if best.is_none() {
                best = Some(checked);
            }
        }
        Ok(best)
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
mod tests {
    use super::*;
    use crate::market::Quote;
    use crate::model::SecurityKind;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;
    use std::sync::Mutex;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn no_coverage_means_fetch_everything() {
        let want = DateRange::new(d(2024, 1, 1), d(2024, 3, 1));
        assert_eq!(missing_ranges(None, want), vec![want]);
    }

    #[test]
    fn fully_covered_means_no_fetch() {
        let covered = DateRange::new(d(2024, 1, 1), d(2024, 12, 31));
        let want = DateRange::new(d(2024, 3, 1), d(2024, 4, 1));
        assert!(missing_ranges(Some(covered), want).is_empty());
    }

    #[test]
    fn fetches_only_head_and_tail() {
        let covered = DateRange::new(d(2024, 2, 1), d(2024, 2, 29));
        let want = DateRange::new(d(2024, 1, 15), d(2024, 3, 10));
        let gaps = missing_ranges(Some(covered), want);
        assert_eq!(
            gaps,
            vec![
                DateRange::new(d(2024, 1, 15), d(2024, 1, 31)),
                DateRange::new(d(2024, 3, 1), d(2024, 3, 10)),
            ]
        );
    }

    struct CountingProvider {
        calls: Mutex<Vec<DateRange>>,
    }

    impl QuoteProvider for CountingProvider {
        fn id(&self) -> &'static str {
            "counting"
        }
        fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>> {
            self.calls.lock().unwrap().push(range);
            Ok(vec![Quote {
                security_id: security.id.clone(),
                date: range.from,
                close: dec!(100),
                currency: security.currency.clone(),
                source: "counting".into(),
            }])
        }
    }

    #[test]
    fn second_request_for_same_range_does_not_hit_provider() {
        let store = Store::open_in_memory().unwrap();
        let sec = Security::new("TEST", "Test", "USD", SecurityKind::Stock).with_source("counting", "test");
        store.save_security(&sec).unwrap();

        let mut svc = MarketDataService::new();
        svc.register(Box::new(CountingProvider {
            calls: Mutex::new(Vec::new()),
        }));

        let range = DateRange::new(d(2024, 1, 1), d(2024, 1, 31));
        svc.ensure_history(&store, &sec, range).unwrap();
        svc.ensure_history(&store, &sec, range).unwrap();

        let provider = svc.provider("counting").unwrap();
        assert_eq!(provider.id(), "counting");
        let saved = store.quotes_in_range(&sec.id, range).unwrap();
        assert_eq!(saved.len(), 1);
    }

    struct ReportingProvider;

    impl QuoteProvider for ReportingProvider {
        fn id(&self) -> &'static str {
            "reporting"
        }
        fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>> {
            Ok(self.fetch_history(security, range)?.quotes)
        }
        fn fetch_history(&self, security: &Security, range: DateRange) -> Result<crate::market::History> {
            Ok(crate::market::History {
                quotes: Vec::new(),
                events: vec![crate::model::SecurityEvent::split(
                    &security.id,
                    range.from,
                    dec!(1),
                    dec!(4),
                    "reporting",
                )],
            })
        }
    }

    /// Quotes fetched before events existed leave no event coverage, so the whole range is
    /// asked again once and the events that come back are saved.
    #[test]
    fn a_range_without_event_coverage_is_fetched_again() {
        let store = Store::open_in_memory().unwrap();
        let sec = Security::new("TEST", "Test", "USD", SecurityKind::Stock).with_source("reporting", "test");
        store.save_security(&sec).unwrap();
        let range = DateRange::new(d(2024, 1, 1), d(2024, 1, 31));
        store.extend_quote_coverage(&sec.id, range).unwrap();

        let svc = MarketDataService::new().with(Box::new(ReportingProvider));
        svc.ensure_history(&store, &sec, range).unwrap();

        assert_eq!(store.event_coverage(&sec.id).unwrap(), Some(range));
        assert_eq!(store.security_events_for(&sec.id).unwrap().len(), 1);
        assert!(missing_ranges(overlap(Some(range), Some(range)), range).is_empty());
    }

    #[test]
    fn the_unsettled_day_is_asked_again() {
        let store = Store::open_in_memory().unwrap();
        let sec = Security::new("TEST", "Test", "USD", SecurityKind::Stock).with_source("counting", "test");
        store.save_security(&sec).unwrap();

        let svc = MarketDataService::new().with(Box::new(CountingProvider {
            calls: Mutex::new(Vec::new()),
        }));

        let range = DateRange::new(d(2024, 1, 10), d(2024, 1, 15));
        let settled = d(2024, 1, 14);
        svc.ensure_history_through(&store, &sec, range, settled).unwrap();

        assert_eq!(
            store.quote_coverage(&sec.id).unwrap(),
            Some(DateRange::new(d(2024, 1, 10), settled))
        );

        assert_eq!(
            missing_ranges(store.quote_coverage(&sec.id).unwrap(), range),
            vec![DateRange::new(d(2024, 1, 15), d(2024, 1, 15))]
        );
    }
}

#[cfg(test)]
mod resolve_tests {
    use super::*;
    use crate::model::SecurityKind;

    struct TwoListings;

    fn listing(symbol: &str, currency: &str, history: Option<bool>) -> SecurityMatch {
        SecurityMatch {
            source: "fake".into(),
            symbol: symbol.into(),
            name: "iShares MSCI Global Semiconductors".into(),
            exchange: None,
            mic: None,
            kind: SecurityKind::Etf,
            currency: Some(currency.into()),
            isin: None,
            has_history: history,
            last_close: None,
        }
    }

    impl SecuritySearch for TwoListings {
        fn id(&self) -> &'static str {
            "fake"
        }

        fn search(&self, _: &str) -> Result<Vec<SecurityMatch>> {
            Ok(vec![
                listing("IE000I8KRLL9.SG", "EUR", None),
                listing("SEMI.AS", "USD", None),
                listing("SEC0.DE", "EUR", None),
            ])
        }

        fn profile(&self, symbol: &str) -> Result<Option<SecurityMatch>> {
            Ok(Some(match symbol {
                "IE000I8KRLL9.SG" => listing(symbol, "EUR", Some(false)),
                "SEMI.AS" => listing(symbol, "USD", Some(true)),
                _ => listing(symbol, "EUR", Some(true)),
            }))
        }
    }

    fn service() -> MarketDataService {
        MarketDataService::new()
            .with_policy(FetchPolicy::none())
            .with_search(Box::new(TwoListings))
    }

    #[test]
    fn a_listing_without_history_never_wins() {
        let found = service().resolve("IE000I8KRLL9").unwrap().unwrap();
        assert_ne!(found.symbol, "IE000I8KRLL9.SG");
        assert_eq!(found.has_history, Some(true));
        assert_eq!(found.isin.as_deref(), Some("IE000I8KRLL9"));
    }

    #[test]
    fn the_preferred_currency_decides_between_working_listings() {
        let service = service();
        assert_eq!(
            service.resolve("IE000I8KRLL9").unwrap().unwrap().symbol,
            "SEMI.AS"
        );
        assert_eq!(
            service
                .resolve_preferring("IE000I8KRLL9", Some("EUR"))
                .unwrap()
                .unwrap()
                .symbol,
            "SEC0.DE"
        );
    }
}

#[cfg(test)]
mod listing_tests {
    use super::*;
    use crate::model::SecurityKind;
    use rust_decimal_macros::dec;

    struct ThreeMarkets;

    impl ListingDirectory for ThreeMarkets {
        fn id(&self) -> &'static str {
            "fake-directory"
        }

        fn listings(&self, isin: &str) -> Result<Vec<Listing>> {
            Ok(["XETR", "XLON", "XTAE"]
                .iter()
                .zip(["EUNL", "IWDA", "SWDA"])
                .map(|(mic, ticker)| Listing {
                    isin: isin.to_string(),
                    mic: (*mic).to_string(),
                    ticker: ticker.to_string(),
                    exchange: crate::market::mic::market_name(mic).map(str::to_string),
                    name: None,
                    symbol: None,
                    currency: None,
                    has_history: None,
                    last_close: None,
                    source: "fake-directory".to_string(),
                })
                .collect())
        }
    }

    struct FakeQuotes;

    impl SecuritySearch for FakeQuotes {
        fn id(&self) -> &'static str {
            "fake"
        }

        /// What a provider search returns for a bare US ticker: the same instrument on two
        /// venues, plus an unrelated company that merely starts with the same letters.
        fn search(&self, query: &str) -> Result<Vec<SecurityMatch>> {
            if query != "AWK" {
                return Ok(Vec::new());
            }
            Ok([
                ("AWK", Some("XNYS")),
                ("AWK.DE", Some("XFRA")),
                ("AWKR", Some("XNAS")),
            ]
            .into_iter()
            .map(|(symbol, mic)| SecurityMatch {
                source: "fake".into(),
                symbol: symbol.into(),
                name: "American Water Works".into(),
                exchange: None,
                mic: mic.map(str::to_string),
                kind: SecurityKind::Stock,
                currency: None,
                isin: None,
                has_history: None,
                last_close: None,
            })
            .collect())
        }

        fn profile(&self, symbol: &str) -> Result<Option<SecurityMatch>> {
            let (currency, history, close) = match symbol {
                "EUNL.DE" => ("EUR", true, dec!(127.455)),
                "IWDA.L" => ("USD", true, dec!(148.09)),
                _ => ("ILS", false, dec!(0)),
            };
            Ok(Some(SecurityMatch {
                source: "fake".into(),
                symbol: symbol.into(),
                name: "iShares Core MSCI World".into(),
                exchange: None,
                mic: None,
                kind: SecurityKind::Etf,
                currency: Some(currency.into()),
                isin: None,
                has_history: Some(history),
                last_close: Some(close),
            }))
        }

        fn symbol_for(&self, ticker: &str, mic: &str) -> Option<String> {
            let suffix = match mic {
                "XETR" => ".DE",
                "XLON" => ".L",
                "XTAE" => ".TA",
                _ => return None,
            };
            Some(format!("{ticker}{suffix}"))
        }
    }

    fn service() -> MarketDataService {
        MarketDataService::new()
            .with_policy(FetchPolicy::none())
            .with_search(Box::new(FakeQuotes))
            .with_directory(Box::new(ThreeMarkets))
    }

    #[test]
    fn an_instrument_without_an_isin_gets_its_venues_from_the_search() {
        let found = service().listings_by_symbol("AWK").unwrap();
        assert_eq!(
            found.iter().map(|l| l.mic.as_str()).collect::<Vec<_>>(),
            ["XNYS", "XFRA"],
            "AWKR is another company, not another venue of AWK"
        );
        assert!(
            found.iter().all(|l| l.isin.is_empty()),
            "there is no ISIN to record"
        );
        assert_eq!(found[0].symbol.as_deref(), Some("AWK"));
        assert_eq!(found[1].symbol.as_deref(), Some("AWK.DE"));
    }

    #[test]
    fn a_venue_the_source_cannot_name_is_not_a_listing() {
        // The suffixed symbol is what the search is run for, and only the base ticker matches.
        assert!(service().listings_by_symbol("EUNL.DE").unwrap().is_empty());
    }

    #[test]
    fn listings_are_named_by_the_quote_provider() {
        let found = service().listings("IE00B4L5Y983").unwrap();
        let symbols: Vec<&str> = found.iter().filter_map(|l| l.symbol.as_deref()).collect();
        assert_eq!(symbols, ["EUNL.DE", "IWDA.L", "SWDA.TA"]);
        assert!(
            found.iter().all(|l| l.currency.is_none()),
            "the currency was not asked for"
        );
    }

    #[test]
    fn the_base_currency_decides_which_listing_wins() {
        let best = service()
            .best_listing("IE00B4L5Y983", Some("EUR"))
            .unwrap()
            .unwrap();
        assert_eq!(best.symbol.as_deref(), Some("EUNL.DE"));
        assert_eq!(best.currency.as_deref(), Some("EUR"));
        assert_eq!(best.last_close, Some(dec!(127.455)));
    }

    #[test]
    fn a_working_listing_beats_the_right_currency() {
        let best = service()
            .best_listing("IE00B4L5Y983", Some("CHF"))
            .unwrap()
            .unwrap();
        assert_eq!(best.symbol.as_deref(), Some("EUNL.DE"));
    }

    #[test]
    fn a_listing_without_prices_is_not_usable() {
        let probed = service().probe_listings(service().listings("IE00B4L5Y983").unwrap(), 10);
        let tel_aviv = probed.iter().find(|l| l.mic == "XTAE").unwrap();
        assert_eq!(tel_aviv.has_history, Some(false));
        assert!(!tel_aviv.is_usable());
    }
}

#[cfg(test)]
mod chain_tests {
    use super::*;
    use crate::market::Quote;
    use crate::model::SecurityKind;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    /// A source that is down, or that closes every day at `close` in `currency`.
    struct Fixed {
        id: &'static str,
        down: bool,
        close: rust_decimal::Decimal,
        currency: &'static str,
        calls: std::sync::Arc<Mutex<usize>>,
    }

    impl Fixed {
        fn up(id: &'static str, close: rust_decimal::Decimal, currency: &'static str) -> Box<Self> {
            Box::new(Fixed {
                id,
                down: false,
                close,
                currency,
                calls: Default::default(),
            })
        }
        fn down(id: &'static str) -> Box<Self> {
            Box::new(Fixed {
                id,
                down: true,
                close: dec!(0),
                currency: "USD",
                calls: Default::default(),
            })
        }
    }

    impl QuoteProvider for Fixed {
        fn id(&self) -> &'static str {
            self.id
        }
        fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>> {
            *self.calls.lock().unwrap() += 1;
            if self.down {
                return Err(Error::Unavailable("503".into()));
            }
            Ok(range
                .from
                .iter_days()
                .take_while(|day| *day <= range.to)
                .map(|date| Quote {
                    security_id: security.id.clone(),
                    date,
                    close: self.close,
                    currency: self.currency.into(),
                    source: self.id.into(),
                })
                .collect())
        }
    }

    fn listed(store: &Store, symbol: &str) -> Security {
        let sec = Security::new(symbol, symbol, "USD", SecurityKind::Stock).with_source("primary", symbol);
        store.save_security(&sec).unwrap();
        store.set_security_symbol(&sec.id, "backup", symbol).unwrap();
        sec
    }

    fn chain(backup: Box<Fixed>) -> MarketDataService {
        MarketDataService::new()
            .with_policy(FetchPolicy::none())
            .with(Fixed::down("primary"))
            .with(backup)
    }

    #[test]
    fn a_fallback_fills_the_gap_without_claiming_coverage() {
        let store = Store::open_in_memory().unwrap();
        let sec = listed(&store, "ACME");
        let week = DateRange::new(d(2024, 6, 3), d(2024, 6, 7));

        // 5 days asked, fallback answers from 14 days earlier: 5 + 14 = 19 rows, all new.
        let saved = chain(Fixed::up("backup", dec!(10), "USD"))
            .ensure_history(&store, &sec, week)
            .unwrap();
        assert_eq!(saved, 19);
        assert_eq!(store.quotes_in_range(&sec.id, week).unwrap()[0].source, "backup");
        assert_eq!(store.quote_coverage(&sec.id).unwrap(), None);
    }

    #[test]
    fn a_fallback_that_disagrees_with_the_stored_series_is_refused() {
        let store = Store::open_in_memory().unwrap();
        let sec = listed(&store, "ACME");
        let week = DateRange::new(d(2024, 6, 3), d(2024, 6, 7));
        store
            .save_quotes(&[Quote {
                security_id: sec.id.clone(),
                date: d(2024, 5, 31),
                close: dec!(10),
                currency: "USD".into(),
                source: "primary".into(),
            }])
            .unwrap();

        // Stored 10, fallback 1000 on 05-31: ratio 100, far outside 2%.
        let err = chain(Fixed::up("backup", dec!(1000), "USD")).ensure_history(&store, &sec, week);
        assert!(matches!(err, Err(Error::Unavailable(_))));
        assert_eq!(store.quotes_in_range(&sec.id, week).unwrap(), vec![]);
    }

    #[test]
    fn a_source_without_the_instrument_s_symbol_is_not_asked() {
        let store = Store::open_in_memory().unwrap();
        let sec = Security::new("SOLO", "Solo", "USD", SecurityKind::Stock).with_source("primary", "SOLO");
        store.save_security(&sec).unwrap();
        let week = DateRange::new(d(2024, 6, 3), d(2024, 6, 7));
        assert!(
            chain(Fixed::up("backup", dec!(10), "USD"))
                .ensure_history(&store, &sec, week)
                .is_err()
        );
    }

    #[test]
    fn a_source_that_keeps_failing_rests_for_the_rest_of_the_refresh() {
        let store = Store::open_in_memory().unwrap();
        let primary = Fixed::down("primary");
        let calls = primary.calls.clone();
        let svc = MarketDataService::new()
            .with_policy(FetchPolicy::none())
            .with(primary);
        let week = DateRange::new(d(2024, 6, 3), d(2024, 6, 7));
        for symbol in ["A", "B", "C", "D", "E"] {
            let _ = svc.ensure_history(&store, &listed(&store, symbol), week);
        }
        assert_eq!(*calls.lock().unwrap(), RESTS_AFTER as usize);
    }
}
