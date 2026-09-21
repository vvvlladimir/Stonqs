//! The catalogue of market-data sources this build ships, and the services assembled from it.
//!
//! A source is one row: its stored id, what a key means to it, whether it is on by default, and a
//! constructor per role it can play. Adding one is a provider file plus a row here — nothing else
//! names a source by string. See ADR-0050.

use crate::fx::{EcbProvider, FrankfurterProvider, FxProvider, FxService, YahooFxProvider};
use crate::inflation::{EurostatProvider, ImfProvider, IndexProvider, InflationService};
use crate::market::{
    CustomProvider, CustomRole, CustomSource, EodhdProvider, KrakenProvider, ListingDirectory,
    MarketDataService, OpenFigiDirectory, QuoteProvider, SecuritySearch, StooqProvider, TwelveDataProvider,
    YahooProvider,
};
use std::collections::HashMap;

/// Where a new instrument's prices come from unless the user picks another source.
pub const DEFAULT_QUOTES: &str = YahooProvider::ID;
/// Which source answers a currency pair.
pub const DEFAULT_FX: &str = EcbProvider::ID;
/// Which source answers a consumer-price region.
pub const DEFAULT_INDEX: &str = EurostatProvider::ID;

/// One role a source can play. Derived from the constructors a row carries, never declared twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Daily closes of an instrument.
    Quotes,
    /// Finding an instrument by name, ticker or ISIN.
    Search,
    /// The venues one ISIN trades on.
    Listings,
    /// Historical exchange rates.
    FxRates,
    /// Monthly consumer-price index levels.
    PriceIndex,
}

/// What an API key means to a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyUse {
    /// Answers without one.
    None,
    /// Answers without one, with a higher limit given one (OpenFIGI).
    Optional,
    /// Refuses every request without one.
    Required,
}

/// A constructor; `None` when no key is saved for the source.
type Make<T> = fn(Option<String>) -> Box<T>;

/// One shipped source.
pub struct SourceInfo {
    pub id: &'static str,
    pub key: KeyUse,
    /// Registered unless the user switched it off. A source kept off still keeps its row, so a
    /// security that names it is shown as a known, disabled source rather than an unknown string.
    pub on_by_default: bool,
    /// Requests a day the free plan allows; `None` for a source without a published allowance.
    pub per_day: Option<u32>,
    quotes: Option<Make<dyn QuoteProvider>>,
    search: Option<Make<dyn SecuritySearch>>,
    listings: Option<Make<dyn ListingDirectory>>,
    fx: Option<Make<dyn FxProvider>>,
    index: Option<Make<dyn IndexProvider>>,
}

impl SourceInfo {
    pub fn capabilities(&self) -> Vec<Capability> {
        [
            (self.quotes.is_some(), Capability::Quotes),
            (self.search.is_some(), Capability::Search),
            (self.listings.is_some(), Capability::Listings),
            (self.fx.is_some(), Capability::FxRates),
            (self.index.is_some(), Capability::PriceIndex),
        ]
        .into_iter()
        .filter_map(|(has, capability)| has.then_some(capability))
        .collect()
    }
}

/// Every source this build can talk to, in the order a chain would try them.
pub const CATALOG: &[SourceInfo] = &[
    SourceInfo {
        id: YahooProvider::ID,
        per_day: None,
        key: KeyUse::None,
        on_by_default: true,
        quotes: Some(|_| Box::new(YahooProvider::new())),
        search: Some(|_| Box::new(YahooProvider::new())),
        listings: None,
        index: None,
        fx: Some(|_| Box::new(YahooFxProvider::new())),
    },
    SourceInfo {
        id: KrakenProvider::ID,
        per_day: None,
        key: KeyUse::None,
        on_by_default: true,
        quotes: Some(|_| Box::new(KrakenProvider::new())),
        search: None,
        listings: None,
        index: None,
        fx: None,
    },
    SourceInfo {
        id: TwelveDataProvider::ID,
        per_day: Some(800),
        key: KeyUse::Required,
        on_by_default: true,
        quotes: Some(|key| Box::new(TwelveDataProvider::new(key.unwrap_or_default()))),
        search: None,
        listings: None,
        index: None,
        fx: Some(|key| Box::new(TwelveDataProvider::new(key.unwrap_or_default()))),
    },
    SourceInfo {
        id: EodhdProvider::ID,
        per_day: Some(20),
        key: KeyUse::Required,
        on_by_default: true,
        quotes: Some(|key| Box::new(EodhdProvider::new(key.unwrap_or_default()))),
        search: None,
        listings: None,
        index: None,
        fx: None,
    },
    SourceInfo {
        id: StooqProvider::ID,
        per_day: None,
        key: KeyUse::None,
        // Off: its CSV now sits behind a JavaScript challenge a plain HTTP client cannot pass.
        on_by_default: false,
        quotes: Some(|_| Box::new(StooqProvider::new())),
        search: None,
        listings: None,
        index: None,
        fx: None,
    },
    SourceInfo {
        id: OpenFigiDirectory::ID,
        per_day: None,
        key: KeyUse::Optional,
        on_by_default: true,
        quotes: None,
        search: None,
        listings: Some(|key| match key {
            Some(key) => Box::new(OpenFigiDirectory::new().with_api_key(key)),
            None => Box::new(OpenFigiDirectory::new()),
        }),
        index: None,
        fx: None,
    },
    SourceInfo {
        id: EcbProvider::ID,
        per_day: None,
        key: KeyUse::None,
        on_by_default: true,
        quotes: None,
        search: None,
        listings: None,
        index: None,
        fx: Some(|_| Box::new(EcbProvider::new())),
    },
    SourceInfo {
        id: FrankfurterProvider::ID,
        per_day: None,
        key: KeyUse::None,
        on_by_default: true,
        quotes: None,
        search: None,
        listings: None,
        index: None,
        fx: Some(|_| Box::new(FrankfurterProvider::new())),
    },
    SourceInfo {
        id: EurostatProvider::ID,
        per_day: None,
        key: KeyUse::None,
        on_by_default: true,
        quotes: None,
        search: None,
        listings: None,
        index: Some(|_| Box::new(EurostatProvider::new())),
        fx: None,
    },
    SourceInfo {
        id: ImfProvider::ID,
        per_day: None,
        key: KeyUse::None,
        on_by_default: true,
        quotes: None,
        search: None,
        listings: None,
        index: Some(|_| Box::new(ImfProvider::new())),
        fx: None,
    },
];

pub fn info(id: &str) -> Option<&'static SourceInfo> {
    CATALOG.iter().find(|s| s.id == id)
}

/// What the user decided about the sources: saved keys and switches that differ from the default.
/// The host fills it from the vault and its settings; the core never reads either itself.
#[derive(Debug, Clone, Default)]
pub struct Setup {
    pub keys: HashMap<String, String>,
    pub switched: HashMap<String, bool>,
    /// The user's own quote sources, asked after every shipped one (ADR-0054).
    pub custom: Vec<CustomSource>,
}

impl Setup {
    /// Whether a source takes part: switched on (or on by default), and not missing a key it needs.
    pub fn is_on(&self, source: &SourceInfo) -> bool {
        let wanted = self
            .switched
            .get(source.id)
            .copied()
            .unwrap_or(source.on_by_default);
        wanted && (source.key != KeyUse::Required || self.keys.contains_key(source.id))
    }

    fn key(&self, source: &SourceInfo) -> Option<String> {
        self.keys.get(source.id).cloned()
    }

    /// The user's own sources in `role` that are switched on and well-formed.
    fn custom(&self, role: CustomRole) -> impl Iterator<Item = &CustomSource> {
        self.custom.iter().filter(move |c| {
            c.role == role && c.validate().is_ok() && self.switched.get(&c.id).copied().unwrap_or(true)
        })
    }
}

fn active(setup: &Setup) -> impl Iterator<Item = &'static SourceInfo> + '_ {
    CATALOG.iter().filter(|s| setup.is_on(s))
}

/// Quote, search and listing sources with the defaults: on by default, no keys.
pub fn quote_service() -> MarketDataService {
    quote_service_with(&Setup::default())
}

/// Quote, search and listing sources that are on under `setup`, in one service.
pub fn quote_service_with(setup: &Setup) -> MarketDataService {
    let mut service = MarketDataService::new();
    for source in active(setup) {
        if let Some(make) = source.quotes {
            service.register(make(setup.key(source)));
        }
        if let Some(make) = source.search {
            service.register_search(make(setup.key(source)));
        }
        if let Some(make) = source.listings {
            service.register_directory(make(setup.key(source)));
        }
    }
    for source in active(setup) {
        if let (Some(limit), Some(_)) = (source.per_day, source.quotes) {
            service.set_budget(source.id, limit);
        }
    }
    for custom in setup.custom(CustomRole::Quotes) {
        service.register(Box::new(CustomProvider::new(
            custom.clone(),
            setup.keys.get(&custom.id).cloned(),
        )));
    }
    service
}

/// Quote sources that are on, `DEFAULT_QUOTES` first — the order a picker offers them in.
pub fn quote_ids(setup: &Setup) -> Vec<String> {
    let (first, rest): (Vec<_>, Vec<_>) = active(setup)
        .filter(|s| s.quotes.is_some())
        .partition(|s| s.id == DEFAULT_QUOTES);
    first
        .into_iter()
        .chain(rest)
        .map(|s| s.id.to_string())
        .chain(setup.custom(CustomRole::Quotes).map(|c| c.id.clone()))
        .collect()
}

/// The order a currency pair is asked in: the central bank, its mirror, then the market's close.
/// A source missing here is asked after these, in catalogue order.
const FX_ORDER: &[&str] = &[EcbProvider::ID, FrankfurterProvider::ID, YahooProvider::ID];

/// FX sources with the defaults.
pub fn fx_service() -> FxService {
    fx_service_with(&Setup::default())
}

/// FX sources that are on under `setup`, as one chain in `FX_ORDER`.
pub fn fx_service_with(setup: &Setup) -> FxService {
    let rank = |id: &str| FX_ORDER.iter().position(|o| *o == id).unwrap_or(FX_ORDER.len());
    let mut rows: Vec<&SourceInfo> = active(setup).filter(|s| s.fx.is_some()).collect();
    rows.sort_by_key(|s| rank(s.id));
    let mut service = FxService::new();
    for source in rows {
        if let Some(make) = source.fx {
            service.register(make(setup.key(source)));
            if let Some(limit) = source.per_day {
                service.set_budget(source.id, limit);
            }
        }
    }
    // The user's own rate sources come last: a shipped source that covers the pair answers first.
    for custom in setup.custom(CustomRole::Fx) {
        service.register(Box::new(CustomProvider::new(
            custom.clone(),
            setup.keys.get(&custom.id).cloned(),
        )));
    }
    service
}

/// The order a region is asked in: the harmonised European index, then the worldwide one.
/// Eurostat is first where it publishes at all — it carries the euro-area aggregate, which the
/// IMF does not, and it publishes sooner.
const INDEX_ORDER: &[&str] = &[EurostatProvider::ID, ImfProvider::ID];

/// Consumer-price sources with the defaults.
pub fn index_service() -> InflationService {
    index_service_with(&Setup::default())
}

/// Consumer-price sources that are on under `setup`, as one chain in `INDEX_ORDER`.
pub fn index_service_with(setup: &Setup) -> InflationService {
    let rank = |id: &str| {
        INDEX_ORDER
            .iter()
            .position(|o| *o == id)
            .unwrap_or(INDEX_ORDER.len())
    };
    let mut rows: Vec<&SourceInfo> = active(setup).filter(|s| s.index.is_some()).collect();
    rows.sort_by_key(|s| rank(s.id));
    let mut service = InflationService::new();
    for source in rows {
        if let Some(make) = source.index {
            service.register(make(setup.key(source)));
        }
    }
    service
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<&str> = CATALOG.iter().map(|s| s.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), CATALOG.len());
    }

    #[test]
    fn every_constructor_builds_the_source_its_row_names() {
        for source in CATALOG {
            let built: Vec<&str> = [
                source.quotes.map(|make| make(Some("k".into())).id()),
                source.search.map(|make| make(Some("k".into())).id()),
                source.listings.map(|make| make(Some("k".into())).id()),
                source.fx.map(|make| make(Some("k".into())).id()),
                source.index.map(|make| make(Some("k".into())).id()),
            ]
            .into_iter()
            .flatten()
            .collect();
            assert!(!built.is_empty(), "{} plays no role", source.id);
            assert!(
                built.iter().all(|id| *id == source.id),
                "{}: {built:?}",
                source.id
            );
        }
    }

    #[test]
    fn the_defaults_are_on_and_can_answer() {
        let quotes = info(DEFAULT_QUOTES).unwrap();
        assert!(quotes.on_by_default && quotes.capabilities().contains(&Capability::Quotes));
        let fx = info(DEFAULT_FX).unwrap();
        assert!(fx.on_by_default && fx.capabilities().contains(&Capability::FxRates));
    }

    #[test]
    fn services_register_only_what_is_on() {
        assert_eq!(
            quote_service().provider_ids(),
            vec![KrakenProvider::ID, YahooProvider::ID]
        );
    }

    #[test]
    fn the_central_bank_is_asked_before_the_market() {
        assert_eq!(
            fx_service().provider_ids(),
            vec![EcbProvider::ID, FrankfurterProvider::ID, YahooProvider::ID]
        );
    }

    #[test]
    fn the_harmonised_index_is_asked_before_the_worldwide_one() {
        assert_eq!(
            index_service().provider_ids(),
            vec![EurostatProvider::ID, ImfProvider::ID]
        );
    }

    #[test]
    fn a_source_that_needs_a_key_is_on_only_once_it_has_one() {
        assert!(
            !quote_ids(&Setup::default())
                .iter()
                .any(|id| id == TwelveDataProvider::ID)
        );
        let mut setup = Setup::default();
        setup.keys.insert(TwelveDataProvider::ID.into(), "k".into());
        assert!(quote_ids(&setup).iter().any(|id| id == TwelveDataProvider::ID));
        assert_eq!(
            fx_service_with(&setup).provider_ids().last(),
            Some(&TwelveDataProvider::ID)
        );
    }

    #[test]
    fn a_custom_source_is_offered_after_the_shipped_ones() {
        let mut setup = Setup::default();
        setup.custom.push(CustomSource {
            id: "custom:bank".into(),
            label: "Bank".into(),
            role: CustomRole::Quotes,
            url: "https://bank.example/{SYMBOL}.json".into(),
            headers: vec![],
            format: crate::market::CustomFormat::Json {
                date_path: "$[*].d".into(),
                close_path: "$[*].c".into(),
            },
            date_format: None,
            factor: None,
            currency: None,
        });
        assert_eq!(quote_ids(&setup).last().map(String::as_str), Some("custom:bank"));
        assert!(quote_service_with(&setup).provider("custom:bank").is_some());

        // The same source as a rate source joins the FX chain last and leaves the quote list.
        setup.custom[0].role = CustomRole::Fx;
        assert_eq!(
            fx_service_with(&setup).provider_ids().last(),
            Some(&"custom:bank")
        );
        assert!(!quote_ids(&setup).iter().any(|id| id == "custom:bank"));
    }

    #[test]
    fn the_user_s_switch_outranks_the_default() {
        let mut setup = Setup::default();
        setup.switched.insert(KrakenProvider::ID.into(), false);
        setup.switched.insert(StooqProvider::ID.into(), true);
        assert_eq!(quote_ids(&setup), vec![YahooProvider::ID, StooqProvider::ID]);
    }
}
