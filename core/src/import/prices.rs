use super::mapping::normalize_alias;
use super::parse::{ImportProblem, ParsedCsv, ProblemCode, parse_date_with, parse_decimal};
use crate::market::Quote;
use crate::model::Security;
use crate::money::normalize_currency;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Column roles recognized by the price importer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PriceField {
    Date,
    Symbol,
    /// Closing price stored by the core.
    Close,
    Currency,
}

/// Mapping from CSV columns to price fields.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceMapping {
    pub columns: BTreeMap<PriceField, String>,
    pub security_id: Option<String>,
    pub default_currency: Option<String>,
}

impl PriceMapping {
    /// Detects column roles from normalized headers.
    pub fn detect(headers: &[String]) -> Self {
        let mut columns = BTreeMap::new();
        let candidates: &[(PriceField, &[&str])] = &[
            (PriceField::Date, &["date", "datum", "дата"]),
            (PriceField::Symbol, &["symbol", "ticker", "тикер", "isin"]),
            (
                PriceField::Close,
                &["close", "price", "kurs", "schluss", "цена", "adjclose", "closing"],
            ),
            (PriceField::Currency, &["currency", "währung", "валюта", "ccy"]),
        ];
        for (field, aliases) in candidates {
            if let Some(header) = headers.iter().find(|h| {
                let n = normalize_alias(h).to_lowercase();
                aliases.iter().any(|a| n == *a || n.contains(a))
            }) {
                columns.insert(*field, header.clone());
            }
        }
        PriceMapping {
            columns,
            ..PriceMapping::default()
        }
    }

    pub fn with_security(mut self, security_id: &str) -> Self {
        self.security_id = Some(security_id.to_string());
        self
    }

    pub fn with_currency(mut self, currency: &str) -> Self {
        self.default_currency = Some(normalize_currency(currency));
        self
    }

    fn column(&self, field: PriceField) -> Option<&str> {
        self.columns.get(&field).map(String::as_str)
    }
}

/// Parsed quotes plus row-level diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceImport {
    pub quotes: Vec<Quote>,
    pub problems: Vec<ImportProblem>,
    pub unknown_symbols: Vec<String>,
}

/// Builds quotes without writing them to storage.
pub fn build_price_import(
    parsed: &ParsedCsv,
    mapping: &PriceMapping,
    securities: &[Security],
) -> PriceImport {
    let decimal_separator = parsed.config.decimal_separator.unwrap_or('.');
    let mut problems = Vec::new();
    let mut quotes = Vec::new();
    let mut unknown: BTreeSet<String> = BTreeSet::new();

    let by_symbol: BTreeMap<String, &Security> = securities
        .iter()
        .map(|s| (normalize_alias(&s.symbol), s))
        .collect();
    let by_isin: BTreeMap<String, &Security> = securities
        .iter()
        .filter_map(|s| s.isin.as_ref().map(|i| (normalize_alias(i), s)))
        .collect();

    let Some(date_column) = mapping.column(PriceField::Date) else {
        problems.push(ImportProblem::file(
            ProblemCode::MissingColumn,
            "no date column is mapped",
        ));
        return PriceImport {
            quotes,
            problems,
            unknown_symbols: Vec::new(),
        };
    };
    let Some(close_column) = mapping.column(PriceField::Close) else {
        problems.push(ImportProblem::file(
            ProblemCode::MissingColumn,
            "no price column is mapped",
        ));
        return PriceImport {
            quotes,
            problems,
            unknown_symbols: Vec::new(),
        };
    };
    let Some(date_format) = parsed.config.date_format.as_deref() else {
        problems.push(ImportProblem::file(
            ProblemCode::BadDate,
            "the date format was not detected — set it explicitly",
        ));
        return PriceImport {
            quotes,
            problems,
            unknown_symbols: Vec::new(),
        };
    };

    for (index, _) in parsed.rows.iter().enumerate() {
        let number = index + 1;
        let value = |column: &str| {
            parsed
                .value(index, column)
                .map(str::trim)
                .filter(|v| !v.is_empty())
        };

        let Some(date) = value(date_column).and_then(|v| parse_date_with(v, date_format)) else {
            problems.push(ImportProblem::cell(
                ProblemCode::BadDate,
                number,
                date_column,
                "the date did not parse",
            ));
            continue;
        };
        let Some(close) = value(close_column).and_then(|v| parse_decimal(v, decimal_separator)) else {
            problems.push(ImportProblem::cell(
                ProblemCode::NotANumber,
                number,
                close_column,
                "the price did not parse",
            ));
            continue;
        };

        let security = match mapping.column(PriceField::Symbol).and_then(value) {
            Some(symbol) => {
                let found = by_symbol
                    .get(&normalize_alias(symbol))
                    .or_else(|| by_isin.get(&normalize_alias(symbol)))
                    .copied();
                if found.is_none() {
                    unknown.insert(symbol.to_string());
                    problems.push(ImportProblem::row(
                        ProblemCode::MissingValue,
                        number,
                        format!("security {symbol:?} is not in the database"),
                    ));
                }
                found.map(|s| (s.id.clone(), s.currency.clone()))
            }

            None => match &mapping.security_id {
                Some(id) => securities
                    .iter()
                    .find(|s| s.id == *id)
                    .map(|s| (s.id.clone(), s.currency.clone())),
                None => {
                    problems.push(ImportProblem::row(
                        ProblemCode::MissingValue,
                        number,
                        "no security is given",
                    ));
                    None
                }
            },
        };
        let Some((security_id, security_currency)) = security else {
            continue;
        };

        let currency = mapping
            .column(PriceField::Currency)
            .and_then(value)
            .map(normalize_currency)
            .or_else(|| mapping.default_currency.clone())
            .unwrap_or(security_currency);

        quotes.push(Quote {
            security_id,
            date,
            close,
            currency,
            // Preserve manual CSV ownership when provider data is refreshed.
            source: "csv".into(),
        });
    }

    PriceImport {
        quotes,
        problems,
        unknown_symbols: unknown.into_iter().collect(),
    }
}
