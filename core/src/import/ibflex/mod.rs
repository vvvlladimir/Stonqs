//! Interactive Brokers Flex Query XML, flattened into the canonical columns and a fixed mapping
//! (ADR-0061).

use super::mapping::{AmountSign, ImportField, ImportMapping};
use super::parse::{ImportProblem, ParseConfig, ParsedCsv, ProblemCode, decode, parse_date_any};
use crate::error::{Error, Result};
use crate::model::TransactionKind;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use std::collections::BTreeMap;

use super::canonical::{
    ACCOUNT, AMOUNT, CURRENCY, DATE, FEE, FX_RATE, ISIN, KIND, LINK_ID as LINK, NAME, NOTE, PRICE, QUANTITY,
    SYMBOL, TAX,
};

const COLUMNS: &[&str] = &[
    DATE, KIND, SYMBOL, ISIN, NAME, QUANTITY, PRICE, AMOUNT, FEE, TAX, CURRENCY, FX_RATE, ACCOUNT, LINK, NOTE,
];

/// Wordings for rows the model has no operation for; shipped on the skip list so they stay visible.
pub const CORPORATE_ACTION: &str = "Corporate action";
pub const DERIVATIVE_TRADE: &str = "Derivative trade";
pub const CANCELLED_TRADE: &str = "Cancelled trade";
/// The two legs of a forex trade. Mapped to `TransferIn`; the negative leg flips to `TransferOut`.
pub const CURRENCY_CONVERSION: &str = "Currency conversion";

/// Whether the bytes are a Flex statement rather than a delimited file.
pub fn is_flex(content: &[u8]) -> bool {
    let head = &content[..content.len().min(4096)];
    let text = String::from_utf8_lossy(head);
    text.contains("<FlexQueryResponse")
}

/// The fixed mapping for what `parse_flex` produces.
pub fn mapping() -> ImportMapping {
    use TransactionKind::*;
    let kinds: &[(&str, TransactionKind)] = &[
        // Trades. `buySell` is already the model's wording.
        ("BUY", Buy),
        ("SELL", Sell),
        (CURRENCY_CONVERSION, TransferIn),
        // Cash transactions, spelled as Interactive Brokers spells them. A wording that spans
        // both directions is mapped to one side and flipped by the sign of the amount.
        ("Dividends", Dividend),
        ("Payment In Lieu Of Dividends", Dividend),
        ("Withholding Tax", Tax),
        ("Broker Interest Received", Interest),
        ("Broker Interest Paid", InterestCharge),
        ("Bond Interest Received", Interest),
        ("Bond Interest Paid", InterestCharge),
        ("Deposits/Withdrawals", Deposit),
        ("Deposits & Withdrawals", Deposit),
        ("Deposits and Withdrawals", Deposit),
        ("Other Fees", Fee),
        ("Advisor Fees", Fee),
        ("Commission Adjustments", Fee),
    ];

    let mut mapping = ImportMapping {
        // Flex always prints a dot and the rows are signed; neither is worth a vote over a file
        // that may hold three rows.
        amount_sign: Some(AmountSign::Signed),
        ..ImportMapping::default()
    };
    for (field, column) in [
        (ImportField::Date, DATE),
        (ImportField::Kind, KIND),
        (ImportField::Symbol, SYMBOL),
        (ImportField::Isin, ISIN),
        (ImportField::Name, NAME),
        (ImportField::Quantity, QUANTITY),
        (ImportField::Price, PRICE),
        (ImportField::Amount, AMOUNT),
        (ImportField::Fee, FEE),
        (ImportField::Tax, TAX),
        (ImportField::Currency, CURRENCY),
        (ImportField::FxRate, FX_RATE),
        (ImportField::Account, ACCOUNT),
        (ImportField::LinkId, LINK),
        (ImportField::Note, NOTE),
    ] {
        mapping.set_column(field, Some(column));
    }
    for (value, kind) in kinds {
        mapping = mapping.with_kind_alias(value, *kind);
    }
    for value in [CORPORATE_ACTION, DERIVATIVE_TRADE, CANCELLED_TRADE] {
        mapping = mapping.with_ignored_kind(value);
    }
    mapping
}

/// Reads a Flex statement into the same shape a CSV parses to.
pub fn parse_flex(content: &[u8]) -> Result<ParsedCsv> {
    let mut problems = Vec::new();
    let text = decode(content, &mut problems);
    let records = read_records(&text)?;

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut builder = Builder::new(&records);
    for record in &records {
        builder.push(record, &mut rows, &mut problems);
    }

    if rows.is_empty() {
        return Err(Error::Invalid(
            "the Flex statement holds no trades, cash transactions or corporate actions: \
             check which sections the query selects"
                .into(),
        ));
    }

    Ok(ParsedCsv {
        headers: COLUMNS.iter().map(|c| c.to_string()).collect(),
        rows,
        config: ParseConfig {
            // Dates are normalised to ISO here, so nothing downstream has to guess the query's
            // own date format — the user picks that in the web office and we never see it.
            date_format: Some("%Y-%m-%d".to_string()),
            decimal_separator: Some('.'),
            has_header: Some(true),
            ..ParseConfig::default()
        },
        problems,
    })
}

/// One element worth reading, with its attributes.
struct Record {
    section: Section,
    attrs: BTreeMap<String, String>,
    /// The statement's own account, used when a row does not repeat it.
    statement_account: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Account,
    SecurityInfo,
    Trade,
    Cash,
    CorporateAction,
}

impl Record {
    fn get(&self, key: &str) -> &str {
        self.attrs.get(key).map(String::as_str).unwrap_or_default()
    }

    /// The first attribute of the list that carries anything.
    fn any(&self, keys: &[&str]) -> &str {
        keys.iter()
            .map(|k| self.get(k))
            .find(|v| !v.is_empty())
            .unwrap_or_default()
    }

    /// What the user calls this account. The alias is theirs; the id is the broker's.
    fn account(&self) -> String {
        let named = self.any(&["acctAlias", "accountId"]);
        if named.is_empty() {
            self.statement_account.clone()
        } else {
            named.to_string()
        }
    }
}

fn read_records(text: &str) -> Result<Vec<Record>> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);

    let mut records = Vec::new();
    let mut statement_account = String::new();
    loop {
        let event = reader
            .read_event()
            .map_err(|e| Error::Invalid(format!("the Flex statement is not valid XML: {e}")))?;
        let start: &BytesStart = match &event {
            Event::Start(e) | Event::Empty(e) => e,
            Event::Eof => break,
            _ => continue,
        };
        let name = start.name();
        let section = match name.as_ref() {
            "FlexStatement" => {
                statement_account = attributes(start).get("accountId").cloned().unwrap_or_default();
                continue;
            }
            "AccountInformation" => Section::Account,
            "SecurityInfo" => Section::SecurityInfo,
            "Trade" => Section::Trade,
            "CashTransaction" => Section::Cash,
            "CorporateAction" => Section::CorporateAction,
            _ => continue,
        };
        records.push(Record {
            section,
            attrs: attributes(start),
            statement_account: statement_account.clone(),
        });
    }
    Ok(records)
}

fn attributes(start: &BytesStart) -> BTreeMap<String, String> {
    start
        .attributes()
        .filter_map(|a| a.ok())
        .filter_map(|a| {
            let value = a
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .ok()?
                .trim()
                .to_string();
            Some((a.key.as_ref().to_string(), value))
        })
        .collect()
}

/// What the whole file has to be read before a single row can be written.
struct Builder<'a> {
    /// A conid the trade rows can look an ISIN up by, from the instrument section.
    isin_by_conid: BTreeMap<&'a str, &'a str>,
    /// An execution is a fill and an order is the trade the user placed. Asking for both prints
    /// each purchase twice, so the coarser level wins whenever the file carries it.
    trade_level: &'static str,
    /// Distinguishes the legs of two forex trades that agree in every visible value.
    forex_legs: usize,
}

impl<'a> Builder<'a> {
    fn new(records: &'a [Record]) -> Self {
        let isin_by_conid = records
            .iter()
            .filter(|r| r.section == Section::SecurityInfo)
            .filter(|r| !r.get("conid").is_empty() && !r.get("isin").is_empty())
            .map(|r| (r.get("conid"), r.get("isin")))
            .collect();

        let has_orders = records
            .iter()
            .any(|r| r.section == Section::Trade && r.get("levelOfDetail").eq_ignore_ascii_case("ORDER"));

        Builder {
            isin_by_conid,
            trade_level: if has_orders { "ORDER" } else { "EXECUTION" },
            forex_legs: 0,
        }
    }

    fn push(&mut self, record: &Record, rows: &mut Vec<Vec<String>>, problems: &mut Vec<ImportProblem>) {
        match record.section {
            Section::Trade => self.trade(record, rows, problems),
            Section::Cash => rows.push(self.cash(record)),
            Section::CorporateAction => rows.push(self.corporate_action(record)),
            Section::Account | Section::SecurityInfo => {}
        }
    }

    fn trade(&mut self, r: &Record, rows: &mut Vec<Vec<String>>, problems: &mut Vec<ImportProblem>) {
        let level = r.get("levelOfDetail");
        // A closed lot and a summary line describe rows that are already in the file.
        if !level.is_empty() && !level.eq_ignore_ascii_case(self.trade_level) {
            return;
        }
        if r.get("assetCategory").eq_ignore_ascii_case("CASH") {
            self.forex(r, rows, problems);
            return;
        }

        let cancelled = r.get("buySell").contains('(');
        let multiplier = r.get("multiplier");
        // The model prices one unit of one instrument; a contract that stands for a hundred of
        // them would make every figure on the row a hundred times too small.
        let derivative = !multiplier.is_empty() && multiplier != "1" && multiplier.parse::<f64>() != Ok(1.0);

        let kind = if cancelled {
            CANCELLED_TRADE.to_string()
        } else if derivative {
            DERIVATIVE_TRADE.to_string()
        } else {
            r.get("buySell").to_uppercase()
        };
        let note = if cancelled || derivative {
            r.get("description").to_string()
        } else {
            String::new()
        };

        let mut row = self.blank();
        self.set(
            &mut row,
            DATE,
            date(r.any(&["tradeDate", "dateTime", "reportDate"])),
        );
        self.set(&mut row, KIND, kind);
        self.set(&mut row, SYMBOL, r.get("symbol").to_string());
        self.set(&mut row, ISIN, self.isin(r));
        self.set(&mut row, NAME, r.get("description").to_string());
        self.set(&mut row, QUANTITY, r.get("quantity").to_string());
        self.set(&mut row, PRICE, r.get("tradePrice").to_string());
        self.set(&mut row, AMOUNT, trade_amount(r));
        self.set(&mut row, FEE, r.get("ibCommission").to_string());
        self.set(&mut row, TAX, r.get("taxes").to_string());
        self.set(&mut row, CURRENCY, r.get("currency").to_string());
        self.set(&mut row, FX_RATE, r.get("fxRateToBase").to_string());
        self.set(&mut row, ACCOUNT, r.account());
        self.set(&mut row, NOTE, note);
        rows.push(row);
    }

    /// A forex trade's two legs are linked here, so `calc` does not read them as crossing the boundary.
    fn forex(&mut self, r: &Record, rows: &mut Vec<Vec<String>>, problems: &mut Vec<ImportProblem>) {
        let symbol = r.get("symbol");
        let quote = r.get("currency");
        let base = symbol.split('.').next().unwrap_or_default().to_uppercase();
        if base.is_empty() || quote.is_empty() {
            problems.push(
                ImportProblem::file(
                    ProblemCode::SuspiciousCurrency,
                    format!("the currency trade {symbol:?} names no pair and was left out"),
                )
                .warn(),
            );
            return;
        }

        self.forex_legs += 1;
        let link = format!(
            "ibflex:fx:{}:{}",
            r.any(&["tradeID", "ibExecID"]),
            self.forex_legs
        );
        let commission_currency = r.any(&["ibCommissionCurrency", "currency"]).to_uppercase();

        for (currency, amount) in [
            (base, r.get("quantity").to_string()),
            (quote.to_uppercase(), trade_amount(r)),
        ] {
            let mut row = self.blank();
            self.set(
                &mut row,
                DATE,
                date(r.any(&["tradeDate", "dateTime", "reportDate"])),
            );
            self.set(&mut row, KIND, CURRENCY_CONVERSION.to_string());
            self.set(&mut row, AMOUNT, amount);
            // The commission is charged in one currency, so it belongs to that leg alone.
            if currency == commission_currency {
                self.set(&mut row, FEE, r.get("ibCommission").to_string());
            }
            self.set(&mut row, CURRENCY, currency);
            self.set(&mut row, FX_RATE, String::new());
            self.set(&mut row, ACCOUNT, r.account());
            self.set(&mut row, LINK, link.clone());
            self.set(&mut row, NOTE, symbol.to_string());
            rows.push(row);
        }
    }

    fn cash(&self, r: &Record) -> Vec<String> {
        let mut row = self.blank();
        self.set(
            &mut row,
            DATE,
            date(r.any(&["dateTime", "settleDate", "reportDate"])),
        );
        self.set(&mut row, KIND, r.get("type").to_string());
        self.set(&mut row, SYMBOL, r.get("symbol").to_string());
        self.set(&mut row, ISIN, self.isin(r));
        self.set(&mut row, NAME, r.get("description").to_string());
        self.set(&mut row, AMOUNT, r.get("amount").to_string());
        self.set(&mut row, CURRENCY, r.get("currency").to_string());
        self.set(&mut row, FX_RATE, r.get("fxRateToBase").to_string());
        self.set(&mut row, ACCOUNT, r.account());
        self.set(&mut row, NOTE, r.get("description").to_string());
        row
    }

    fn corporate_action(&self, r: &Record) -> Vec<String> {
        let mut row = self.blank();
        self.set(
            &mut row,
            DATE,
            date(r.any(&["dateTime", "reportDate", "actionDate"])),
        );
        self.set(&mut row, KIND, CORPORATE_ACTION.to_string());
        self.set(&mut row, SYMBOL, r.get("symbol").to_string());
        self.set(&mut row, ISIN, self.isin(r));
        self.set(&mut row, NAME, r.get("description").to_string());
        self.set(&mut row, QUANTITY, r.get("quantity").to_string());
        self.set(&mut row, AMOUNT, r.any(&["proceeds", "value"]).to_string());
        self.set(&mut row, CURRENCY, r.get("currency").to_string());
        self.set(&mut row, FX_RATE, r.get("fxRateToBase").to_string());
        self.set(&mut row, ACCOUNT, r.account());
        self.set(&mut row, NOTE, r.get("description").to_string());
        row
    }

    fn isin(&self, r: &Record) -> String {
        let own = r.get("isin");
        if !own.is_empty() {
            return own.to_string();
        }
        self.isin_by_conid
            .get(r.get("conid"))
            .map(|i| i.to_string())
            .unwrap_or_default()
    }

    fn blank(&self) -> Vec<String> {
        vec![String::new(); COLUMNS.len()]
    }

    fn set(&self, row: &mut [String], column: &str, value: String) {
        if let Some(at) = COLUMNS.iter().position(|c| *c == column) {
            row[at] = value;
        }
    }
}

/// What the trade did to the cash, signed the way the rest of the file signs: negative for a
/// purchase. `proceeds` says it directly; `tradeMoney` says the opposite of it.
fn trade_amount(r: &Record) -> String {
    let proceeds = r.get("proceeds");
    if !proceeds.is_empty() {
        return proceeds.to_string();
    }
    let money = r.get("tradeMoney");
    if !money.is_empty() {
        return negate(money);
    }
    let quantity = r.get("quantity");
    match quantity.strip_prefix('-') {
        Some(positive) => positive.to_string(),
        None if quantity.is_empty() => String::new(),
        None => negate(quantity),
    }
}

fn negate(value: &str) -> String {
    match value.strip_prefix('-') {
        Some(positive) => positive.to_string(),
        None => format!("-{value}"),
    }
}

/// The day out of a Flex timestamp. The query's own date format is the user's choice in the web
/// office, so the shapes it can arrive in are read here and normalised to one.
fn date(value: &str) -> String {
    let day = value
        .split([';', ',', ' ', 'T'])
        .next()
        .unwrap_or_default()
        .trim();
    if day.len() == 8 && day.chars().all(|c| c.is_ascii_digit()) {
        return format!("{}-{}-{}", &day[0..4], &day[4..6], &day[6..8]);
    }
    match parse_date_any(day) {
        Some((parsed, _)) => parsed.to_string(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests;
