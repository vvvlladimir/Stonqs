//! What a refresh reports: progress events, failures as codes, the status snapshot.

use serde::Serialize;
use sq_core::prelude::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefreshMode {
    /// The recent tail, with a three-day correction window.
    CatchUp,
    /// Up to five years of history.
    Full,
    /// Only what nothing was fetched for yet: a new instrument, a new currency.
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
        cancelled: bool,
    },
}

/// Which step failed. A code, not a sentence: the UI writes the headline.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    Database,
    Securities,
    Currencies,
    /// One security's quotes.
    Quote,
    /// One currency pair's rates.
    Rate,
    /// The portfolio region's consumer-price index.
    Index,
    /// Securities whose ticker field holds an ISIN.
    Unidentified,
    /// The refresh thread panicked.
    Internal,
}

/// What went wrong underneath, so the UI can say what to fix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCause {
    /// Offline, timeout, throttling or a server error: try later.
    Unreachable,
    /// A 4xx: usually a symbol the provider does not know.
    Rejected,
    /// 401/403: a missing, wrong or expired key.
    Unauthorized,
    /// An answer without usable prices: wrong, delisted or unsupported ticker.
    NoData,
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
    /// A symbol, a currency pair, or a comma-separated list.
    pub subject: String,
    /// English detail, shown under the UI's own headline.
    pub detail: String,
    /// The source asked, when one was reached.
    pub source: Option<String>,
}

impl Failure {
    pub(super) fn new(code: FailureCode, e: &Error, subject: String, source: Option<String>) -> Self {
        Failure {
            code,
            cause: FailureCause::from(e),
            subject,
            detail: e.to_string(),
            source,
        }
    }
}

/// Snapshot for a screen opened while a refresh runs.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RefreshStatus {
    pub running: bool,
    pub label: Option<String>,
    pub done: usize,
    pub total: usize,
    pub fetched: usize,
    /// RFC 3339.
    pub last_finished: Option<String>,
    pub failures: Vec<Failure>,
    pub cancelled: bool,
    /// A missing-data fetch asked for meanwhile; it starts when this refresh ends.
    #[serde(skip)]
    pub queued: bool,
}

/// What a run hands to `finish`.
#[derive(Default)]
pub(super) struct Outcome {
    pub fetched: usize,
    pub failed: Vec<Failure>,
    pub cancelled: bool,
    /// Instruments moved to another venue by this run.
    pub relisted: usize,
}

impl Outcome {
    pub(super) fn fatal(code: FailureCode, e: Error) -> Self {
        Outcome {
            failed: vec![Failure::new(code, &e, String::new(), None)],
            ..Outcome::default()
        }
    }
}
