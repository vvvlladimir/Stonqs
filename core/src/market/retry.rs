//! Retry policy shared by market and other synchronous network adapters.

use crate::error::Result;
use std::time::Duration;
/// Number of attempts and the initial exponential backoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FetchPolicy {
    pub attempts: u32,
    pub backoff: Duration,
}

impl Default for FetchPolicy {
    fn default() -> Self {
        FetchPolicy {
            attempts: 3,
            backoff: Duration::from_secs(1),
        }
    }
}

impl FetchPolicy {
    pub fn none() -> Self {
        FetchPolicy {
            attempts: 1,
            backoff: Duration::ZERO,
        }
    }
    /// Run an operation again only when its error is transient.
    pub fn run<T>(&self, mut attempt: impl FnMut() -> Result<T>) -> Result<T> {
        let mut delay = self.backoff;
        for _ in 1..self.attempts.max(1) {
            match attempt() {
                Err(e) if e.is_transient() => {
                    if !delay.is_zero() {
                        std::thread::sleep(delay);
                    }
                    delay *= 2;
                }
                done => return done,
            }
        }
        attempt()
    }
}

/// Daily request allowances of keyed sources, counted in the store so a restart does not reset
/// them. A source without one is not counted at all.
#[derive(Debug, Clone, Default)]
pub struct Budgets {
    per_day: std::collections::HashMap<&'static str, u32>,
}

impl Budgets {
    pub fn set(&mut self, source: &'static str, per_day: u32) {
        self.per_day.insert(source, per_day);
    }

    /// Spends one request of `source` today, or refuses with `RateLimited` once the day's are gone.
    pub fn spend(&self, store: &crate::storage::Store, source: &str) -> Result<()> {
        let Some(&limit) = self.per_day.get(source) else {
            return Ok(());
        };
        let today = chrono::Utc::now().date_naive();
        if store.take_request(source, today, limit)? {
            Ok(())
        } else {
            Err(crate::error::Error::RateLimited(format!(
                "{source}: the {limit} requests of {today} are spent"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use std::cell::Cell;

    fn instant(attempts: u32) -> FetchPolicy {
        FetchPolicy {
            attempts,
            backoff: Duration::ZERO,
        }
    }

    #[test]
    fn success_does_not_retry() {
        let calls = Cell::new(0);
        let out = instant(3).run(|| {
            calls.set(calls.get() + 1);
            Ok(42)
        });
        assert_eq!(out.unwrap(), 42);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn transient_error_is_retried_until_it_succeeds() {
        let calls = Cell::new(0);
        let out = instant(3).run(|| {
            calls.set(calls.get() + 1);
            if calls.get() < 3 {
                Err(Error::RateLimited("429".into()))
            } else {
                Ok("ok")
            }
        });
        assert_eq!(out.unwrap(), "ok");
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn transient_error_gives_up_after_the_last_attempt() {
        let calls = Cell::new(0);
        let out: Result<()> = instant(3).run(|| {
            calls.set(calls.get() + 1);
            Err(Error::Unavailable("503".into()))
        });
        assert!(out.is_err());
        assert_eq!(calls.get(), 3);
    }

    #[test]
    fn permanent_error_is_not_retried() {
        let calls = Cell::new(0);
        let out: Result<()> = instant(3).run(|| {
            calls.set(calls.get() + 1);
            Err(Error::Network("404".into()))
        });
        assert!(out.is_err());
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn a_refused_key_is_not_retried() {
        let calls = Cell::new(0);
        let out: Result<()> = instant(3).run(|| {
            calls.set(calls.get() + 1);
            Err(Error::Unauthorized("401".into()))
        });
        assert!(out.is_err());
        assert_eq!(calls.get(), 1);
    }
}
