use super::IndexProvider;
use crate::error::{Error, Result};
use crate::market::{DateRange, FetchPolicy};
use crate::model::normalize_region;
use crate::storage::Store;

/// Provider chain that fetches index levels and persists them in `Store`.
#[derive(Default)]
pub struct InflationService {
    /// In the order a region is asked: the first that covers it and answers wins.
    providers: Vec<Box<dyn IndexProvider>>,
    policy: FetchPolicy,
}

impl InflationService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(mut self, policy: FetchPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Appends a provider to the end of the chain; registering an id again replaces it in place.
    pub fn register(&mut self, provider: Box<dyn IndexProvider>) {
        match self.providers.iter_mut().find(|p| p.id() == provider.id()) {
            Some(slot) => *slot = provider,
            None => self.providers.push(provider),
        }
    }

    pub fn with(mut self, provider: Box<dyn IndexProvider>) -> Self {
        self.register(provider);
        self
    }

    pub fn provider_ids(&self) -> Vec<&'static str> {
        self.providers.iter().map(|p| p.id()).collect()
    }

    /// The source a region is asked first.
    pub fn first_for(&self, region: &str) -> Option<&'static str> {
        let region = normalize_region(region);
        self.providers.iter().find(|p| p.covers(&region)).map(|p| p.id())
    }

    /// Fetches one region from the first source that covers it, falling through to the next
    /// when one fails. Unlike the FX chain a fallback does not fill gaps: publishers use
    /// different index bases (2015 = 100 against 2010 = 100), so one region's series comes
    /// whole from one source or the ratio of two months inside it would be a fiction.
    /// Returns the rows saved and the source that answered.
    pub fn ensure_index(
        &self,
        store: &Store,
        region: &str,
        range: DateRange,
    ) -> Result<(usize, &'static str)> {
        let region = normalize_region(region);
        let mut first_error = None;
        for provider in self.providers.iter().filter(|p| p.covers(&region)) {
            match self.policy.run(|| provider.fetch(&region, range)) {
                Ok(points) => {
                    let saved = store.save_index_from(&region, &points, provider.id(), range)?;
                    return Ok((saved, provider.id()));
                }
                Err(e) => {
                    first_error.get_or_insert(e);
                }
            }
        }
        Err(first_error.unwrap_or_else(|| Error::NotFound(format!("inflation source for {region}"))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inflation::{IndexLookup, IndexPoint, StaticIndexProvider};
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    /// Publishes a fixed set of regions, or fails every request.
    struct Narrow {
        id: &'static str,
        covers: &'static [&'static str],
        down: bool,
        inner: StaticIndexProvider,
    }

    impl IndexProvider for Narrow {
        fn id(&self) -> &'static str {
            self.id
        }
        fn fetch(&self, region: &str, range: DateRange) -> Result<Vec<IndexPoint>> {
            if self.down {
                return Err(Error::Unavailable("503".into()));
            }
            self.inner.fetch(region, range)
        }
        fn covers(&self, region: &str) -> bool {
            self.covers.contains(&region)
        }
    }

    fn year() -> DateRange {
        DateRange::new(d("2024-01-01"), d("2024-12-31"))
    }

    fn harmonised(down: bool) -> Box<Narrow> {
        Box::new(Narrow {
            id: "harmonised",
            covers: &["EA", "DE"],
            down,
            inner: StaticIndexProvider::new()
                .with("DE", d("2024-01-01"), dec!(118.0))
                .with("DE", d("2024-02-01"), dec!(118.6)),
        })
    }

    fn worldwide() -> Box<Narrow> {
        Box::new(Narrow {
            id: "worldwide",
            covers: &["DE", "JP"],
            down: false,
            // The same months on another base: 2010 = 100 rather than 2015 = 100.
            inner: StaticIndexProvider::new()
                .with("DE", d("2024-01-01"), dec!(129.8))
                .with("DE", d("2024-02-01"), dec!(130.5))
                .with("JP", d("2024-01-01"), dec!(106.9)),
        })
    }

    #[test]
    fn the_first_source_that_covers_the_region_answers() {
        let store = Store::open_in_memory().unwrap();
        let service = InflationService::new()
            .with_policy(FetchPolicy::none())
            .with(harmonised(false))
            .with(worldwide());
        assert_eq!(
            service.ensure_index(&store, "DE", year()).unwrap(),
            (2, "harmonised")
        );
        assert_eq!(
            service.ensure_index(&store, "JP", year()).unwrap(),
            (1, "worldwide")
        );
    }

    #[test]
    fn a_failing_source_falls_through_to_the_next() {
        let store = Store::open_in_memory().unwrap();
        let service = InflationService::new()
            .with_policy(FetchPolicy::none())
            .with(harmonised(true))
            .with(worldwide());
        assert_eq!(
            service.ensure_index(&store, "DE", year()).unwrap(),
            (2, "worldwide")
        );
    }

    #[test]
    fn a_second_source_replaces_the_series_rather_than_mixing_bases() {
        let store = Store::open_in_memory().unwrap();
        InflationService::new()
            .with_policy(FetchPolicy::none())
            .with(harmonised(false))
            .ensure_index(&store, "DE", year())
            .unwrap();
        InflationService::new()
            .with_policy(FetchPolicy::none())
            .with(worldwide())
            .ensure_index(&store, "DE", year())
            .unwrap();
        // Not 118.0 kept beside 130.5: one base for the whole region.
        assert_eq!(
            store.index_as_of("DE", d("2024-01-31")).unwrap(),
            Some(dec!(129.8))
        );
        assert_eq!(
            store.index_as_of("DE", d("2024-02-29")).unwrap(),
            Some(dec!(130.5))
        );
    }

    #[test]
    fn a_region_nobody_covers_is_not_found() {
        let store = Store::open_in_memory().unwrap();
        let service = InflationService::new().with(harmonised(false));
        assert!(matches!(
            service.ensure_index(&store, "JP", year()),
            Err(Error::NotFound(_))
        ));
    }
}
