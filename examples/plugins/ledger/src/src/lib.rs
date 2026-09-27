//! A writer for the plain-text accounting journal that `ledger` and `hledger` read.
//!
//! It exists as an example of the contract (ADR-0080), and it is a real writer: a journal is one
//! balanced entry per operation, several lines each, which no column layout can express. It is
//! handed the app's own transaction file and returns the journal's bytes — nothing here can read
//! a file, open a socket or ask the time.

use serde::Deserialize;
use std::fmt::Write;

wit_bindgen::generate!({
    path: "wit",
    world: "writer",
});

/// The part of a `stonqs.transactions` row a journal needs. Every value is a string in that
/// format, and a decimal string is exactly what a journal amount is, so none is parsed.
#[derive(Deserialize)]
struct Row {
    date: String,
    kind: String,
    account: Option<String>,
    symbol: Option<String>,
    quantity: Option<String>,
    price: Option<String>,
    amount: Option<String>,
    fee: Option<String>,
    fee_currency: Option<String>,
    tax: Option<String>,
    tax_currency: Option<String>,
    currency: String,
    note: Option<String>,
}

#[derive(Deserialize)]
struct File {
    format: String,
    version: u32,
    rows: Vec<Row>,
}

struct Ledger;

export!(Ledger);

impl Guest for Ledger {
    fn write(canonical: String) -> Result<Vec<u8>, String> {
        let file: File = serde_json::from_str(&canonical).map_err(|e| e.to_string())?;
        if file.format != "stonqs.transactions" || file.version != 1 {
            return Err(format!("{} version {} is not a file this writer knows", file.format, file.version));
        }
        let mut out = String::from("; Written from a stonqs.transactions file.\n");
        for row in &file.rows {
            out.push('\n');
            entry(&mut out, row);
        }
        Ok(out.into_bytes())
    }
}

/// One operation as one balanced entry. The cash leg is left without an amount, which a journal
/// reads as "whatever balances" — so a charge in another currency still balances, per currency.
fn entry(out: &mut String, row: &Row) {
    let account = account_name(row.account.as_deref().unwrap_or("Unknown"));
    let cash = format!("Assets:{account}:Cash");
    let symbol = row.symbol.as_deref().unwrap_or("");
    let amount = row.amount.as_deref().unwrap_or("0");
    let money = |value: &str| format!("{value} {}", row.currency);

    let (payee, postings): (String, Vec<(String, String)>) = match row.kind.as_str() {
        "BUY" | "SELL" => {
            let quantity = row.quantity.as_deref().unwrap_or("0");
            let sign = if row.kind == "SELL" { "-" } else { "" };
            let price = row.price.as_deref().unwrap_or("0");
            let verb = if row.kind == "SELL" { "Sell" } else { "Buy" };
            (
                format!("{verb} {symbol}"),
                vec![(
                    format!("Assets:{account}:{}", account_name(symbol)),
                    format!("{sign}{quantity} {} @ {}", commodity(symbol), money(price)),
                )],
            )
        }
        "DIVIDEND" => (
            format!("Dividend {symbol}"),
            vec![(format!("Income:Dividends:{}", account_name(symbol)), money(&negated(amount)))],
        ),
        "INTEREST" => ("Interest".into(), vec![("Income:Interest".into(), money(&negated(amount)))]),
        "DEPOSIT" => ("Deposit".into(), vec![("Equity:Transfers".into(), money(&negated(amount)))]),
        "WITHDRAWAL" => ("Withdrawal".into(), vec![("Equity:Transfers".into(), money(amount))]),
        "FEE" => ("Fee".into(), vec![("Expenses:Fees".into(), money(amount))]),
        "TAX" => ("Tax".into(), vec![("Expenses:Taxes".into(), money(amount))]),
        other => {
            // A split, a delivery, a transfer between one's own accounts: nothing a journal of
            // cash and cost says the same way, so it is named rather than guessed at.
            let _ = writeln!(out, "; {} {other} {symbol} not written: no journal equivalent", row.date);
            return;
        }
    };

    let _ = writeln!(out, "{} * {}", row.date, payee.trim_end());
    if let Some(note) = row.note.as_deref().filter(|n| !n.trim().is_empty()) {
        let _ = writeln!(out, "    ; {}", note.trim());
    }
    for (account, amount) in &postings {
        posting(out, account, amount);
    }
    let charge = |value: &Option<String>, currency: &Option<String>| {
        value
            .as_deref()
            .map(|v| format!("{v} {}", currency.as_deref().unwrap_or(&row.currency)))
    };
    if let Some(fee) = charge(&row.fee, &row.fee_currency) {
        posting(out, "Expenses:Fees", &fee);
    }
    if let Some(tax) = charge(&row.tax, &row.tax_currency) {
        posting(out, "Expenses:Taxes", &tax);
    }
    let _ = writeln!(out, "    {cash}");
}

fn posting(out: &mut String, account: &str, amount: &str) {
    let _ = writeln!(out, "    {account:<36}  {amount}");
}

fn negated(value: &str) -> String {
    match value.strip_prefix('-') {
        Some(positive) => positive.to_string(),
        None => format!("-{value}"),
    }
}

/// A journal ends an account name at two spaces and splits it at colons, so neither survives.
fn account_name(name: &str) -> String {
    name.replace(':', "-").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A commodity of letters alone is written bare; anything else (`IWDA.L`, `BRK-B`) is quoted.
fn commodity(symbol: &str) -> String {
    if !symbol.is_empty() && symbol.chars().all(|c| c.is_ascii_alphabetic()) {
        symbol.to_string()
    } else {
        format!("\"{symbol}\"")
    }
}
