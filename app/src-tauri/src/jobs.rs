//! Background refresh of market quotes and FX rates.

use crate::error::UiResult;
use crate::events::emit_changed;
use crate::state::AppState;
use chrono::{Duration, Months, NaiveDate, Utc};
use serde::Serialize;
use sq_core::prelude::*;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};

/// Quote source IDs for the security form, the default first.
#[tauri::command]
pub fn quote_providers(state: State<AppState>) -> Vec<String> {
    sources::quote_ids(&state.market_setup())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshMode {
    /// Refresh the recent tail, including a three-day correction window.
    CatchUp,
    /// Refresh up to five years of history.
    Full,
    /// Fetch only what nothing was fetched for yet: a new instrument, a new currency.
    Missing,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Progress {
    Started {
        total: usize,
    },
    Item {
        label: String,
        done: usize,
        total: usize,
        fetched: usize,
    },
    Finished {
        fetched: usize,
        failed: Vec<Failure>,
        /// Whether the refresh was cancelled.
        cancelled: bool,
    },
}

/// Why a refresh step failed. A code, not a sentence: the UI writes the headline.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    /// The database could not be opened.
    Database,
    /// The security list could not be read.
    Securities,
    /// The currency list could not be read.
    Currencies,
    /// One security's quotes did not arrive.
    Quote,
    /// One currency pair's rates did not arrive.
    Rate,
    /// The consumer-price index of the portfolio's region did not arrive.
    Index,
    /// Securities whose ticker field holds an ISIN; a provider cannot answer for them.
    Unidentified,
    /// The refresh thread panicked.
    Internal,
}

/// What went wrong underneath, so the UI can say what to fix rather than repeat the English detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCause {
    /// No connection, a timeout, throttling or a server error: trying later is the fix.
    Unreachable,
    /// The provider refused the request (4xx): usually a symbol it does not know.
    Rejected,
    /// The provider refused the key (401/403): a missing, wrong or expired key.
    Unauthorized,
    /// The provider answered without usable prices: a wrong, delisted or unsupported ticker.
    NoData,
    /// The local database failed.
    Storage,
    Other,
}

impl From<&Error> for FailureCause {
    fn from(e: &Error) -> Self {
        match e {
            Error::Unavailable(_) | Error::RateLimited(_) => FailureCause::Unreachable,
            Error::Network(_) => FailureCause::Rejected,
            Error::Unauthorized(_) => FailureCause::Unauthorized,
            Error::BadProviderData { .. } | Error::MissingMarketData { .. } => FailureCause::NoData,
            Error::Storage(_) => FailureCause::Storage,
            _ => FailureCause::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Failure {
    pub code: FailureCode,
    pub cause: FailureCause,
    /// What the failure is about: a symbol, a currency pair, a comma-separated list.
    pub subject: String,
    /// English detail from the underlying error; the UI shows it under its own headline.
    pub detail: String,
    /// The source that was asked and failed, when one was: the instrument's own, or the first
    /// that covers the pair. `None` when no source was reached at all.
    pub source: Option<String>,
}

/// Snapshot for screens opened while a refresh is running.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RefreshStatus {
    pub running: bool,
    pub label: Option<String>,
    pub done: usize,
    pub total: usize,
    pub fetched: usize,
    /// Last refresh completion time in RFC 3339.
    pub last_finished: Option<String>,
    pub failures: Vec<Failure>,
    /// Whether the last refresh was cancelled.
    pub cancelled: bool,
    /// A missing-data fetch asked for while this refresh ran; it starts when this one ends.
    #[serde(skip)]
    pub queued: bool,
}

/// Start a refresh unless one is already running.
#[tauri::command]
pub fn market_refresh(app: AppHandle, state: State<AppState>, mode: RefreshMode) -> UiResult<bool> {
    Ok(start(&app, &state, mode))
}

/// Request cancellation between network requests.
#[tauri::command]
pub fn market_refresh_cancel(state: State<AppState>) -> UiResult<()> {
    state.cancel_refresh.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn refresh_status(state: State<AppState>) -> UiResult<RefreshStatus> {
    Ok(state.refresh()?.clone())
}

/// Shared entry point for manual and startup refreshes.
pub fn start(app: &AppHandle, state: &AppState, mode: RefreshMode) -> bool {
    {
        let Ok(mut status) = state.refresh() else {
            return false;
        };
        if status.running {
            return false;
        }
        *status = RefreshStatus {
            running: true,
            cancelled: false,
            ..status.clone()
        };
    }
    state.cancel_refresh.store(false, Ordering::Relaxed);

    // A locked profile has no key to give, and nothing of it is refreshed until it is unlocked.
    let Ok(access) = state.db_access() else {
        if let Ok(mut status) = state.refresh() {
            status.running = false;
        }
        return false;
    };
    let (base, region) = match state.portfolio() {
        Ok(p) => (p.base_currency.clone(), p.inflation_region.clone()),
        Err(_) => return false,
    };
    let handle = app.clone();
    let setup = state.market_setup();

    std::thread::spawn(move || {
        let guarded = std::panic::AssertUnwindSafe(|| {
            let state = handle.state::<AppState>();
            let _in_use = state.db_in_use();
            run(&handle, &access, &setup, base, region, mode)
        });
        let outcome = std::panic::catch_unwind(guarded).unwrap_or_else(|_| Outcome {
            fetched: 0,
            failed: vec![Failure {
                code: FailureCode::Internal,
                cause: FailureCause::Other,
                subject: String::new(),
                detail: "the refresh thread panicked".into(),
                source: None,
            }],
            cancelled: false,
        });
        finish(&handle, &access.path, mode, outcome);
    });
    true
}

/// Fetches what no refresh has fetched yet — a new instrument, a new currency. While a refresh
/// runs it is queued instead: that refresh listed its instruments before this one existed.
pub fn fetch_missing(app: &AppHandle, state: &AppState) {
    if let Ok(mut status) = state.refresh()
        && status.running
    {
        status.queued = true;
        return;
    }
    start(app, state, RefreshMode::Missing);
}

/// Refresh outcome passed to the finalizer.
struct Outcome {
    fetched: usize,
    failed: Vec<Failure>,
    cancelled: bool,
}

fn finish(app: &AppHandle, db_path: &std::path::Path, mode: RefreshMode, outcome: Outcome) {
    let Outcome {
        fetched,
        failed,
        cancelled,
    } = outcome;
    let now = Utc::now().to_rfc3339();
    // Fetching one new instrument is not a refresh of the portfolio: the time of the last refresh
    // and its failures stay what the last real refresh left, unless this fetch failed itself.
    let partial = mode == RefreshMode::Missing;
    let mut queued = false;

    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut status) = state.refresh() {
            queued = status.queued;
            *status = RefreshStatus {
                running: false,
                label: None,
                done: 0,
                total: 0,
                fetched,
                last_finished: if partial {
                    status.last_finished.clone()
                } else {
                    Some(now.clone())
                },
                failures: if partial && failed.is_empty() {
                    status.failures.clone()
                } else {
                    failed.clone()
                },
                cancelled,
                queued: false,
            };
        }
        // A refresh outlives a profile switch: its time belongs to the profile it ran for, and
        // the settings in memory are by then another profile's.
        if !cancelled
            && !partial
            && state.db_path().is_ok_and(|open| open == db_path)
            && let Ok(mut settings) = state.settings()
        {
            settings.last_refresh = Some(now);
            let _ = crate::settings::store(db_path, &settings);
        }
    }

    let _ = app.emit(
        "market:progress",
        Progress::Finished {
            fetched,
            failed,
            cancelled,
        },
    );
    if !partial || fetched > 0 {
        let _ = emit_changed(app, "quotes");
    }
    if queued && let Some(state) = app.try_state::<AppState>() {
        start(app, &state, RefreshMode::Missing);
    }
}

fn report(app: &AppHandle, progress: Progress) {
    if let Some(state) = app.try_state::<AppState>()
        && let Ok(mut status) = state.refresh()
    {
        match &progress {
            Progress::Started { total } => {
                status.total = *total;
                status.done = 0;
                status.fetched = 0;
            }
            Progress::Item {
                label,
                done,
                total,
                fetched,
            } => {
                status.label = Some(label.clone());
                status.done = *done;
                status.total = *total;
                status.fetched = *fetched;
            }
            Progress::Finished { .. } => {}
        }
    }
    let _ = app.emit("market:progress", progress);
}

/// Check whether cancellation was requested.
fn cancelled(app: &AppHandle) -> bool {
    app.try_state::<AppState>()
        .map(|state| state.cancel_refresh.load(Ordering::Relaxed))
        .unwrap_or(false)
}

fn fatal(code: FailureCode, e: Error) -> Outcome {
    Outcome {
        fetched: 0,
        failed: vec![Failure {
            code,
            cause: FailureCause::from(&e),
            subject: String::new(),
            detail: e.to_string(),
            source: None,
        }],
        cancelled: false,
    }
}

fn run(
    app: &AppHandle,
    access: &crate::state::DbAccess,
    setup: &sources::Setup,
    base: String,
    region: Option<String>,
    mode: RefreshMode,
) -> Outcome {
    let mut fetched = 0usize;
    let mut failed = Vec::new();

    let store = match access.open() {
        Ok(store) => store,
        Err(e) => return fatal(FailureCode::Database, e),
    };

    let today = Utc::now().date_naive();
    let settled_through = today - Duration::days(1);
    let quotes = sources::quote_service_with(setup);
    let rates = sources::fx_service_with(setup);

    let (mut securities, needs_lookup): (Vec<Security>, Vec<Security>) = match store.list_securities() {
        Ok(list) => list
            .into_iter()
            .filter(|s| s.data_source.is_some())
            .partition(Security::is_quotable),
        Err(e) => return fatal(FailureCode::Securities, e),
    };
    let missing = mode == RefreshMode::Missing;
    if missing {
        // Fetched means both coverages hold a range; anything less is fetched in full.
        securities.retain(|s| {
            !matches!(
                (store.quote_coverage(&s.id), store.event_coverage(&s.id)),
                (Ok(Some(_)), Ok(Some(_)))
            )
        });
    }
    if !needs_lookup.is_empty() && !missing {
        let symbols: Vec<&str> = needs_lookup.iter().map(|s| s.symbol.as_str()).collect();
        failed.push(Failure {
            code: FailureCode::Unidentified,
            cause: FailureCause::Other,
            subject: symbols.join(", "),
            detail: "the ticker field holds an ISIN, not a provider symbol".into(),
            source: None,
        });
    }

    let mut currencies = match store.distinct_currencies() {
        Ok(set) => set,
        Err(e) => return fatal(FailureCode::Currencies, e),
    };
    currencies.remove(&base);
    let mut pairs: Vec<String> = currencies.into_iter().collect();
    if missing {
        pairs.retain(|c| {
            store
                .rate_series(c, &base, today)
                .map(|series| series.is_empty())
                .unwrap_or(true)
        });
    }

    let total = securities.len() + pairs.len();
    if missing && total == 0 {
        return Outcome {
            fetched,
            failed,
            cancelled: false,
        };
    }
    report(app, Progress::Started { total });
    let mut done = 0usize;

    for security in &securities {
        if cancelled(app) {
            return Outcome {
                fetched,
                failed,
                cancelled: true,
            };
        }
        // Quotes fetched before events were asked for: backfill the full window once.
        let known = match store.event_coverage(&security.id) {
            Ok(Some(_)) => store.latest_quote_date(&security.id).ok().flatten(),
            _ => None,
        };
        let range = DateRange::new(since(mode, known, today), today);
        match quotes.ensure_history_through(&store, security, range, settled_through) {
            Ok(saved) => fetched += saved,
            Err(e) => failed.push(Failure {
                code: FailureCode::Quote,
                cause: FailureCause::from(&e),
                subject: security.symbol.clone(),
                detail: e.to_string(),
                source: security.data_source.clone(),
            }),
        }
        done += 1;
        report(
            app,
            Progress::Item {
                label: security.symbol.clone(),
                done,
                total,
                fetched,
            },
        );
    }

    for currency in &pairs {
        if cancelled(app) {
            return Outcome {
                fetched,
                failed,
                cancelled: true,
            };
        }
        let known = store
            .rate_series(currency, &base, today)
            .ok()
            .and_then(|series| series.keys().next_back().copied());
        let range = DateRange::new(since(mode, known, today), today);
        match rates.ensure_rates(&store, currency, &base, range) {
            Ok((saved, _)) => fetched += saved,
            Err(e) => failed.push(Failure {
                code: FailureCode::Rate,
                cause: FailureCause::from(&e),
                subject: format!("{currency}/{base}"),
                detail: e.to_string(),
                source: rates.first_for(currency, &base).map(str::to_string),
            }),
        }
        done += 1;
        report(
            app,
            Progress::Item {
                label: format!("{currency}/{base}"),
                done,
                total,
                fetched,
            },
        );
    }

    if let Some(region) = region.as_deref()
        && !cancelled(app)
        && let Some(failure) = refresh_index(&store, setup, region, mode, today, &mut fetched)
    {
        failed.push(failure);
    }

    // Log crossings while this job's own connection is open; the UI announces them after `finish`.
    let _ = crate::commands::alerts::check_alerts(&store, crate::commands::alerts::today());

    Outcome {
        fetched,
        failed,
        cancelled: false,
    }
}

/// Fetches the region's consumer-price index when the stored series does not already reach
/// last month. It is asked once a month rather than every refresh: an index is published
/// monthly, so a daily ask spends a request to be told the same figure.
fn refresh_index(
    store: &sq_core::storage::Store,
    setup: &sources::Setup,
    region: &str,
    mode: RefreshMode,
    today: NaiveDate,
    fetched: &mut usize,
) -> Option<Failure> {
    use sq_core::inflation::IndexLookup;

    let published_through = store.index_through(region).ok().flatten();
    // A month's level lands weeks after that month ends, so holding last month's is as current
    // as this source ever gets. Both sides are a month's first day — comparing one against the
    // end of a month would never be satisfied, and the index would be re-asked every refresh.
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
        Err(e) => Some(Failure {
            code: FailureCode::Index,
            cause: FailureCause::from(&e),
            subject: region.to_string(),
            detail: e.to_string(),
            source: service.first_for(region).map(str::to_string),
        }),
    }
}

/// Select the refresh start date, retaining a three-day correction window.
fn since(mode: RefreshMode, known: Option<NaiveDate>, today: NaiveDate) -> NaiveDate {
    let full = today - Duration::days(365 * 5);
    match (mode, known) {
        (RefreshMode::Full, _) | (_, None) => full,
        (_, Some(last)) => (last - Duration::days(3)).max(full),
    }
}

/// Market-data coverage exposed to the frontend.
#[derive(Debug, Serialize)]
pub struct DataCoverage {
    pub security_id: String,
    pub symbol: String,
    pub data_source: Option<String>,
    pub last_quote: Option<String>,
    pub first_quote: Option<String>,
    pub quote_count: usize,
    /// Calendar days since the last quote, or `null` when empty.
    pub stale_days: Option<i64>,
    /// Currency of the latest quote.
    pub quote_currency: Option<String>,
    pub mic: Option<String>,
    /// Venue name resolved from the ISO 10383 MIC.
    pub venue: Option<String>,
}

#[tauri::command]
pub fn data_coverage(state: State<AppState>) -> UiResult<Vec<DataCoverage>> {
    let store = state.store()?;
    let stats = store.quote_stats()?;
    let today = chrono::Local::now().date_naive();

    Ok(store
        .list_securities()?
        .into_iter()
        .map(|s| {
            let stat = stats.get(&s.id);
            DataCoverage {
                last_quote: stat.map(|q| q.range.to.to_string()),
                first_quote: stat.map(|q| q.range.from.to_string()),
                quote_count: stat.map(|q| q.count).unwrap_or(0),
                stale_days: stat.map(|q| (today - q.range.to).num_days()),
                quote_currency: stat.map(|q| q.currency.clone()),
                venue: s
                    .mic
                    .as_deref()
                    .and_then(sq_core::market::mic::market_name)
                    .map(str::to_string),
                mic: s.mic,
                security_id: s.id,
                symbol: s.symbol,
                data_source: s.data_source,
            }
        })
        .collect())
}
