//! The crate's single error type.

use thiserror::Error;

/// One enum for all of core's errors — callers (UI, CLI) almost always just
/// want to display it, and `thiserror` gives `Display`/`From` for free.
#[derive(Debug, Error)]
pub enum Error {
    #[error("storage error: {0}")]
    Storage(#[from] rusqlite::Error),

    #[error("network error: {0}")]
    Network(String),

    /// A likely-transient failure (timeout, dropped connection, 429, 5xx),
    /// kept separate from [`Error::Network`] so retry policy can switch on
    /// the error type instead of parsing the message.
    #[error("temporarily unavailable: {0}")]
    Unavailable(String),

    /// The source is throttling us (429). Transient like [`Error::Unavailable`], but told apart
    /// so a caller can rest that source and ask another instead of waiting it out.
    #[error("rate limited: {0}")]
    RateLimited(String),

    /// The source refused the credentials (401/403): a missing or wrong key. Retrying cannot help.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("provider `{provider}` returned unusable data: {detail}")]
    BadProviderData { provider: &'static str, detail: String },

    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid data: {0}")]
    Invalid(String),

    /// Missing price/rate is never a silent `None` — see `money-and-fx.md`.
    #[error("no {kind} for `{key}` on or before {date}")]
    MissingMarketData {
        kind: &'static str,
        key: String,
        date: chrono::NaiveDate,
    },

    #[error("math error: {0}")]
    Math(String),

    /// The copy taken before a schema upgrade could not be written, so the upgrade did not run.
    /// A migration cannot be undone, and refusing to start is recoverable where a lost portfolio
    /// is not.
    #[error("could not back up the database before migrating: {0}")]
    Backup(String),
}

impl Error {
    /// Whether retrying is worth it. Sole consumer: [`crate::market::FetchPolicy`].
    pub fn is_transient(&self) -> bool {
        matches!(self, Error::Unavailable(_) | Error::RateLimited(_))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        let text = e.to_string();
        match e {
            // 429 (Yahoo throttling a long list) and 5xx both clear up after a pause.
            ureq::Error::StatusCode(429) => Error::RateLimited(text),
            ureq::Error::StatusCode(401 | 403) => Error::Unauthorized(text),
            ureq::Error::StatusCode(500..=599) => Error::Unavailable(text),
            ureq::Error::Timeout(_)
            | ureq::Error::Io(_)
            | ureq::Error::ConnectionFailed
            | ureq::Error::HostNotFound => Error::Unavailable(text),
            _ => Error::Network(text),
        }
    }
}

// io::Error only ever arrives here from reading an HTTP response body.
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Unavailable(e.to_string())
    }
}
