use super::FxProvider;
use crate::error::{Error, Result};
use crate::market::{Budgets, DateRange, FetchPolicy};
use crate::money::normalize_currency;
use crate::storage::Store;

/// Provider chain that fetches FX data and persists it in `Store`.
#[derive(Default)]
pub struct FxService {
    /// In the order a pair is asked: the first that covers both currencies and answers wins.
    providers: Vec<Box<dyn FxProvider>>,
    policy: FetchPolicy,
    budgets: Budgets,
}
/// Sets the retry policy used around provider calls.
impl FxService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: FetchPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Appends a provider to the end of the chain; registering an id again replaces it in place.
    pub fn register(&mut self, provider: Box<dyn FxProvider>) {
        match self.providers.iter_mut().find(|p| p.id() == provider.id()) {
            Some(slot) => *slot = provider,
            None => self.providers.push(provider),
        }
    }

    pub fn with(mut self, provider: Box<dyn FxProvider>) -> Self {
        self.register(provider);
        self
    }

    /// Caps a source at `per_day` requests, counted in the store the rates are saved to.
    pub fn set_budget(&mut self, source: &'static str, per_day: u32) {
        self.budgets.set(source, per_day);
    }

    /// The source a pair is asked first — the one whose error a failed `ensure_rates` reports.
    pub fn first_for(&self, base: &str, quote: &str) -> Option<&'static str> {
        let (base, quote) = (normalize_currency(base), normalize_currency(quote));
        self.providers
            .iter()
            .find(|p| p.covers(&base) && p.covers(&quote))
            .map(|p| p.id())
    }

    pub fn provider_ids(&self) -> Vec<&'static str> {
        self.providers.iter().map(|p| p.id()).collect()
    }

    /// Fetches and stores one pair from the first source that covers it, falling through to the
    /// next when one fails. An empty answer is final: a weekend is not a reason to mix sources.
    /// Returns the rows saved and the source that answered.
    pub fn ensure_rates(
        &self,
        store: &Store,
        base: &str,
        quote: &str,
        range: DateRange,
    ) -> Result<(usize, &'static str)> {
        let (base, quote) = (normalize_currency(base), normalize_currency(quote));
        let mut first_error = None;
        let covering = self
            .providers
            .iter()
            .filter(|p| p.covers(&base) && p.covers(&quote));
        for (rank, provider) in covering.enumerate() {
            let answer = self
                .budgets
                .spend(store, provider.id())
                .and_then(|()| self.policy.run(|| provider.fetch(&base, &quote, range)));
            match answer {
                // The pair's own source is the first that covers it; any later one only fills gaps.
                Ok(rates) => {
                    let saved = store.save_fx_rates_from(&rates, provider.id(), rank == 0)?;
                    return Ok((saved, provider.id()));
                }
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
        Err(first_error.unwrap_or_else(|| Error::NotFound(format!("fx source for {base}/{quote}"))))
    }

    /// Fetches and stores one pair from the named source only.
    pub fn ensure_rates_from(
        &self,
        store: &Store,
        provider_id: &str,
        base: &str,
        quote: &str,
        range: DateRange,
    ) -> Result<usize> {
        let provider = self
            .providers
            .iter()
            .find(|p| p.id() == provider_id)
            .ok_or_else(|| Error::NotFound(format!("fx provider {provider_id}")))?;
        let (base, quote) = (normalize_currency(base), normalize_currency(quote));
        self.budgets.spend(store, provider.id())?;
        let rates = self.policy.run(|| provider.fetch(&base, &quote, range))?;
        store.save_fx_rates_from(&rates, provider.id(), true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::{FxRate, StaticFxProvider};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    /// Publishes a fixed set of currencies, or fails every request.
    struct Narrow {
        id: &'static str,
        covers: &'static [&'static str],
        down: bool,
        inner: StaticFxProvider,
    }

    impl FxProvider for Narrow {
        fn id(&self) -> &'static str {
            self.id
        }
        fn fetch(&self, base: &str, quote: &str, range: DateRange) -> Result<Vec<FxRate>> {
            if self.down {
                return Err(Error::Unavailable("503".into()));
            }
            self.inner.fetch(base, quote, range)
        }
        fn covers(&self, currency: &str) -> bool {
            self.covers.contains(&currency)
        }
    }

    fn week() -> DateRange {
        DateRange::new(d("2024-06-03"), d("2024-06-07"))
    }

    fn central(down: bool) -> Box<Narrow> {
        Box::new(Narrow {
            id: "central",
            covers: &["EUR", "USD"],
            down,
            inner: StaticFxProvider::new().with("USD", "EUR", d("2024-06-03"), dec!(0.92)),
        })
    }

    fn market() -> Box<Narrow> {
        Box::new(Narrow {
            id: "market",
            covers: &["EUR", "USD", "RUB"],
            down: false,
            inner: StaticFxProvider::new()
                .with("USD", "EUR", d("2024-06-03"), dec!(0.93))
                .with("RUB", "EUR", d("2024-06-03"), dec!(0.0102)),
        })
    }

    #[test]
    fn the_first_source_that_covers_the_pair_answers() {
        let store = Store::open_in_memory().unwrap();
        let fx = FxService::new()
            .with_policy(FetchPolicy::none())
            .with(central(false))
            .with(market());
        assert_eq!(
            fx.ensure_rates(&store, "USD", "EUR", week()).unwrap(),
            (1, "central")
        );
        assert_eq!(
            fx.ensure_rates(&store, "RUB", "EUR", week()).unwrap(),
            (1, "market")
        );
    }

    #[test]
    fn a_failing_source_falls_through_to_the_next() {
        let store = Store::open_in_memory().unwrap();
        let fx = FxService::new()
            .with_policy(FetchPolicy::none())
            .with(central(true))
            .with(market());
        assert_eq!(
            fx.ensure_rates(&store, "USD", "EUR", week()).unwrap(),
            (1, "market")
        );
    }

    #[test]
    fn a_fallback_fills_gaps_and_never_rewrites_the_primary() {
        let store = Store::open_in_memory().unwrap();
        let healthy = FxService::new()
            .with_policy(FetchPolicy::none())
            .with(central(false))
            .with(market());
        healthy.ensure_rates(&store, "USD", "EUR", week()).unwrap();
        // central wrote 0.92 on 06-03; with central down, market's 0.93 for the same day is ignored.
        let outage = FxService::new()
            .with_policy(FetchPolicy::none())
            .with(central(true))
            .with(market());
        assert_eq!(
            outage.ensure_rates(&store, "USD", "EUR", week()).unwrap(),
            (0, "market")
        );
        assert_eq!(
            store.rate_series("USD", "EUR", d("2024-06-07")).unwrap()[&d("2024-06-03")],
            dec!(0.92)
        );
        assert_eq!(
            store.fx_rate_sources("USD", "EUR").unwrap()[&d("2024-06-03")],
            "central"
        );
    }

    #[test]
    fn a_spent_daily_budget_passes_the_pair_to_the_next_source() {
        let store = Store::open_in_memory().unwrap();
        let mut fx = FxService::new()
            .with_policy(FetchPolicy::none())
            .with(central(false))
            .with(market());
        fx.set_budget("central", 1);
        // One request allowed today: the first ask spends it, the second goes to `market`.
        assert_eq!(
            fx.ensure_rates(&store, "USD", "EUR", week()).unwrap().1,
            "central"
        );
        assert_eq!(fx.ensure_rates(&store, "USD", "EUR", week()).unwrap().1, "market");
        let today = chrono::Utc::now().date_naive();
        assert_eq!(store.requests_on(today).unwrap()["central"], 1);
    }

    #[test]
    fn an_empty_answer_is_final() {
        let store = Store::open_in_memory().unwrap();
        let fx = FxService::new()
            .with_policy(FetchPolicy::none())
            .with(central(false))
            .with(market());
        let weekend = DateRange::new(d("2024-06-08"), d("2024-06-09"));
        assert_eq!(
            fx.ensure_rates(&store, "USD", "EUR", weekend).unwrap(),
            (0, "central")
        );
    }

    #[test]
    fn a_pair_nobody_covers_is_not_found() {
        let store = Store::open_in_memory().unwrap();
        let fx = FxService::new().with(central(false));
        assert!(matches!(
            fx.ensure_rates(&store, "RUB", "EUR", week()),
            Err(Error::NotFound(_))
        ));
    }
}
