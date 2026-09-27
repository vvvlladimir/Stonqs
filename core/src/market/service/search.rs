//! Finding an instrument by name, ticker or ISIN.

use super::{MAX_PROFILE_PROBES, MarketDataService};
use crate::error::Result;
use crate::market::SecurityMatch;
use crate::model::is_isin;
use crate::money::normalize_currency;

fn rank(found: &SecurityMatch) -> u8 {
    u8::from(found.has_history == Some(true)) * 2 + u8::from(found.currency.is_some())
}

fn looks_like_a_placeholder(symbol: &str, query: &str) -> u8 {
    let root = symbol.split('.').next().unwrap_or(symbol);
    u8::from(is_isin(root) || root.eq_ignore_ascii_case(query))
}

impl MarketDataService {
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
}
