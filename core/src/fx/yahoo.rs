use super::{FxProvider, FxRate};
use crate::error::{Error, Result};
use crate::market::{DateRange, QuoteProvider, YahooProvider};
use crate::model::{Security, SecurityKind};
use crate::money::normalize_currency;

/// Rates read off Yahoo's `BASEQUOTE=X` series. A market close rather than a central bank's fixing,
/// so it stands behind the ECB and answers the currencies the ECB does not publish.
#[derive(Default)]
pub struct YahooFxProvider {
    quotes: YahooProvider,
}

impl YahooFxProvider {
    pub fn new() -> Self {
        Self::default()
    }

    fn symbol(base: &str, quote: &str) -> String {
        format!("{base}{quote}=X")
    }
}

impl FxProvider for YahooFxProvider {
    fn id(&self) -> &'static str {
        YahooProvider::ID
    }

    fn fetch(&self, base: &str, quote: &str, range: DateRange) -> Result<Vec<FxRate>> {
        let (base, quote) = (normalize_currency(base), normalize_currency(quote));
        if base == quote {
            return Ok(Vec::new());
        }
        let symbol = Self::symbol(&base, &quote);
        let pair = Security::new(&symbol, &symbol, &quote, SecurityKind::Other)
            .with_source(YahooProvider::ID, &symbol);
        let closes = self.quotes.fetch(&pair, range)?;
        // A series quoted in anything but the quote currency is not this pair's rate.
        if let Some(wrong) = closes.iter().find(|q| q.currency != quote) {
            return Err(Error::BadProviderData {
                provider: YahooProvider::ID,
                detail: format!("{symbol} quoted in {}", wrong.currency),
            });
        }
        Ok(closes
            .into_iter()
            .map(|q| FxRate::new(&base, &quote, q.date, q.close))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn names_the_pair_the_way_yahoo_does() {
        assert_eq!(YahooFxProvider::symbol("EUR", "RUB"), "EURRUB=X");
    }

    #[test]
    #[ignore = "requires network"]
    fn fetches_a_pair_the_ecb_does_not_publish() {
        let d = |s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        let range = DateRange::new(d("2024-06-03"), d("2024-06-07"));
        let rates = YahooFxProvider::new().fetch("EUR", "RUB", range).unwrap();
        assert!(!rates.is_empty());
        assert!(rates.iter().all(|r| r.base == "EUR" && r.quote == "RUB"));
    }
}
