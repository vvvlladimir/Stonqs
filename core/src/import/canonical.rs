//! The app's own transaction file: what every reader produces and what the app writes back.
//!
//! It is a *third reader* rather than a second import (ADR-0066): the rows are flattened into the
//! same `ParsedCsv` the CSV and Flex readers hand over, with this module's column names and a
//! fixed mapping, so the wizard, the overrides, the identity rules and the commit are the ones
//! every file goes through.
//!
//! Two spellings, one meaning. JSON is the interchange — a reader, a plugin or somebody's script
//! writes it. A CSV whose headers are these same names needs no reader at all: they are the
//! canonical header aliases, so ordinary detection maps them one to one.
//!
//! Nothing here names an internal id. An account is its name and an instrument its ticker and
//! ISIN, because a file that carried database ids would only ever import back into the database
//! it came from.

use super::mapping::{AmountBasis, AmountSign, ImportField, ImportMapping};
use super::parse::{ParseConfig, ParsedCsv};
use crate::error::{Error, Result};
use crate::model::{Account, Security, Transaction, TransactionKind};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// What the file calls itself. A JSON file that does not say this is not ours.
pub const FORMAT: &str = "stonqs.transactions";

/// The only version this build writes, and the newest it reads.
pub const VERSION: u32 = 1;

/// Column names, which are also the canonical header aliases — the first alias of each field,
/// so a CSV written with these headers is understood by plain detection. Public because they are
/// the format's own vocabulary: a reader that flattens something else into this table
/// (`ibflex`) names its columns from here rather than inventing a second spelling.
pub const DATE: &str = "date";
pub const KIND: &str = "type";
pub const ACCOUNT: &str = "account";
pub const SYMBOL: &str = "symbol";
pub const ISIN: &str = "isin";
pub const NAME: &str = "name";
pub const QUANTITY: &str = "quantity";
pub const PRICE: &str = "price";
pub const AMOUNT: &str = "amount";
pub const FEE: &str = "fee";
pub const FEE_CURRENCY: &str = "fee currency";
pub const TAX: &str = "tax";
pub const TAX_CURRENCY: &str = "tax currency";
pub const CURRENCY: &str = "currency";
pub const FX_RATE: &str = "fx rate";
pub const LINK_ID: &str = "link id";
pub const EXTERNAL_ID: &str = "external id";
pub const NOTE: &str = "note";

const COLUMNS: &[&str] = &[
    DATE,
    KIND,
    ACCOUNT,
    SYMBOL,
    ISIN,
    NAME,
    QUANTITY,
    PRICE,
    AMOUNT,
    FEE,
    FEE_CURRENCY,
    TAX,
    TAX_CURRENCY,
    CURRENCY,
    FX_RATE,
    LINK_ID,
    EXTERNAL_ID,
    NOTE,
];

/// One operation, as text. Every value is a string on purpose: the file states what the broker
/// stated, and the same parsing that reads a CSV reads this — one place where a number becomes a
/// `Decimal`, not two.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalRow {
    pub date: String,
    pub kind: String,
    /// The account's **name**, as the portfolio that wrote the file knew it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub isin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee_currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_currency: Option<String>,
    pub currency: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fx_rate: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The file itself: what it is, which version of it, and the operations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalFile {
    pub format: String,
    pub version: u32,
    pub rows: Vec<CanonicalRow>,
}

/// Whether the bytes are one of our own transaction files. Asked before the CSV reader, so it
/// has to be cheap and certain: a JSON document that names the format in its first field.
pub fn is_canonical(content: &[u8]) -> bool {
    let head = &content[..content.len().min(512)];
    let text = String::from_utf8_lossy(head);
    text.trim_start().starts_with('{') && text.contains(FORMAT)
}

/// Reads the file into the table every other reader produces.
pub fn parse_canonical(content: &[u8]) -> Result<ParsedCsv> {
    let file: CanonicalFile = serde_json::from_slice(content)
        .map_err(|e| Error::Invalid(format!("this is not a readable {FORMAT} file: {e}")))?;
    if file.format != FORMAT {
        return Err(Error::Invalid(format!(
            "the file calls itself {:?}, which this build does not read",
            file.format
        )));
    }
    if file.version > VERSION {
        return Err(Error::Invalid(format!(
            "the file is version {}, newer than the {VERSION} this build reads — update the app",
            file.version
        )));
    }

    let rows = file.rows.iter().map(row_cells).collect();
    Ok(ParsedCsv {
        headers: COLUMNS.iter().map(|c| c.to_string()).collect(),
        rows,
        config: ParseConfig {
            // The format states both, so nothing downstream votes on them.
            date_format: Some("%Y-%m-%d".to_string()),
            decimal_separator: Some('.'),
            has_header: Some(true),
            ..ParseConfig::default()
        },
        problems: Vec::new(),
    })
}

fn row_cells(row: &CanonicalRow) -> Vec<String> {
    let text = |value: &Option<String>| value.clone().unwrap_or_default();
    vec![
        row.date.clone(),
        row.kind.clone(),
        text(&row.account),
        text(&row.symbol),
        text(&row.isin),
        text(&row.name),
        text(&row.quantity),
        text(&row.price),
        text(&row.amount),
        text(&row.fee),
        text(&row.fee_currency),
        text(&row.tax),
        text(&row.tax_currency),
        row.currency.clone(),
        text(&row.fx_rate),
        text(&row.link_id),
        text(&row.external_id),
        text(&row.note),
    ]
}

/// The fixed layout of what `parse_canonical` produces. The columns are this module's own, so
/// there is nothing for a user to lay out differently — and every operation the model has is
/// spelled here, because the file writes the model's own wording rather than a broker's.
pub fn mapping() -> ImportMapping {
    let mut mapping = ImportMapping {
        // The file states the amount before charges and signs nothing: this is our own shape,
        // not a broker's convention to be inferred.
        amount_sign: Some(AmountSign::Unsigned),
        amount_basis: Some(AmountBasis::Gross),
        ..ImportMapping::default()
    };
    for (field, column) in [
        (ImportField::Date, DATE),
        (ImportField::Kind, KIND),
        (ImportField::Account, ACCOUNT),
        (ImportField::Symbol, SYMBOL),
        (ImportField::Isin, ISIN),
        (ImportField::Name, NAME),
        (ImportField::Quantity, QUANTITY),
        (ImportField::Price, PRICE),
        (ImportField::Amount, AMOUNT),
        (ImportField::Fee, FEE),
        (ImportField::FeeCurrency, FEE_CURRENCY),
        (ImportField::Tax, TAX),
        (ImportField::TaxCurrency, TAX_CURRENCY),
        (ImportField::Currency, CURRENCY),
        (ImportField::FxRate, FX_RATE),
        (ImportField::LinkId, LINK_ID),
        (ImportField::ExternalId, EXTERNAL_ID),
        (ImportField::Note, NOTE),
    ] {
        mapping.set_column(field, Some(column));
    }
    for kind in TransactionKind::ALL {
        mapping = mapping.with_kind_alias(kind.as_str(), *kind);
    }
    mapping
}

/// Writes the operations as a file of our own. Ids are left behind: an account travels as its
/// name and an instrument as its ticker and ISIN, so the file imports into another portfolio.
pub fn canonical_to_file(
    transactions: &[Transaction],
    accounts: &[Account],
    securities: &[Security],
) -> Result<String> {
    let file = CanonicalFile {
        format: FORMAT.to_string(),
        version: VERSION,
        rows: transactions
            .iter()
            .map(|t| to_row(t, accounts, securities))
            .collect(),
    };
    serde_json::to_string_pretty(&file).map_err(|e| Error::Invalid(e.to_string()))
}

fn to_row(t: &Transaction, accounts: &[Account], securities: &[Security]) -> CanonicalRow {
    let security = t
        .security_id
        .as_ref()
        .and_then(|id| securities.iter().find(|s| s.id == *id));
    let number = |value: Decimal| (!value.is_zero()).then(|| value.normalize().to_string());

    CanonicalRow {
        date: t.date.to_string(),
        kind: t.kind.as_str().to_string(),
        account: accounts
            .iter()
            .find(|a| a.id == t.account_id)
            .map(|a| a.name.clone()),
        symbol: security.map(|s| s.symbol.clone()),
        isin: security.and_then(|s| s.isin.clone()),
        name: security.map(|s| s.name.clone()),
        quantity: number(t.quantity),
        price: number(t.price),
        // The amount is written even when it is zero: a row with no amount at all reads as a
        // missing value rather than as a delivery that moved no money.
        amount: Some(t.amount.normalize().to_string()),
        fee: number(t.fees),
        fee_currency: t.fee_currency.clone(),
        tax: number(t.taxes),
        tax_currency: t.tax_currency.clone(),
        currency: t.currency.clone(),
        fx_rate: t.fx_rate_to_base.map(|r| r.normalize().to_string()),
        link_id: t.link_id.clone(),
        external_id: t.external_id.clone(),
        note: t.note.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SecurityKind;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    fn day() -> NaiveDate {
        NaiveDate::from_ymd_opt(2024, 3, 1).unwrap()
    }

    #[test]
    fn a_written_file_reads_back_as_the_same_table() {
        let cash = Account::deposit("Broker · cash", "EUR");
        let depot = Account::securities("Broker", "EUR", &cash.id);
        let apple = Security::new("AAPL", "Apple Inc.", "USD", SecurityKind::Stock);

        let buy = Transaction::buy(&depot.id, &apple.id, day(), dec!(10), dec!(100), "EUR")
            .with_fees_in(dec!(12), "USD")
            .with_external_id("TR-1");

        let text = canonical_to_file(&[buy], &[cash, depot], &[apple]).unwrap();
        assert!(is_canonical(text.as_bytes()));

        let parsed = parse_canonical(text.as_bytes()).unwrap();
        assert_eq!(parsed.value(0, "type"), Some("BUY"));
        assert_eq!(parsed.value(0, "account"), Some("Broker"));
        assert_eq!(parsed.value(0, "symbol"), Some("AAPL"));
        assert_eq!(parsed.value(0, "amount"), Some("1000"));
        assert_eq!(parsed.value(0, "fee"), Some("12"));
        assert_eq!(parsed.value(0, "fee currency"), Some("USD"));
        assert_eq!(parsed.value(0, "external id"), Some("TR-1"));
    }

    #[test]
    fn the_columns_are_the_names_ordinary_detection_already_knows() {
        // The CSV spelling of this format needs no reader: its headers are the canonical
        // aliases, so `detect` maps every one of them to the field it is named after.
        let headers: Vec<String> = COLUMNS.iter().map(|c| c.to_string()).collect();
        let detected = ImportMapping::detect(&headers);
        for (field, column) in mapping().columns {
            assert_eq!(
                detected.column(field),
                Some(column.as_str()),
                "{field:?} is not detected from its own canonical header"
            );
        }
    }

    #[test]
    fn a_file_from_a_newer_version_is_refused_rather_than_half_read() {
        let text = format!(r#"{{"format":"{FORMAT}","version":{},"rows":[]}}"#, VERSION + 1);
        let error = parse_canonical(text.as_bytes()).unwrap_err().to_string();
        assert!(error.contains("newer"), "{error}");
    }

    #[test]
    fn json_that_is_not_ours_is_not_claimed() {
        assert!(!is_canonical(b"{\"format\":\"something.else\",\"rows\":[]}"));
        assert!(!is_canonical(b"date,type,amount\n2024-01-01,BUY,100\n"));
    }
}
