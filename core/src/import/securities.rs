use crate::market::SecurityMatch;
use crate::model::{Security, SecurityKind, is_isin};
use crate::money::{Currency, normalize_currency};
use serde::{Deserialize, Serialize};

/// Security definition prepared during preview and persisted during commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityDraft {
    pub symbol: String,
    pub name: String,
    pub currency: Currency,
    pub kind: SecurityKind,
    pub isin: Option<String>,

    pub data_source: Option<String>,
    pub data_symbol: Option<String>,
    pub exchange: Option<String>,
    /// ISO 10383 code behind `exchange`, when the source named a supported venue.
    pub mic: Option<String>,
}

impl SecurityDraft {
    /// Builds a draft from an external search result.
    pub fn from_match(found: &SecurityMatch, fallback_currency: &str) -> Self {
        SecurityDraft {
            symbol: found.symbol.to_uppercase(),
            name: found.name.clone(),
            currency: found
                .currency
                .clone()
                .unwrap_or_else(|| normalize_currency(fallback_currency)),
            kind: found.kind,
            isin: found.isin.clone(),
            data_source: Some(found.source.clone()),

            data_symbol: None,
            exchange: found.exchange.clone(),
            mic: found.mic.clone(),
        }
    }

    /// Builds an unresolved draft without treating an ISIN as a quote symbol.
    pub fn unresolved(symbol: &str, name: Option<&str>, currency: &str) -> Self {
        let isin = is_isin(symbol).then(|| symbol.to_uppercase());
        SecurityDraft {
            symbol: symbol.to_uppercase(),
            name: name.unwrap_or(symbol).to_string(),
            currency: normalize_currency(currency),
            kind: SecurityKind::Other,
            isin,
            data_source: None,
            data_symbol: None,
            exchange: None,
            mic: None,
        }
    }
    /// Converts the draft into the storage model.
    pub fn to_security(&self) -> Security {
        Security {
            isin: self.isin.clone(),
            mic: self.mic.clone(),
            data_source: self.data_source.clone(),
            data_symbol: self.data_symbol.clone(),
            ..Security::new(
                self.symbol.to_uppercase(),
                self.name.clone(),
                &self.currency,
                self.kind,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found() -> SecurityMatch {
        SecurityMatch {
            source: "yahoo".into(),
            symbol: "CSSPX.MI".into(),
            name: "iShares Core S&P 500 UCITS ETF USD (Acc)".into(),
            exchange: Some("Milan".into()),
            mic: Some("XMIL".into()),
            kind: SecurityKind::Etf,
            currency: Some("EUR".into()),
            last_close: None,
            isin: Some("IE00B5BMR087".into()),
            has_history: Some(true),
        }
    }
    #[test]
    fn resolved_draft_carries_the_provider() {
        let draft = SecurityDraft::from_match(&found(), "USD");
        assert_eq!(draft.symbol, "CSSPX.MI");
        assert_eq!(
            draft.currency, "EUR",
            "the source currency wins over the row currency"
        );
        assert_eq!(draft.data_source.as_deref(), Some("yahoo"));
        assert_eq!(draft.to_security().isin.as_deref(), Some("IE00B5BMR087"));
    }
    #[test]
    fn currency_falls_back_to_the_row() {
        let draft = SecurityDraft::from_match(
            &SecurityMatch {
                currency: None,
                ..found()
            },
            "EUR",
        );
        assert_eq!(draft.currency, "EUR");
    }
    #[test]
    fn unresolved_isin_gets_no_source() {
        let draft = SecurityDraft::unresolved("IE00B5BMR087", Some("Core S&P 500"), "EUR");
        assert_eq!(draft.isin.as_deref(), Some("IE00B5BMR087"));
        assert_eq!(draft.name, "Core S&P 500");
        assert!(draft.data_source.is_none());
    }
}
