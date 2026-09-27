//! One refresh pass on its own connection: quotes, then FX, then the price index.

use super::relist::relist;
use super::status::{Failure, FailureCause, FailureCode, Outcome, Progress, RefreshMode};
use super::window::{asked_from, since, sparse_history, start_from, stored_rates_from};
use super::{cancelled, report};
use chrono::{Duration, Months, NaiveDate, Utc};
use sq_core::fx::FxService;
use sq_core::market::MarketDataService;
use sq_core::prelude::*;
use sq_core::storage::{HistoryNeed, Store};
use tauri::AppHandle;

pub(super) fn run(
    app: &AppHandle,
    access: &crate::state::DbAccess,
    setup: &sources::Setup,
    base: String,
    region: Option<String>,
    mode: RefreshMode,
) -> Outcome {
    let store = match access.open() {
        Ok(store) => store,
        Err(e) => return Outcome::fatal(FailureCode::Database, e),
    };
    let today = Utc::now().date_naive();
    let quotes = sources::quote_service_with(setup);
    let rates = sources::fx_service_with(setup);
    let missing = mode == RefreshMode::Missing;

    let (mut securities, needs_lookup): (Vec<Security>, Vec<Security>) = match store.list_securities() {
        Ok(list) => list
            .into_iter()
            .filter(|s| s.data_source.is_some())
            .partition(Security::is_quotable),
        Err(e) => return Outcome::fatal(FailureCode::Securities, e),
    };
    // How far back the ledger needs data: a window short of the first operation is a hole too.
    let need = store.history_need().unwrap_or_default();
    if missing {
        // Not fetched yet also means: not reaching the first operation, or no series to show.
        securities.retain(|s| {
            let held = need.securities.get(&s.id).copied();
            match asked_from(&store, &s.id) {
                None => true,
                Some(from) => {
                    held.is_some_and(|first| first < from)
                        || sparse_history(held, store.quote_span(&s.id).ok().flatten(), today)
                }
            }
        });
    }
    let mut out = Outcome::default();
    if !needs_lookup.is_empty() && !missing {
        let symbols: Vec<&str> = needs_lookup.iter().map(|s| s.symbol.as_str()).collect();
        out.failed.push(Failure {
            code: FailureCode::Unidentified,
            cause: FailureCause::Other,
            subject: symbols.join(", "),
            detail: "the ticker field holds an ISIN, not a provider symbol".into(),
            source: None,
        });
    }

    let mut currencies = match store.distinct_currencies() {
        Ok(set) => set,
        Err(e) => return Outcome::fatal(FailureCode::Currencies, e),
    };
    currencies.remove(&base);
    let mut pairs: Vec<String> = currencies.into_iter().collect();
    if missing {
        pairs.retain(|c| match stored_rates_from(&store, c, &base, today) {
            None => true,
            Some(from) => need.currencies.get(c).is_some_and(|first| *first < from),
        });
    }

    let total = securities.len() + pairs.len();
    if missing && total == 0 {
        return out;
    }
    report(app, Progress::Started { total });
    let mut pass = Pass {
        app,
        store: &store,
        need: &need,
        mode,
        today,
        total,
        done: 0,
        out,
    };

    for security in &securities {
        if cancelled(app) {
            return pass.cancel();
        }
        pass.security(&quotes, security);
    }
    for currency in &pairs {
        if cancelled(app) {
            return pass.cancel();
        }
        pass.pair(&rates, currency, &base);
    }
    let mut out = pass.out;

    if let Some(region) = region.as_deref()
        && !cancelled(app)
        && let Some(failure) = refresh_index(&store, setup, region, mode, today, &mut out.fetched)
    {
        out.failed.push(failure);
    }

    // Logged on this connection; the UI announces them after `finish`.
    let _ = crate::commands::alerts::check_alerts(&store, crate::commands::alerts::today());
    out
}

/// The per-item half of a run, counting progress as it goes.
struct Pass<'a> {
    app: &'a AppHandle,
    store: &'a Store,
    need: &'a HistoryNeed,
    mode: RefreshMode,
    today: NaiveDate,
    total: usize,
    done: usize,
    out: Outcome,
}

impl Pass<'_> {
    fn cancel(mut self) -> Outcome {
        self.out.cancelled = true;
        self.out
    }

    fn step(&mut self, label: String) {
        self.done += 1;
        report(
            self.app,
            Progress::Item {
                label,
                done: self.done,
                total: self.total,
                fetched: self.out.fetched,
            },
        );
    }

    fn security(&mut self, quotes: &MarketDataService, security: &Security) {
        let (store, today) = (self.store, self.today);
        let settled_through = today - Duration::days(1);
        // Quotes fetched before events were asked for: backfill the full window once.
        let known = match store.event_coverage(&security.id) {
            Ok(Some(_)) => store.latest_quote_date(&security.id).ok().flatten(),
            _ => None,
        };
        let held = self.need.securities.get(&security.id).copied();
        let from = start_from(self.mode, known, today, asked_from(store, &security.id), held);
        let range = DateRange::new(from, today);
        match quotes.ensure_history_through(store, security, range, settled_through) {
            Ok(saved) => {
                self.out.fetched += saved;
                // Then the days its own source has not published yet (ADR-0079).
                match quotes.ensure_latest(store, security, today) {
                    Ok(saved) => self.out.fetched += saved,
                    Err(e) => {
                        let source = store.latest_symbol(&security.id).ok().flatten().map(|(s, _)| s);
                        self.out.failed.push(Failure::new(
                            FailureCode::Quote,
                            &e,
                            security.symbol.clone(),
                            source,
                        ));
                    }
                }
            }
            Err(e) => self.out.failed.push(Failure::new(
                FailureCode::Quote,
                &e,
                security.symbol.clone(),
                security.data_source.clone(),
            )),
        }
        // A file names a ticker, not a venue, so a first fetch may land where nothing is quoted.
        if self.mode == RefreshMode::Missing
            && let Some(saved) = relist(store, quotes, security, held, today, settled_through)
        {
            self.out.fetched += saved;
            self.out.relisted += 1;
        }
        self.step(security.symbol.clone());
    }

    fn pair(&mut self, rates: &FxService, currency: &str, base: &str) {
        let (store, today) = (self.store, self.today);
        let known = store
            .rate_series(currency, base, today)
            .ok()
            .and_then(|series| series.keys().next_back().copied());
        let from = start_from(
            self.mode,
            known,
            today,
            stored_rates_from(store, currency, base, today),
            self.need.currencies.get(currency).copied(),
        );
        let range = DateRange::new(from, today);
        let subject = format!("{currency}/{base}");
        match rates.ensure_rates(store, currency, base, range) {
            Ok((saved, _)) => self.out.fetched += saved,
            Err(e) => {
                let source = rates.first_for(currency, base).map(str::to_string);
                self.out
                    .failed
                    .push(Failure::new(FailureCode::Rate, &e, subject.clone(), source));
            }
        }
        self.step(subject);
    }
}

/// Asked only when the stored index does not reach last month: it is published monthly.
fn refresh_index(
    store: &Store,
    setup: &sources::Setup,
    region: &str,
    mode: RefreshMode,
    today: NaiveDate,
    fetched: &mut usize,
) -> Option<Failure> {
    use sq_core::inflation::IndexLookup;

    let published_through = store.index_through(region).ok().flatten();
    // Both sides are a month's first day; against a month's end this would never hold.
    let current = sq_core::inflation::first_of_month(today) - Months::new(1);
    if mode != RefreshMode::Full && published_through.is_some_and(|month| month >= current) {
        return None;
    }
    let service = sources::index_service_with(setup);
    let range = DateRange::new(since(mode, published_through, today), today);
    match service.ensure_index(store, region, range) {
        Ok((saved, _)) => {
            *fetched += saved;
            None
        }
        Err(e) => Some(Failure::new(
            FailureCode::Index,
            &e,
            region.to_string(),
            service.first_for(region).map(str::to_string),
        )),
    }
}
