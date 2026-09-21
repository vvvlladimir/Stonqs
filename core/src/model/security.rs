use crate::money::{Currency, normalize_currency};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SecurityKind {
    Stock,
    Etf,
    Bond,
    Fund,
    Crypto,
    Other,
}

/// A tradable instrument: stock, ETF, bond.
///
/// Two distinct symbols: [`Security::symbol`] (user-facing, "AAPL") vs
/// [`Security::data_symbol`] (provider-facing, "aapl.us" on Stooq). Merging
/// them would bake a specific provider into the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Security {
    pub id: String,
    pub symbol: String,
    pub isin: Option<String>,
    pub name: String,
    /// Currency the security trades and quotes in — not the account's currency.
    pub currency: Currency,
    pub kind: SecurityKind,
    /// Quote provider id (`"stooq"`); `None` = prices entered manually.
    pub data_source: Option<String>,
    /// Provider-side symbol; falls back to `symbol` when `None`.
    pub data_symbol: Option<String>,
    /// Minimum tradable quantity step; `None` = kind default ([`Security::effective_quantity_step`]).
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub quantity_step: Option<Decimal>,
    /// Chosen listing's venue, ISO 10383 code (`XETR`); `None` = no listing chosen yet.
    #[serde(default)]
    pub mic: Option<String>,
    /// German securities number, printed by German brokers beside the ISIN.
    #[serde(default)]
    pub wkn: Option<String>,
    /// The user's own note about the instrument; never touched by a provider.
    #[serde(default)]
    pub note: Option<String>,
}

impl Security {
    pub fn new(
        symbol: impl Into<String>,
        name: impl Into<String>,
        currency: &str,
        kind: SecurityKind,
    ) -> Self {
        Security {
            id: super::new_id(),
            symbol: symbol.into(),
            isin: None,
            name: name.into(),
            currency: normalize_currency(currency),
            kind,
            data_source: None,
            data_symbol: None,
            quantity_step: None,
            mic: None,
            wkn: None,
            note: None,
        }
    }

    pub fn with_source(mut self, source: &str, data_symbol: &str) -> Self {
        self.data_source = Some(source.to_string());
        self.data_symbol = Some(data_symbol.to_string());
        self
    }

    pub fn provider_symbol(&self) -> &str {
        self.data_symbol.as_deref().unwrap_or(&self.symbol)
    }

    /// False for a bare ISIN or an unticked-venue placeholder (`<ISIN>.SG`) — see CLAUDE.md invariants.
    pub fn is_quotable(&self) -> bool {
        self.data_source.is_some() && !symbol_is_isin(self.provider_symbol())
    }

    /// Provider is set but the symbol won't resolve until identified via the directory.
    pub fn needs_lookup(&self) -> bool {
        self.data_source.is_some() && symbol_is_isin(self.provider_symbol())
    }

    pub fn with_quantity_step(mut self, step: Decimal) -> Self {
        self.quantity_step = Some(step);
        self
    }

    /// Crypto defaults to fractional; everything else defaults to whole shares.
    pub fn effective_quantity_step(&self) -> Decimal {
        self.quantity_step.unwrap_or(match self.kind {
            SecurityKind::Crypto => Decimal::new(1, 8),
            _ => Decimal::ONE,
        })
    }

    /// Priority: explicit step, then step observed in history, then kind default.
    pub fn quantity_step_for(&self, observed: Option<Decimal>) -> Decimal {
        self.quantity_step
            .or(observed)
            .unwrap_or_else(|| self.effective_quantity_step())
    }

    /// Rounds down, not to nearest — a "buy more" recommendation must never overspend the drift.
    pub fn round_to_step(&self, quantity: Decimal) -> Decimal {
        floor_to_step(quantity, self.effective_quantity_step())
    }
}

pub fn floor_to_step(quantity: Decimal, step: Decimal) -> Decimal {
    if step.is_zero() {
        return quantity;
    }
    (quantity / step).floor() * step
}

/// Step observed across trade quantities: 10^-(max decimal scale seen), read off the normalized number.
pub fn observed_quantity_step(quantities: impl IntoIterator<Item = Decimal>) -> Option<Decimal> {
    let mut scale: Option<u32> = None;
    for quantity in quantities {
        if quantity.is_zero() {
            continue;
        }
        let seen = quantity.normalize().scale();
        scale = Some(scale.map_or(seen, |best: u32| best.max(seen)));
    }
    scale.map(|s| Decimal::new(1, s))
}

impl SecurityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            SecurityKind::Stock => "STOCK",
            SecurityKind::Etf => "ETF",
            SecurityKind::Bond => "BOND",
            SecurityKind::Fund => "FUND",
            SecurityKind::Crypto => "CRYPTO",
            SecurityKind::Other => "OTHER",
        }
    }

    pub fn parse(s: &str) -> crate::error::Result<Self> {
        Ok(match s {
            "STOCK" => SecurityKind::Stock,
            "ETF" => SecurityKind::Etf,
            "BOND" => SecurityKind::Bond,
            "FUND" => SecurityKind::Fund,
            "CRYPTO" => SecurityKind::Crypto,
            "OTHER" => SecurityKind::Other,
            other => {
                return Err(crate::error::Error::Invalid(format!(
                    "unknown security kind {other:?}"
                )));
            }
        })
    }
}

/// Separate from [`is_isin`] — that asks "is this value an ISIN?", this asks "is this symbol useless for quotes?".
fn symbol_is_isin(symbol: &str) -> bool {
    is_isin(symbol.split('.').next().unwrap_or(symbol))
}

/// Two-letter country code, nine-character security code, check digit (Luhn).
pub fn is_isin(value: &str) -> bool {
    let value = value.trim();
    if value.len() != 12 || !value.is_ascii() {
        return false;
    }
    let bytes = value.as_bytes();
    if !bytes[..2].iter().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    if !bytes[2..11].iter().all(|b| b.is_ascii_alphanumeric()) || !bytes[11].is_ascii_digit() {
        return false;
    }

    // Letters expand to two digits (A = 10, ..., Z = 35), then Luhn right-to-left over the digit string.
    let digits: Vec<u32> = value
        .to_ascii_uppercase()
        .chars()
        .flat_map(|c| {
            let v = if c.is_ascii_digit() {
                c as u32 - '0' as u32
            } else {
                c as u32 - 'A' as u32 + 10
            };
            if v >= 10 { vec![v / 10, v % 10] } else { vec![v] }
        })
        .collect();

    let mut sum = 0;
    for (i, d) in digits.iter().rev().enumerate() {
        let d = if i % 2 == 1 {
            let doubled = d * 2;
            if doubled > 9 { doubled - 9 } else { doubled }
        } else {
            *d
        };
        sum += d;
    }
    sum % 10 == 0
}

#[cfg(test)]
mod step_tests {
    use super::{Security, SecurityKind, observed_quantity_step};
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    /// 3.426301 has 6 decimals -> observed step = 10^-6 = 0.000001. A round
    /// trade alongside it doesn't override (max scale wins). SQLite's
    /// trailing zeros ("4.000000") don't count — compared normalized.
    #[test]
    fn the_step_is_the_widest_scale_seen_in_the_quantities() {
        assert_eq!(
            observed_quantity_step([dec!(3.426301), dec!(4.000000)]),
            Some(dec!(0.000001))
        );
        assert_eq!(observed_quantity_step([dec!(4.000000)]), Some(Decimal::ONE));
        // Zero quantities are cash operations, not fractionality signal.
        assert_eq!(observed_quantity_step([Decimal::ZERO]), None);
        assert_eq!(observed_quantity_step([]), None);
    }

    #[test]
    fn an_explicit_step_wins_over_the_history() {
        let etf = Security::new("VWCE", "world", "EUR", SecurityKind::Etf);
        assert_eq!(etf.quantity_step_for(None), Decimal::ONE);
        assert_eq!(etf.quantity_step_for(Some(dec!(0.000001))), dec!(0.000001));

        let manual = etf.clone().with_quantity_step(dec!(0.001));
        assert_eq!(manual.quantity_step_for(Some(dec!(0.000001))), dec!(0.001));
    }
}

#[cfg(test)]
mod isin_tests {
    use super::is_isin;

    #[test]
    fn accepts_real_isins() {
        // Check digits are issuer-assigned; these are real codes from a Trade Republic export.
        assert!(is_isin("IE00B5BMR087"));
        assert!(is_isin("US0378331005"));
        assert!(is_isin("CNE100000296"));
        assert!(is_isin("DE0007164600"));
    }

    #[test]
    fn a_security_whose_symbol_is_an_isin_is_not_quotable() {
        use super::{Security, SecurityKind};

        let resolved = Security::new("CSSPX.MI", "iShares Core S&P 500", "EUR", SecurityKind::Etf)
            .with_source("yahoo", "CSSPX.MI");
        assert!(resolved.is_quotable());
        assert!(!resolved.needs_lookup());

        let mut broken = Security::new("IE00B5BMR087", "IE00B5BMR087", "EUR", SecurityKind::Other);
        broken.data_source = Some("yahoo".into());
        assert!(!broken.is_quotable());
        assert!(broken.needs_lookup());

        // Venue placeholder: request succeeds but never returns candles.
        let mut placeholder = Security::new(
            "IE000I8KRLL9.SG",
            "iShares MSCI Global",
            "EUR",
            SecurityKind::Fund,
        );
        placeholder.data_source = Some("yahoo".into());
        assert!(!placeholder.is_quotable());
        assert!(placeholder.needs_lookup());

        // Manual price entry isn't an error and needs no identification.
        let manual = Security::new("IE00B5BMR087", "IE00B5BMR087", "EUR", SecurityKind::Other);
        assert!(!manual.is_quotable());
        assert!(!manual.needs_lookup());
    }

    #[test]
    fn rejects_tickers_and_broken_checksums() {
        assert!(!is_isin("AAPL"));
        assert!(!is_isin("CSSPX.MI"));
        assert!(!is_isin(""));
        // Same length and shape, but the check digit doesn't validate.
        assert!(!is_isin("US0378331006"));
        assert!(!is_isin("123456789012"));
    }
}
