//! Background refresh of market quotes, FX rates and the price index.

mod coverage;
mod relist;
mod run;
mod status;
mod window;

pub use coverage::*;
pub use status::{Failure, FailureCause, FailureCode, Progress, RefreshMode, RefreshStatus};
pub use window::sparse_history;

use crate::error::UiResult;
use crate::events::emit_changed;
use crate::state::AppState;
use chrono::Utc;
use sq_core::sources;
use status::Outcome;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};

/// Quote source ids for the security form, the default first.
#[tauri::command]
pub fn quote_providers(state: State<AppState>) -> Vec<String> {
    sources::quote_ids(&state.market_setup())
}

/// Starts a refresh unless one is already running.
#[tauri::command]
pub fn market_refresh(app: AppHandle, state: State<AppState>, mode: RefreshMode) -> UiResult<bool> {
    Ok(start(&app, &state, mode))
}

/// Cancels between network requests.
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
    // Sources not chosen yet: nobody to ask, and that is not a failure (ADR-0076).
    if !state.settings().map(|s| s.sources_configured).unwrap_or(false) {
        return false;
    }
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

    // A locked profile has no key to give.
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
            run::run(&handle, &access, &setup, base, region, mode)
        });
        let outcome = std::panic::catch_unwind(guarded).unwrap_or_else(|_| Outcome {
            failed: vec![Failure {
                code: FailureCode::Internal,
                cause: FailureCause::Other,
                subject: String::new(),
                detail: "the refresh thread panicked".into(),
                source: None,
            }],
            ..Outcome::default()
        });
        finish(&handle, &access.path, mode, outcome);
    });
    true
}

/// Fetches what no refresh has fetched yet. Queued while a refresh runs: that one listed its
/// instruments before this one existed.
pub fn fetch_missing(app: &AppHandle, state: &AppState) {
    if let Ok(mut status) = state.refresh()
        && status.running
    {
        status.queued = true;
        return;
    }
    start(app, state, RefreshMode::Missing);
}

fn finish(app: &AppHandle, db_path: &std::path::Path, mode: RefreshMode, outcome: Outcome) {
    let Outcome {
        fetched,
        failed,
        cancelled,
        relisted,
    } = outcome;
    let now = Utc::now().to_rfc3339();
    // A missing-data fetch is not a refresh: the last refresh's time and failures stand unless
    // this fetch failed itself.
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
        // Only into the profile it ran for: the one in memory may have been switched since.
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
    // A relisted instrument changed its ticker and currency, not only its quotes.
    if relisted > 0 {
        let _ = emit_changed(app, "securities");
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

fn cancelled(app: &AppHandle) -> bool {
    app.try_state::<AppState>()
        .map(|state| state.cancel_refresh.load(Ordering::Relaxed))
        .unwrap_or(false)
}
