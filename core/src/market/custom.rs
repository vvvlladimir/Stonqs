//! A quote source the user describes instead of one this build ships: a URL template and where
//! in the answer the dates and closes are. Data, not code — no script ever runs. See ADR-0054.

use super::{DateRange, Quote, QuoteProvider};
use crate::error::{Error, Result};
use crate::import::{ParseConfig, parse_csv, parse_date_with, parse_decimal};
use crate::model::Security;
use crate::money::{major_currency, normalize_currency};
use chrono::{DateTime, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::io::Read;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Duration;

/// Larger answers are refused rather than read: a daily series for decades fits many times over.
const MAX_BODY: u64 = 5 * 1024 * 1024;
/// A custom source's id always starts with this, so it can never shadow a shipped one.
pub const CUSTOM_PREFIX: &str = "custom:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CustomFormat {
    /// Two paths into a JSON answer, zipped by position: `$.values[*].datetime`, `$[*].close`.
    Json { date_path: String, close_path: String },
    /// Two columns of a CSV answer, by header name or zero-based index.
    Csv {
        date_column: String,
        close_column: String,
    },
}

/// What a custom source answers: an instrument's closes, or a currency pair's rates.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomRole {
    #[default]
    Quotes,
    /// The template takes `{BASE}` and `{QUOTE}`; each close is units of quote per one base.
    Fx,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomHeader {
    pub name: String,
    /// `{KEY}` inside it is replaced by the key saved for this source; the key is never stored here.
    pub value: String,
}

/// One user-described source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomSource {
    /// `custom:<slug>`; stored as `securities.data_source` like any other source id.
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub role: CustomRole,
    /// `https://…` with `{SYMBOL} {ISIN} {MIC} {CURRENCY} {FROM} {TO} {FROM:%s} {TO:%s} {KEY}`.
    pub url: String,
    #[serde(default)]
    pub headers: Vec<CustomHeader>,
    pub format: CustomFormat,
    /// A chrono format (`%d.%m.%Y`); absent means ISO dates or Unix seconds/milliseconds.
    #[serde(default)]
    pub date_format: Option<String>,
    /// Multiplies every close, for a source that answers in cents (`0.01`).
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub factor: Option<Decimal>,
    /// The currency the closes are in; absent means the instrument's own.
    #[serde(default)]
    pub currency: Option<String>,
}

impl CustomSource {
    /// Whether the definition can be asked at all; the reason is a code, the UI writes the words.
    pub fn validate(&self) -> std::result::Result<(), &'static str> {
        if !self.id.starts_with(CUSTOM_PREFIX) || self.id.len() == CUSTOM_PREFIX.len() {
            return Err("bad_id");
        }
        if !self.url.starts_with("https://") {
            return Err("not_https");
        }
        let (a, b) = match &self.format {
            CustomFormat::Json {
                date_path,
                close_path,
            } => (date_path, close_path),
            CustomFormat::Csv {
                date_column,
                close_column,
            } => (date_column, close_column),
        };
        if a.trim().is_empty() || b.trim().is_empty() {
            return Err("missing_path");
        }
        Ok(())
    }
}

/// `QuoteProvider::id` is `&'static str`; a user's id is interned once per distinct value.
fn intern(id: &str) -> &'static str {
    static IDS: Mutex<Option<HashSet<&'static str>>> = Mutex::new(None);
    let mut ids = IDS.lock().unwrap_or_else(|e| e.into_inner());
    let set = ids.get_or_insert_with(HashSet::new);
    if let Some(known) = set.get(id) {
        return known;
    }
    let leaked: &'static str = Box::leak(id.to_string().into_boxed_str());
    set.insert(leaked);
    leaked
}

pub struct CustomProvider {
    id: &'static str,
    def: CustomSource,
    key: Option<String>,
    http_timeout: Duration,
}

impl CustomProvider {
    pub fn new(def: CustomSource, key: Option<String>) -> Self {
        CustomProvider {
            id: intern(&def.id),
            def,
            key,
            http_timeout: Duration::from_secs(20),
        }
    }

    pub fn role(&self) -> CustomRole {
        self.def.role
    }

    fn fill_pair(&self, base: &str, quote: &str, range: DateRange) -> String {
        let pair = Security::new(base, base, quote, crate::model::SecurityKind::Other);
        self.fill(&self.def.url, &pair, range)
            .replace("{BASE}", base)
            .replace("{QUOTE}", quote)
    }

    fn fill(&self, template: &str, security: &Security, range: DateRange) -> String {
        let stamp = |d: NaiveDate| {
            d.and_hms_opt(0, 0, 0)
                .map(|t| t.and_utc().timestamp())
                .unwrap_or(0)
        };
        template
            .replace("{SYMBOL}", &urlencode(security.provider_symbol()))
            .replace("{ISIN}", security.isin.as_deref().unwrap_or(""))
            .replace("{MIC}", security.mic.as_deref().unwrap_or(""))
            .replace("{CURRENCY}", &security.currency)
            .replace("{FROM:%s}", &stamp(range.from).to_string())
            .replace("{TO:%s}", &stamp(range.to).to_string())
            .replace("{FROM}", &range.from.to_string())
            .replace("{TO}", &range.to.to_string())
            .replace("{KEY}", self.key.as_deref().unwrap_or(""))
    }

    fn get(&self, url: &str) -> Result<String> {
        if !url.starts_with("https://") {
            return Err(Error::Invalid("a custom source must use https".into()));
        }
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.http_timeout))
            .max_redirects(0)
            .build()
            .new_agent();
        let mut request = agent.get(url);
        for header in &self.def.headers {
            let value = header.value.replace("{KEY}", self.key.as_deref().unwrap_or(""));
            request = request.header(header.name.as_str(), value.as_str());
        }
        let mut body = String::new();
        request
            .call()?
            .body_mut()
            .as_reader()
            .take(MAX_BODY)
            .read_to_string(&mut body)?;
        Ok(body)
    }

    /// Reads an answer into `(date, close)` pairs, oldest first, before currency and factor.
    pub fn parse(&self, body: &str) -> Result<Vec<(NaiveDate, Decimal)>> {
        let bad = |detail: String| Error::BadProviderData {
            provider: self.id,
            detail,
        };
        let pairs: Vec<(String, String)> = match &self.def.format {
            CustomFormat::Json {
                date_path,
                close_path,
            } => {
                let json: Value =
                    serde_json::from_str(body).map_err(|e| bad(format!("malformed JSON: {e}")))?;
                let dates = select(&json, date_path).map_err(|e| bad(format!("date path: {e}")))?;
                let closes = select(&json, close_path).map_err(|e| bad(format!("close path: {e}")))?;
                if dates.len() != closes.len() {
                    return Err(bad(format!("{} dates but {} closes", dates.len(), closes.len())));
                }
                dates
                    .into_iter()
                    .map(scalar)
                    .zip(closes.into_iter().map(scalar))
                    .collect()
            }
            CustomFormat::Csv {
                date_column,
                close_column,
            } => {
                let parsed = parse_csv(body.as_bytes(), &ParseConfig::default())?;
                let column = |name: &str| {
                    parsed
                        .headers
                        .iter()
                        .position(|h| h.trim().eq_ignore_ascii_case(name.trim()))
                        .or_else(|| name.trim().parse::<usize>().ok())
                        .ok_or_else(|| bad(format!("no column {name:?}")))
                };
                let (d, c) = (column(date_column)?, column(close_column)?);
                parsed
                    .rows
                    .iter()
                    .filter_map(|row| Some((row.get(d)?.clone(), row.get(c)?.clone())))
                    .collect()
            }
        };
        let mut out: Vec<(NaiveDate, Decimal)> = pairs
            .iter()
            .filter_map(|(date, close)| {
                let date = self.date(date)?;
                let close = Decimal::from_str(close.trim())
                    .ok()
                    .or_else(|| parse_decimal(close, '.'))?;
                (close > Decimal::ZERO).then_some((date, close))
            })
            .collect();
        if out.is_empty() && !pairs.is_empty() {
            return Err(bad("no row had both a readable date and a positive close".into()));
        }
        out.sort_by_key(|(d, _)| *d);
        out.dedup_by_key(|(d, _)| *d);
        Ok(out)
    }

    fn date(&self, value: &str) -> Option<NaiveDate> {
        let value = value.trim().trim_matches('"');
        if let Some(format) = &self.def.date_format {
            return parse_date_with(value, format);
        }
        if let Ok(number) = value.parse::<i64>() {
            // Past 10^11 seconds is the year 5138: such a number is milliseconds.
            let seconds = if number.abs() >= 100_000_000_000 {
                number / 1000
            } else {
                number
            };
            return DateTime::from_timestamp(seconds, 0).map(|t| t.date_naive());
        }
        NaiveDate::parse_from_str(value.get(..10)?, "%Y-%m-%d").ok()
    }
}

impl QuoteProvider for CustomProvider {
    fn id(&self) -> &'static str {
        self.id
    }

    fn fetch(&self, security: &Security, range: DateRange) -> Result<Vec<Quote>> {
        let body = self.get(&self.fill(&self.def.url, security, range))?;
        let (currency, minor) = match &self.def.currency {
            Some(raw) => major_currency(raw),
            None => (normalize_currency(&security.currency), Decimal::ONE),
        };
        let factor = self.def.factor.unwrap_or(Decimal::ONE) * minor;
        Ok(self
            .parse(&body)?
            .into_iter()
            .filter(|(d, _)| *d >= range.from && *d <= range.to)
            .map(|(date, close)| Quote {
                security_id: security.id.clone(),
                date,
                close: close * factor,
                currency: currency.clone(),
                source: self.id.to_string(),
            })
            .collect())
    }
}

impl crate::fx::FxProvider for CustomProvider {
    fn id(&self) -> &'static str {
        self.id
    }

    fn fetch(&self, base: &str, quote: &str, range: DateRange) -> Result<Vec<crate::fx::FxRate>> {
        let (base, quote) = (normalize_currency(base), normalize_currency(quote));
        if base == quote {
            return Ok(Vec::new());
        }
        let body = self.get(&self.fill_pair(&base, &quote, range))?;
        let factor = self.def.factor.unwrap_or(Decimal::ONE);
        Ok(self
            .parse(&body)?
            .into_iter()
            .filter(|(d, _)| *d >= range.from && *d <= range.to)
            .map(|(date, rate)| crate::fx::FxRate::new(&base, &quote, date, rate * factor))
            .collect())
    }
}

fn scalar(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The part of JSONPath a price feed needs: `$`, `.name`, `['name']`, `[n]`, `[*]`, `.*`.
fn select<'a>(root: &'a Value, path: &str) -> std::result::Result<Vec<&'a Value>, String> {
    let rest = path.trim().strip_prefix('$').ok_or("a path starts with $")?;
    let mut current = vec![root];
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        let step: Step = match c {
            '.' => {
                let mut name = String::new();
                while let Some(&n) = chars.peek() {
                    if n == '.' || n == '[' {
                        break;
                    }
                    name.push(n);
                    chars.next();
                }
                if name == "*" { Step::All } else { Step::Key(name) }
            }
            '[' => {
                let mut inner = String::new();
                for n in chars.by_ref() {
                    if n == ']' {
                        break;
                    }
                    inner.push(n);
                }
                let inner = inner.trim();
                if inner == "*" {
                    Step::All
                } else if let Ok(i) = inner.parse::<usize>() {
                    Step::Index(i)
                } else {
                    Step::Key(inner.trim_matches(|q| q == '\'' || q == '"').to_string())
                }
            }
            other => return Err(format!("unexpected {other:?}")),
        };
        current = current
            .into_iter()
            .flat_map(|v| -> Vec<&Value> {
                match (&step, v) {
                    (Step::Key(k), Value::Object(map)) => map.get(k).into_iter().collect(),
                    (Step::Index(i), Value::Array(items)) => items.get(*i).into_iter().collect(),
                    (Step::All, Value::Array(items)) => items.iter().collect(),
                    (Step::All, Value::Object(map)) => map.values().collect(),
                    _ => Vec::new(),
                }
            })
            .collect();
    }
    Ok(current)
}

enum Step {
    Key(String),
    Index(usize),
    All,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SecurityKind;
    use rust_decimal_macros::dec;

    fn source(format: CustomFormat) -> CustomProvider {
        CustomProvider::new(
            CustomSource {
                id: "custom:feed".into(),
                label: "Feed".into(),
                role: CustomRole::Quotes,
                url: "https://example.com/{SYMBOL}?from={FROM}&to={TO}&t={FROM:%s}&k={KEY}".into(),
                headers: vec![],
                format,
                date_format: None,
                factor: None,
                currency: None,
            },
            Some("secret".into()),
        )
    }

    fn json(date: &str, close: &str) -> CustomProvider {
        source(CustomFormat::Json {
            date_path: date.into(),
            close_path: close.into(),
        })
    }

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn fills_the_url_template() {
        let sec = Security::new("A B", "A", "EUR", SecurityKind::Etf).with_source("custom:feed", "A B");
        let url = json("$", "$").fill(
            &json("$", "$").def.url,
            &sec,
            DateRange::new(d("2024-06-03"), d("2024-06-07")),
        );
        assert_eq!(
            url,
            "https://example.com/A%20B?from=2024-06-03&to=2024-06-07&t=1717372800&k=secret"
        );
    }

    #[test]
    fn zips_a_list_of_objects_newest_first_into_oldest_first() {
        let body = r#"{"values":[{"datetime":"2024-06-04","close":"101.5"},{"datetime":"2024-06-03","close":"100"}]}"#;
        let rows = json("$.values[*].datetime", "$.values[*].close")
            .parse(body)
            .unwrap();
        assert_eq!(
            rows,
            vec![(d("2024-06-03"), dec!(100)), (d("2024-06-04"), dec!(101.5))]
        );
    }

    #[test]
    fn zips_parallel_arrays_with_unix_seconds() {
        // 1717372800 = 2024-06-03T00:00Z, as Yahoo's chart API prints its timestamps.
        let body = r#"{"chart":{"result":[{"timestamp":[1717372800],"close":[194.03]}]}}"#;
        let rows = json("$.chart.result[0].timestamp[*]", "$.chart.result[0].close[*]")
            .parse(body)
            .unwrap();
        assert_eq!(rows, vec![(d("2024-06-03"), dec!(194.03))]);
    }

    #[test]
    fn reads_csv_columns_by_name() {
        let feed = source(CustomFormat::Csv {
            date_column: "Date".into(),
            close_column: "Close".into(),
        });
        let rows = feed
            .parse("Date,Open,Close\n2024-06-03,1,10.5\n2024-06-04,1,11\n")
            .unwrap();
        assert_eq!(rows[1], (d("2024-06-04"), dec!(11)));
    }

    #[test]
    fn a_path_that_finds_nothing_is_the_source_s_fault_not_an_empty_week() {
        let err =
            json("$.values[*].date", "$.values[*].close").parse(r#"{"values":[{"date":"x","close":"y"}]}"#);
        assert!(matches!(err, Err(Error::BadProviderData { .. })));
    }

    #[test]
    fn a_definition_must_be_https_and_name_both_paths() {
        let mut def = json("$[*].d", "$[*].c").def;
        assert_eq!(def.validate(), Ok(()));
        def.url = "http://example.com".into();
        assert_eq!(def.validate(), Err("not_https"));
    }

    #[test]
    #[ignore = "requires network"]
    fn reads_a_real_feed_described_as_data() {
        let feed = CustomProvider::new(
            CustomSource {
                id: "custom:eod".into(),
                label: "EOD".into(),
                role: CustomRole::Quotes,
                url: "https://eodhd.com/api/eod/{SYMBOL}?from={FROM}&to={TO}&fmt=json&api_token={KEY}".into(),
                headers: vec![],
                format: CustomFormat::Json {
                    date_path: "$[*].date".into(),
                    close_path: "$[*].close".into(),
                },
                date_format: None,
                factor: None,
                currency: Some("USD".into()),
            },
            Some("demo".into()),
        );
        let aapl =
            Security::new("AAPL", "Apple", "USD", SecurityKind::Stock).with_source("custom:eod", "AAPL.US");
        let quotes = feed
            .fetch(&aapl, DateRange::new(d("2024-06-03"), d("2024-06-07")))
            .unwrap();
        assert_eq!((quotes.len(), quotes[0].close), (5, dec!(194.03)));
    }

    #[test]
    fn a_pair_template_names_both_currencies() {
        let mut feed = json("$", "$");
        feed.def.url = "https://fx.example/{BASE}{QUOTE}?from={FROM}".into();
        let url = feed.fill_pair("EUR", "ARS", DateRange::new(d("2024-06-03"), d("2024-06-07")));
        assert_eq!(url, "https://fx.example/EURARS?from=2024-06-03");
    }
}
