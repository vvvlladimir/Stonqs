use super::Quote;
use crate::error::Result;
use crate::model::{Security, SecurityEvent};
use chrono::NaiveDate;
/// Inclusive date range passed to quote providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    pub from: NaiveDate,
    pub to: NaiveDate,
}

impl DateRange {
    pub fn new(from: NaiveDate, to: NaiveDate) -> Self {
        DateRange { from, to }
    }
}
/// Minimal synchronous quote-provider interface; orchestration stays in the service.
pub trait QuoteProvider: Send + Sync {
    fn id(&self) -> &'static str;
    /// Return available observations only; lookup code supplies backward fill.
    fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>>;

    /// Whether this source can quote an instrument of this kind at all. A chain skips one that
    /// cannot rather than spending a request on it.
    fn covers(&self, _security: &Security) -> bool {
        true
    }

    /// Quotes plus the dividends and splits the same response carries. Overridden only by a
    /// provider that gets events in that one request; asking for them separately is not its job.
    fn fetch_history(&self, security: &Security, range: DateRange) -> Result<History> {
        Ok(History {
            quotes: self.fetch(security, range)?,
            events: Vec::new(),
        })
    }
}

/// What one provider request returned.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct History {
    pub quotes: Vec<Quote>,
    pub events: Vec<SecurityEvent>,
}
