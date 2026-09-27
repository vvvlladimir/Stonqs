//! Which venues an instrument trades on, and which of them has prices.

use super::{MAX_PROFILE_PROBES, MarketDataService};
use crate::error::Result;
use crate::market::{Listing, mic};
use crate::money::normalize_currency;

/// The ticker without the venue suffix a provider spells into its symbol.
fn base_ticker(symbol: &str) -> String {
    let symbol = symbol.trim().to_uppercase();
    match symbol.find('.') {
        Some(dot) => symbol[..dot].to_string(),
        None => symbol,
    }
}

impl MarketDataService {
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

    /// Venues of an instrument with no ISIN, from a search for the bare ticker. `isin` is empty,
    /// so the caller must not cache the result.
    pub fn listings_by_symbol(&self, symbol: &str) -> Result<Vec<Listing>> {
        let ticker = base_ticker(symbol);
        if ticker.is_empty() {
            return Ok(Vec::new());
        }
        let mut out: Vec<Listing> = Vec::new();
        for found in self.search(&ticker)? {
            // Only the same ticker on another venue is the same instrument.
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
}
