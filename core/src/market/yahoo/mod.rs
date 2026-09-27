use super::{DateRange, History, Quote, QuoteProvider, SecurityMatch, SecuritySearch};
use crate::error::{Error, Result};
use crate::model::{Security, SecurityEvent, SecurityKind};
use chrono::{DateTime, NaiveTime, TimeZone, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
/// Yahoo Finance adapter for daily quotes, search, and symbol profiles.
pub struct YahooProvider {
    http_timeout: Duration,
}

impl Default for YahooProvider {
    fn default() -> Self {
        YahooProvider {
            http_timeout: Duration::from_secs(20),
        }
    }
}
#[derive(Deserialize)]
struct ChartResponse {
    chart: Chart,
}

#[derive(Deserialize)]
struct Chart {
    #[serde(default)]
    result: Vec<ChartResult>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ChartResult {
    meta: Meta,
    #[serde(default)]
    timestamp: Vec<i64>,
    indicators: Indicators,
    #[serde(default)]
    events: ChartEvents,
}

/// Dividends and splits, present only when the request asked for `events`; keyed by timestamp.
#[derive(Deserialize, Default)]
struct ChartEvents {
    #[serde(default)]
    dividends: HashMap<String, DividendEvent>,
    #[serde(default)]
    splits: HashMap<String, SplitEvent>,
}

/// Yahoo's JSON wire format uses floating-point amounts and ratios.
#[derive(Deserialize)]
struct DividendEvent {
    amount: f64,
    date: i64,
}

#[derive(Deserialize)]
struct SplitEvent {
    date: i64,
    /// Shares after the split: a 4-for-1 is `numerator = 4, denominator = 1`.
    numerator: f64,
    denominator: f64,
}

#[derive(Deserialize)]
struct Meta {
    #[serde(default)]
    currency: Option<String>,
    #[serde(default)]
    symbol: Option<String>,
    #[serde(rename = "longName", default)]
    long_name: Option<String>,
    #[serde(rename = "shortName", default)]
    short_name: Option<String>,
    #[serde(rename = "fullExchangeName", default)]
    full_exchange_name: Option<String>,
    #[serde(rename = "instrumentType", default)]
    instrument_type: Option<String>,
    #[serde(rename = "gmtoffset", default)]
    gmt_offset: i64,
}

#[derive(Deserialize)]
struct Indicators {
    #[serde(default)]
    quote: Vec<QuoteArrays>,
}

#[derive(Deserialize)]
struct QuoteArrays {
    /// Yahoo's JSON wire format uses floating-point close values.
    #[serde(default)]
    close: Vec<Option<f64>>,
}

impl YahooProvider {
    /// The stored name of this source (`securities.data_source`, `quotes.source`).
    pub const ID: &'static str = "yahoo";

    pub fn new() -> Self {
        Self::default()
    }

    fn url(symbol: &str, range: DateRange) -> String {
        let start = Utc
            .from_utc_datetime(
                &range
                    .from
                    .pred_opt()
                    .unwrap_or(range.from)
                    .and_time(NaiveTime::MIN),
            )
            .timestamp();
        let end = Utc
            .from_utc_datetime(&range.to.succ_opt().unwrap_or(range.to).and_time(NaiveTime::MIN))
            .timestamp();
        format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?period1={start}&period2={end}&interval=1d&events=div%7Csplit"
        )
    }

    /// Yahoo returns wire-format floating-point prices; round their f32 noise once.
    fn price_from_json(v: f64) -> Option<Decimal> {
        Decimal::from_f64(v)?.round_sf(7)
    }

    fn get(&self, url: String) -> Result<String> {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.http_timeout))
            .build()
            .new_agent();
        Ok(agent
            .get(url)
            .header(
                "User-Agent",
                concat!("stonqs/", env!("CARGO_PKG_VERSION"), " (+https://stonqs.app)"),
            )
            .call()?
            .body_mut()
            .read_to_string()?)
    }

    fn search_url(query: &str) -> String {
        format!(
            "https://query1.finance.yahoo.com/v1/finance/search?q={}&quotesCount=10&newsCount=0",
            urlencode(query)
        )
    }

    fn profile_url(symbol: &str) -> String {
        format!(
            "https://query1.finance.yahoo.com/v8/finance/chart/{}?range=1mo&interval=1d",
            urlencode(symbol)
        )
    }

    fn kind_from_type(quote_type: Option<&str>) -> SecurityKind {
        match quote_type.unwrap_or_default().to_uppercase().as_str() {
            "EQUITY" => SecurityKind::Stock,
            "ETF" => SecurityKind::Etf,
            "MUTUALFUND" => SecurityKind::Fund,
            "CRYPTOCURRENCY" => SecurityKind::Crypto,
            "BOND" => SecurityKind::Bond,
            _ => SecurityKind::Other,
        }
    }

    /// The suffix decides; only suffix-less US venues fall back to the exchange name.
    pub(crate) fn mic_of(symbol: &str, exchange: Option<&str>) -> Option<&'static str> {
        let symbol = symbol.trim().to_uppercase();
        let suffix = symbol.rfind('.').map(|dot| &symbol[dot..]);
        if let Some(suffix) = suffix
            && let Some((mic, _)) = YAHOO_SUFFIXES
                .iter()
                .find(|(_, s)| !s.is_empty() && s.eq_ignore_ascii_case(suffix))
        {
            return Some(mic);
        }
        // A suffix we do not know is a venue we do not support, not a US listing.
        if suffix.is_some() {
            return None;
        }
        let exchange = exchange?.trim().to_uppercase();
        YAHOO_US_EXCHANGES
            .iter()
            .find(|(name, _)| exchange.starts_with(name))
            .map(|(_, mic)| *mic)
    }

    pub(crate) fn parse_search(body: &str) -> Result<Vec<SecurityMatch>> {
        let parsed: SearchResponse = serde_json::from_str(body).map_err(|e| Error::BadProviderData {
            provider: YahooProvider::ID,
            detail: format!("malformed JSON: {e}"),
        })?;
        Ok(parsed
            .quotes
            .into_iter()
            .filter(|q| q.is_yahoo_finance)
            .filter_map(|q| {
                let symbol = q.symbol?;
                let name = q.longname.or(q.shortname).unwrap_or_else(|| symbol.clone());
                Some(SecurityMatch {
                    source: YahooProvider::ID.to_string(),
                    kind: Self::kind_from_type(q.quote_type.as_deref()),
                    mic: Self::mic_of(&symbol, q.exchange_display.as_deref()).map(str::to_string),
                    exchange: q.exchange_display,
                    currency: None,
                    isin: None,
                    has_history: None,
                    last_close: None,
                    symbol,
                    name,
                })
            })
            .collect())
    }

    pub(crate) fn parse_profile(symbol: &str, body: &str) -> Result<Option<SecurityMatch>> {
        let parsed: ChartResponse = serde_json::from_str(body).map_err(|e| Error::BadProviderData {
            provider: YahooProvider::ID,
            detail: format!("malformed JSON: {e}"),
        })?;
        let Some(result) = parsed.chart.result.first() else {
            return Ok(None);
        };
        let meta = &result.meta;
        let symbol = meta.symbol.clone().unwrap_or_else(|| symbol.to_string());
        let (currency, factor) = match meta.currency.as_deref() {
            Some(raw) => {
                let (c, f) = crate::money::major_currency(raw);
                (Some(c), f)
            }
            None => (None, Decimal::ONE),
        };
        let name = meta
            .long_name
            .clone()
            .or_else(|| meta.short_name.clone())
            .unwrap_or_else(|| symbol.clone());
        Ok(Some(SecurityMatch {
            source: YahooProvider::ID.to_string(),
            kind: Self::kind_from_type(meta.instrument_type.as_deref()),
            mic: Self::mic_of(&symbol, meta.full_exchange_name.as_deref()).map(str::to_string),
            exchange: meta.full_exchange_name.clone(),
            currency,
            isin: None,
            has_history: Some(!result.timestamp.is_empty()),
            last_close: result
                .indicators
                .quote
                .first()
                .and_then(|q| q.close.iter().rev().flatten().next().copied())
                .and_then(Self::price_from_json)
                .map(|c| c * factor),
            symbol,
            name,
        }))
    }

    #[cfg(test)]
    pub(crate) fn parse_chart(security: &Security, body: &str) -> Result<Vec<Quote>> {
        Ok(Self::parse_history(security, body)?.quotes)
    }

    pub(crate) fn parse_history(security: &Security, body: &str) -> Result<History> {
        let parsed: ChartResponse = serde_json::from_str(body).map_err(|e| Error::BadProviderData {
            provider: YahooProvider::ID,
            detail: format!("malformed JSON: {e}"),
        })?;

        if let Some(err) = parsed.chart.error {
            return Err(Error::BadProviderData {
                provider: YahooProvider::ID,
                detail: err.to_string(),
            });
        }
        let result = parsed.chart.result.first().ok_or(Error::BadProviderData {
            provider: YahooProvider::ID,
            detail: "empty chart.result".to_string(),
        })?;
        let closes = &result
            .indicators
            .quote
            .first()
            .ok_or(Error::BadProviderData {
                provider: YahooProvider::ID,
                detail: "no quote arrays".to_string(),
            })?
            .close;

        let (currency, factor) = match result.meta.currency.as_deref() {
            Some(raw) => crate::money::major_currency(raw),
            None => (security.currency.clone(), Decimal::ONE),
        };

        // Apply the source timezone before assigning the calendar date.
        let offset = result.meta.gmt_offset;
        let day = |ts: i64| DateTime::from_timestamp(ts + offset, 0).map(|local| local.date_naive());

        let mut out = Vec::with_capacity(result.timestamp.len());
        for (i, ts) in result.timestamp.iter().enumerate() {
            let Some(Some(close)) = closes.get(i).copied() else {
                continue;
            };
            let Some(close) = Self::price_from_json(close) else {
                continue;
            };
            if close <= Decimal::ZERO {
                continue;
            }
            let Some(date) = day(*ts) else {
                continue;
            };
            out.push(Quote {
                security_id: security.id.clone(),
                date,
                close: close * factor,
                currency: currency.clone(),
                source: YahooProvider::ID.to_string(),
            });
        }

        let mut events = Vec::new();
        for dividend in result.events.dividends.values() {
            let (Some(date), Some(amount)) = (day(dividend.date), Self::price_from_json(dividend.amount))
            else {
                continue;
            };
            if amount > Decimal::ZERO {
                // Pence stay pence per share until folded, exactly like the closes beside them.
                events.push(SecurityEvent::dividend(
                    &security.id,
                    date,
                    amount * factor,
                    &currency,
                    YahooProvider::ID,
                ));
            }
        }
        for split in result.events.splits.values() {
            let ratio = |v: f64| {
                Decimal::from_f64(v)
                    .map(|d| d.normalize())
                    .filter(|d| *d > Decimal::ZERO)
            };
            let (Some(date), Some(from), Some(to)) =
                (day(split.date), ratio(split.denominator), ratio(split.numerator))
            else {
                continue;
            };
            events.push(SecurityEvent::split(
                &security.id,
                date,
                from,
                to,
                YahooProvider::ID,
            ));
        }
        // The events arrive as a JSON object; a stable order keeps a re-fetch comparable.
        events.sort_by(|a: &SecurityEvent, b| (a.date, a.kind.as_str()).cmp(&(b.date, b.kind.as_str())));

        Ok(History { quotes: out, events })
    }
}
#[derive(Deserialize)]
struct SearchResponse {
    #[serde(default)]
    quotes: Vec<SearchQuote>,
}

#[derive(Deserialize)]
struct SearchQuote {
    #[serde(default)]
    symbol: Option<String>,
    #[serde(default)]
    shortname: Option<String>,
    #[serde(default)]
    longname: Option<String>,
    #[serde(rename = "exchDisp", default)]
    exchange_display: Option<String>,
    #[serde(rename = "quoteType", default)]
    quote_type: Option<String>,
    #[serde(rename = "isYahooFinance", default)]
    is_yahoo_finance: bool,
}

impl QuoteProvider for YahooProvider {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>> {
        Ok(self.fetch_history(security, range)?.quotes)
    }

    fn fetch_history(&self, security: &Security, range: DateRange) -> Result<History> {
        let body = self.get(Self::url(security.provider_symbol(), range))?;
        Self::parse_history(security, &body)
    }
}

impl SecuritySearch for YahooProvider {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn search(&self, query: &str) -> Result<Vec<SecurityMatch>> {
        Self::parse_search(&self.get(Self::search_url(query))?)
    }

    fn profile(&self, symbol: &str) -> Result<Option<SecurityMatch>> {
        Self::parse_profile(symbol, &self.get(Self::profile_url(symbol))?)
    }

    fn symbol_for(&self, ticker: &str, mic: &str) -> Option<String> {
        let suffix = YAHOO_SUFFIXES.iter().find(|(m, _)| *m == mic)?.1;
        Some(format!("{ticker}{suffix}"))
    }

    fn mic_for(&self, symbol: &str, exchange: Option<&str>) -> Option<&'static str> {
        YahooProvider::mic_of(symbol, exchange)
    }
}

/// Yahoo's own exchange names for the venues whose symbols carry no suffix, matched
/// case-insensitively by prefix: "NasdaqGS", "NasdaqCM" and "NASDAQ" are all Nasdaq.
const YAHOO_US_EXCHANGES: &[(&str, &str)] = &[
    ("NYSEARCA", "ARCX"),
    ("NYSE ARCA", "ARCX"),
    ("NYSEAMERICAN", "XNYS"),
    ("NYSE", "XNYS"),
    ("NASDAQ", "XNAS"),
    ("NMS", "XNAS"),
    ("NGM", "XNAS"),
    ("NCM", "XNAS"),
    ("NYQ", "XNYS"),
    ("PCX", "ARCX"),
];

const YAHOO_SUFFIXES: &[(&str, &str)] = &[
    ("XETR", ".DE"),
    ("XFRA", ".F"),
    ("XAMS", ".AS"),
    ("XPAR", ".PA"),
    ("XMIL", ".MI"),
    ("XLON", ".L"),
    ("XSWX", ".SW"),
    ("XMAD", ".MC"),
    ("XBRU", ".BR"),
    ("XLIS", ".LS"),
    ("XWBO", ".VI"),
    ("XSTO", ".ST"),
    ("XCSE", ".CO"),
    ("XHEL", ".HE"),
    ("XOSL", ".OL"),
    ("XWAR", ".WA"),
    ("XNAS", ""),
    ("XNYS", ""),
    ("ARCX", ""),
    ("XTSE", ".TO"),
    ("XHKG", ".HK"),
    ("XTKS", ".T"),
    ("XASX", ".AX"),
    ("XSES", ".SI"),
    ("XJSE", ".JO"),
    ("XTAE", ".TA"),
    ("XMEX", ".MX"),
    ("BVMF", ".SA"),
    ("XNSE", ".NS"),
];

fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests;
