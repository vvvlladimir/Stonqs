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

    /// The venue behind a Yahoo symbol. The suffix decides whenever there is one — it is the
    /// venue spelled into the symbol. Only the suffix-less US venues fall back to the exchange
    /// name, because there one ticker is shared by Nasdaq, NYSE and Arca.
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
mod tests {
    use super::*;
    use crate::model::SecurityKind;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn aapl() -> Security {
        Security::new("AAPL", "Apple Inc.", "USD", SecurityKind::Stock).with_source("yahoo", "AAPL")
    }

    const SAMPLE: &str = r#"{
        "chart": {
            "result": [{
                "meta": {"currency": "USD", "symbol": "AAPL", "gmtoffset": -14400},
                "timestamp": [1717421400, 1717507800, 1717594200],
                "indicators": {"quote": [{"close": [194.02999877929688, null, 195.8699951171875]}]}
            }],
            "error": null
        }
    }"#;

    #[test]
    fn every_supported_market_can_be_named() {
        for m in crate::market::mic::supported_mics() {
            assert!(
                YAHOO_SUFFIXES.iter().any(|(mic, _)| *mic == m),
                "venue {m} has no Yahoo suffix"
            );
        }
    }

    #[test]
    fn the_suffix_names_the_venue_and_beats_the_exchange_name() {
        assert_eq!(YahooProvider::mic_of("EUNL.DE", Some("NYSE")), Some("XETR"));
        assert_eq!(YahooProvider::mic_of("iwda.l", None), Some("XLON"));
        assert_eq!(
            YahooProvider::mic_of("SAP.XX", Some("NYSE")),
            None,
            "an unknown suffix is an unsupported venue, not a US listing"
        );
    }

    #[test]
    fn a_suffixless_symbol_takes_the_venue_from_the_exchange_name() {
        assert_eq!(YahooProvider::mic_of("AWK", Some("NYSE")), Some("XNYS"));
        assert_eq!(YahooProvider::mic_of("AAPL", Some("NasdaqGS")), Some("XNAS"));
        assert_eq!(YahooProvider::mic_of("SPY", Some("NYSEArca")), Some("ARCX"));
        assert_eq!(YahooProvider::mic_of("AWK", None), None);
        assert_eq!(YahooProvider::mic_of("AWK", Some("Bombay")), None);
    }

    #[test]
    fn every_venue_named_by_mic_of_is_a_supported_market() {
        for (_, mic) in YAHOO_US_EXCHANGES {
            assert!(
                crate::market::mic::market_name(mic).is_some(),
                "venue {mic} is not a supported market"
            );
        }
    }

    #[test]
    fn a_listing_is_named_by_ticker_and_exchange() {
        let yahoo = YahooProvider::new();
        assert_eq!(yahoo.symbol_for("EUNL", "XETR").as_deref(), Some("EUNL.DE"));
        assert_eq!(yahoo.symbol_for("AAPL", "XNAS").as_deref(), Some("AAPL"));
        assert_eq!(yahoo.symbol_for("EUNL", "XXXX"), None);
    }

    #[test]
    fn london_pence_are_converted_to_pounds() {
        let swda = Security::new("SWDA.L", "iShares Core MSCI World", "GBP", SecurityKind::Etf)
            .with_source("yahoo", "SWDA.L");
        let body = r#"{
            "chart": {
                "result": [{
                    "meta": {"currency": "GBp", "symbol": "SWDA.L", "gmtoffset": 3600},
                    "timestamp": [1717421400],
                    "indicators": {"quote": [{"close": [10953.0]}]}
                }],
                "error": null
            }
        }"#;
        let quotes = YahooProvider::parse_chart(&swda, body).unwrap();
        assert_eq!(quotes[0].currency, "GBP");
        assert_eq!(quotes[0].close, dec!(109.53));
    }

    #[test]
    fn parses_chart_and_skips_null_days() {
        let quotes = YahooProvider::parse_chart(&aapl(), SAMPLE).unwrap();
        assert_eq!(quotes.len(), 2, "a day with a null price is skipped");
        assert_eq!(quotes[0].date, NaiveDate::from_ymd_opt(2024, 6, 3).unwrap());
        assert_eq!(quotes[1].date, NaiveDate::from_ymd_opt(2024, 6, 5).unwrap());
        assert_eq!(quotes[0].currency, "USD");
        assert_eq!(quotes[0].source, "yahoo");
    }

    /// NVIDIA's 10-for-1 on 2024-06-10 and the 0.01 USD dividend the next day, as Yahoo prints them.
    #[test]
    fn dividends_and_splits_come_with_the_same_response() {
        let nvda = Security::new("NVDA", "NVIDIA", "USD", SecurityKind::Stock).with_source("yahoo", "NVDA");
        let body = r#"{
            "chart": {"result": [{
                "meta": {"currency": "USD", "symbol": "NVDA", "gmtoffset": -14400},
                "timestamp": [1718026200, 1718112600],
                "indicators": {"quote": [{"close": [121.79, 120.91]}]},
                "events": {
                    "dividends": {"1718112600": {"amount": 0.01, "date": 1718112600}},
                    "splits": {"1718026200": {"date": 1718026200, "numerator": 10.0,
                                              "denominator": 1.0, "splitRatio": "10:1"}}
                }
            }], "error": null}
        }"#;
        let history = YahooProvider::parse_history(&nvda, body).unwrap();
        assert_eq!(history.quotes.len(), 2);
        assert_eq!(history.events.len(), 2);

        let split = &history.events[0];
        assert_eq!(split.date, NaiveDate::from_ymd_opt(2024, 6, 10).unwrap());
        assert_eq!(
            (split.ratio_from, split.ratio_to),
            (Some(dec!(1)), Some(dec!(10)))
        );

        let dividend = &history.events[1];
        assert_eq!(dividend.date, NaiveDate::from_ymd_opt(2024, 6, 11).unwrap());
        assert_eq!(dividend.amount, Some(dec!(0.01)));
        assert_eq!(dividend.currency.as_deref(), Some("USD"));
        assert_eq!(dividend.source.as_deref(), Some("yahoo"));
    }

    #[test]
    fn a_response_without_events_has_none() {
        assert!(
            YahooProvider::parse_history(&aapl(), SAMPLE)
                .unwrap()
                .events
                .is_empty()
        );
    }

    #[test]
    fn f32_noise_is_rounded_away() {
        let quotes = YahooProvider::parse_chart(&aapl(), SAMPLE).unwrap();
        assert_eq!(quotes[0].close, dec!(194.03));
        assert_eq!(quotes[1].close, dec!(195.87));
    }

    #[test]
    fn small_prices_keep_significant_digits() {
        assert_eq!(
            YahooProvider::price_from_json(0.000001234567).unwrap(),
            dec!(0.000001234567)
        );
    }

    #[test]
    fn reports_provider_error() {
        let body = r#"{"chart":{"result":[],"error":{"code":"Not Found","description":"No data found"}}}"#;
        assert!(matches!(
            YahooProvider::parse_chart(&aapl(), body),
            Err(Error::BadProviderData {
                provider: "yahoo",
                ..
            })
        ));
    }

    const SEARCH_SAMPLE: &str = r#"{
        "quotes": [
            {"exchange": "MIL", "shortname": "ISHARES CORE S&P 500 UCITS ETF ",
             "quoteType": "ETF", "symbol": "CSSPX.MI", "typeDisp": "ETF",
             "longname": "iShares Core S&P 500 UCITS ETF USD (Acc)",
             "exchDisp": "Milan", "isYahooFinance": true},
            {"index": "industry", "name": "Asset Management", "isYahooFinance": false}
        ]
    }"#;

    #[test]
    fn search_keeps_only_instruments() {
        let found = YahooProvider::parse_search(SEARCH_SAMPLE).unwrap();
        assert_eq!(found.len(), 1, "an industry entry is not an instrument");
        assert_eq!(found[0].symbol, "CSSPX.MI");
        assert_eq!(found[0].name, "iShares Core S&P 500 UCITS ETF USD (Acc)");
        assert_eq!(found[0].exchange.as_deref(), Some("Milan"));
        assert_eq!(found[0].kind, SecurityKind::Etf);
        assert!(!found[0].is_complete());
    }

    const PROFILE_SAMPLE: &str = r#"{
        "chart": {"result": [{
            "meta": {"currency": "EUR", "symbol": "CSSPX.MI", "gmtoffset": 7200,
                     "fullExchangeName": "Milan", "instrumentType": "ETF",
                     "longName": "iShares Core S&P 500 UCITS ETF USD (Acc)"},
            "timestamp": [1717421400], "indicators": {"quote": [{"close": [715.13]}]}
        }], "error": null}
    }"#;

    #[test]
    fn profile_fills_the_currency() {
        let found = YahooProvider::parse_profile("CSSPX.MI", PROFILE_SAMPLE)
            .unwrap()
            .unwrap();
        assert_eq!(found.currency.as_deref(), Some("EUR"));
        assert_eq!(found.name, "iShares Core S&P 500 UCITS ETF USD (Acc)");
        assert_eq!(found.has_history, Some(true));
        assert!(found.is_complete());
    }

    #[test]
    fn a_listing_without_candles_is_not_complete() {
        let body = r#"{
            "chart": {"result": [{
                "meta": {"currency": "EUR", "symbol": "IE000I8KRLL9.SG",
                         "fullExchangeName": "Stuttgart", "instrumentType": "MUTUALFUND",
                         "shortName": "iShares MSCI Global Semiconduct", "gmtoffset": 7200,
                         "regularMarketPrice": 17.006},
                "timestamp": [], "indicators": {"quote": [{"close": []}]}
            }], "error": null}
        }"#;
        let found = YahooProvider::parse_profile("IE000I8KRLL9.SG", body)
            .unwrap()
            .unwrap();
        assert_eq!(found.currency.as_deref(), Some("EUR"), "the currency is there");
        assert_eq!(found.has_history, Some(false), "and the prices are not");
        assert!(!found.is_complete());
    }

    #[test]
    fn query_is_escaped() {
        assert!(YahooProvider::search_url("S&P 500").contains("S%26P%20500"));
    }

    #[test]
    #[ignore = "requires network"]
    fn resolves_a_real_isin() {
        let provider = YahooProvider::new();
        let found = provider.search("IE00B5BMR087").unwrap();
        assert_eq!(found[0].symbol, "CSSPX.MI");
        let profile = provider.profile(&found[0].symbol).unwrap().unwrap();
        assert_eq!(profile.currency.as_deref(), Some("EUR"));
    }
}
