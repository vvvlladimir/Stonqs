//! Coordinates quote, search and listing sources with retries, budgets and persistence.

mod history;
mod listings;
mod search;

#[cfg(test)]
use history::{missing_ranges, overlap};

use super::{FetchPolicy, ListingDirectory, QuoteProvider, SecuritySearch};
use std::collections::HashMap;
use std::sync::Mutex;

/// How many candidates are probed for candles before giving up.
const MAX_PROFILE_PROBES: usize = 5;
/// Consecutive failures after which a source rests for this service's life (one refresh).
const RESTS_AFTER: u32 = 3;
/// How far before a gap a fallback is asked, so its closes overlap the stored series.
const OVERLAP_DAYS: i64 = 14;

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

    pub fn provider_ids(&self) -> Vec<&'static str> {
        let mut ids: Vec<&'static str> = self.providers.keys().copied().collect();
        ids.sort_unstable();
        ids
    }
}

#[cfg(test)]
mod chain_tests;
#[cfg(test)]
mod listing_tests;
#[cfg(test)]
mod resolve_tests;
#[cfg(test)]
mod tests;
