//! Exporting one table of the report as CSV. The section is one string, so a menu item in the
//! UI is the whole request and no export exists that the screen cannot show.

use super::csv::{maybe, money, row};
use super::summary::reports_summary;
use super::types::ReportsData;
use crate::error::{UiError, UiResult};
use crate::state::AppState;
use tauri::State;

/// Which table of the report a CSV export contains. One string, so a menu item in the
/// UI is the whole request and no export exists that the screen cannot show.
#[tauri::command]
pub fn report_export(state: State<AppState>, section: String, from: String, to: String) -> UiResult<String> {
    // An export is of what the screen shows, so it follows the picker like the screen does.
    let data = reports_summary(state, from, to, None)?;
    let c = &data.base_currency;

    Ok(match section.as_str() {
        "gains.year" => gains_year(&data, c),
        "gains.security" => gains_security(&data, c),
        "gains.detail" => gains_detail(&data, c),
        "dividends.year" => dividends_year(&data, c),
        "dividends.security" => dividends_security(&data, c),
        "dividends.detail" => dividends_detail(&data, c),
        "charges.year" => charges_year(&data, c),
        "charges.kind" => charges_kind(&data, c),
        "charges.account" => charges_account(&data, c),
        "charges.detail" => charges_detail(&data, c),
        other => return Err(UiError::invalid(format!("unknown report section {other:?}"))),
    })
}
fn gains_year(data: &ReportsData, c: &str) -> String {
    let mut csv = String::new();
    csv.push_str(&row([
        "year",
        "disposals",
        &format!("proceeds {c}"),
        &format!("cost basis {c}"),
        &format!("fees {c}"),
        &format!("taxes {c}"),
        &format!("result {c}"),
        "return",
    ]));
    for r in &data.gains_by_year {
        let s = &r.summary;
        csv.push_str(&row([
            r.year.to_string().as_str(),
            s.disposals.to_string().as_str(),
            money(s.proceeds_base).as_str(),
            money(s.cost_base).as_str(),
            money(s.fees_base).as_str(),
            money(s.taxes_base).as_str(),
            money(s.gain_base).as_str(),
            maybe(r.return_on_cost).as_str(),
        ]));
    }
    csv
}

fn gains_security(data: &ReportsData, c: &str) -> String {
    let mut csv = String::new();
    csv.push_str(&row([
        "ticker",
        "name",
        "disposals",
        &format!("proceeds {c}"),
        &format!("cost basis {c}"),
        &format!("fees {c}"),
        &format!("taxes {c}"),
        &format!("result {c}"),
        "return",
    ]));
    for r in &data.gains_by_security {
        let s = &r.summary;
        csv.push_str(&row([
            r.symbol.as_str(),
            r.name.as_str(),
            s.disposals.to_string().as_str(),
            money(s.proceeds_base).as_str(),
            money(s.cost_base).as_str(),
            money(s.fees_base).as_str(),
            money(s.taxes_base).as_str(),
            money(s.gain_base).as_str(),
            maybe(r.return_on_cost).as_str(),
        ]));
    }
    csv
}

fn gains_detail(data: &ReportsData, c: &str) -> String {
    let mut csv = String::new();
    csv.push_str(&row([
        "date",
        "transaction",
        "ticker",
        "name",
        "quantity",
        &format!("proceeds {c}"),
        &format!("cost basis {c}"),
        &format!("fees {c}"),
        &format!("taxes {c}"),
        &format!("result {c}"),
        "return",
    ]));
    for r in &data.disposals {
        let g = &r.gain;
        csv.push_str(&row([
            g.date.to_string().as_str(),
            g.kind.as_str(),
            r.symbol.as_str(),
            r.name.as_str(),
            money(g.quantity).as_str(),
            money(g.proceeds_base).as_str(),
            money(g.cost_base).as_str(),
            money(g.fees_base).as_str(),
            money(g.taxes_base).as_str(),
            money(g.gain_base).as_str(),
            maybe(r.return_on_cost).as_str(),
        ]));
    }
    csv
}

fn dividends_year(data: &ReportsData, c: &str) -> String {
    let mut csv = String::new();
    csv.push_str(&row([
        "year",
        "payments",
        &format!("accrued {c}"),
        &format!("tax {c}"),
        &format!("fees {c}"),
        &format!("received {c}"),
    ]));
    for r in &data.dividends_by_year {
        let s = &r.summary;
        csv.push_str(&row([
            r.year.to_string().as_str(),
            s.payments.to_string().as_str(),
            money(s.gross_base).as_str(),
            money(s.taxes_base).as_str(),
            money(s.fees_base).as_str(),
            money(s.net_base).as_str(),
        ]));
    }
    csv
}

fn dividends_security(data: &ReportsData, c: &str) -> String {
    let mut csv = String::new();
    csv.push_str(&row([
        "ticker",
        "name",
        "payments",
        &format!("accrued {c}"),
        &format!("tax {c}"),
        &format!("received {c}"),
        "yield on cost, all time",
    ]));
    for r in &data.dividends_by_security {
        let s = &r.summary;
        csv.push_str(&row([
            r.symbol.as_str(),
            r.name.as_str(),
            s.payments.to_string().as_str(),
            money(s.gross_base).as_str(),
            money(s.taxes_base).as_str(),
            money(s.net_base).as_str(),
            maybe(r.yield_on_cost).as_str(),
        ]));
    }
    csv
}

fn dividends_detail(data: &ReportsData, c: &str) -> String {
    let mut csv = String::new();
    csv.push_str(&row([
        "date",
        "account",
        "ticker",
        "name",
        "currency",
        "accrued in currency",
        &format!("accrued {c}"),
        &format!("tax {c}"),
        &format!("fees {c}"),
        &format!("received {c}"),
    ]));
    for r in &data.payments {
        let p = &r.record;
        csv.push_str(&row([
            p.date.to_string().as_str(),
            r.account.as_str(),
            r.symbol.as_str(),
            r.name.as_str(),
            p.currency.as_str(),
            money(p.gross_in_currency).as_str(),
            money(p.gross_base).as_str(),
            money(p.taxes_base).as_str(),
            money(p.fees_base).as_str(),
            money(p.net_base).as_str(),
        ]));
    }
    csv
}

fn charges_year(data: &ReportsData, c: &str) -> String {
    let mut csv = String::new();
    csv.push_str(&row([
        "year",
        "transactions",
        &format!("fees {c}"),
        &format!("taxes {c}"),
        &format!("total {c}"),
    ]));
    for r in &data.charges_by_year {
        let s = &r.summary;
        csv.push_str(&row([
            r.year.to_string().as_str(),
            s.count.to_string().as_str(),
            money(s.fees_base).as_str(),
            money(s.taxes_base).as_str(),
            money(r.total_base).as_str(),
        ]));
    }
    csv
}

fn charges_kind(data: &ReportsData, c: &str) -> String {
    let mut csv = String::new();
    csv.push_str(&row([
        "kind",
        "transactions",
        &format!("fees {c}"),
        &format!("taxes {c}"),
        &format!("total {c}"),
    ]));
    for r in &data.charges_by_kind {
        let s = &r.summary;
        csv.push_str(&row([
            r.kind.as_str(),
            s.count.to_string().as_str(),
            money(s.fees_base).as_str(),
            money(s.taxes_base).as_str(),
            money(r.total_base).as_str(),
        ]));
    }
    csv
}

fn charges_account(data: &ReportsData, c: &str) -> String {
    let mut csv = String::new();
    csv.push_str(&row([
        "account",
        "transactions",
        &format!("fees {c}"),
        &format!("taxes {c}"),
        &format!("total {c}"),
    ]));
    for r in &data.charges_by_account {
        let s = &r.summary;
        csv.push_str(&row([
            r.account.as_str(),
            s.count.to_string().as_str(),
            money(s.fees_base).as_str(),
            money(s.taxes_base).as_str(),
            money(r.total_base).as_str(),
        ]));
    }
    csv
}

fn charges_detail(data: &ReportsData, c: &str) -> String {
    let mut csv = String::new();
    csv.push_str(&row([
        "date",
        "kind",
        "account",
        "ticker",
        "name",
        "currency",
        "amount in currency",
        &format!("amount {c}"),
    ]));
    for r in &data.charge_rows {
        let ch = &r.record;
        csv.push_str(&row([
            ch.date.to_string().as_str(),
            ch.kind.as_str(),
            r.account.as_str(),
            r.symbol.as_str(),
            r.name.as_str(),
            ch.currency.as_str(),
            money(ch.amount_in_currency).as_str(),
            money(ch.amount_base).as_str(),
        ]));
    }
    csv
}

/// `footer` is the disclaimer the file leaves with, written by the frontend because it is a
/// sentence and the language is known only there (ADR-0023). It is one cell on a row of its own
/// after a blank line, so a spreadsheet shows it and a parser reading the header sees the tables
/// end first.
#[tauri::command]
pub fn report_save(
    state: State<AppState>,
    section: String,
    from: String,
    to: String,
    path: String,
    footer: Option<String>,
) -> UiResult<()> {
    let mut csv = report_export(state, section, from, to)?;
    if let Some(note) = footer.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        csv.push('\n');
        csv.push_str(&row([note]));
    }
    // Excel reads a semicolon-separated file as UTF-8 only when it opens with a BOM.
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(csv.as_bytes());
    std::fs::write(&path, bytes).map_err(|e| UiError::invalid(format!("cannot write {path}: {e}")))
}
