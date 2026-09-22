use crate::commands::parse_date;
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::state::AppState;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sq_core::calc::{
    MonthlyNet, YearlyNet, transaction_amount_base, transaction_net_base, transactions_net_by_month,
    transactions_net_by_year,
};
use sq_core::import::canonical_to_file;
use sq_core::prelude::*;
use std::str::FromStr;
use tauri::{AppHandle, State};

#[derive(Debug, Serialize)]
pub struct TransactionRow {
    #[serde(flatten)]
    pub transaction: Transaction,
    pub symbol: Option<String>,
    pub account_name: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub net_base: Decimal,
}

#[derive(Debug, Serialize)]
pub struct TransactionsData {
    pub base_currency: String,
    pub rows: Vec<TransactionRow>,
    pub monthly_net: Vec<MonthlyNet>,
    pub yearly_net: Vec<YearlyNet>,
    #[serde(with = "rust_decimal::serde::str")]
    pub total_net: Decimal,
}

#[derive(Debug, Default, Deserialize)]
pub struct TransactionFilter {
    pub account_id: Option<String>,
    pub security_id: Option<String>,
    pub kind: Option<TransactionKind>,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// Writes the filtered operations as the app's own transaction file. The export is of what the
/// screen shows, so it takes the same filter the list does — and it names accounts and
/// instruments rather than ids, so the file imports into another portfolio (ADR-0066).
#[tauri::command]
pub fn transactions_export(state: State<AppState>, filter: TransactionFilter) -> UiResult<String> {
    let store = state.store()?;
    let portfolio = state.scoped_portfolio(&store)?;
    let rows = filtered_transactions(&store, &portfolio.account_ids, &filter)?;
    Ok(canonical_to_file(
        &rows,
        &store.list_accounts()?,
        &store.list_securities()?,
    )?)
}

#[tauri::command]
pub fn transactions_export_save(
    state: State<AppState>,
    filter: TransactionFilter,
    path: String,
) -> UiResult<()> {
    let text = transactions_export(state, filter)?;
    std::fs::write(&path, text).map_err(|e| UiError::invalid(format!("cannot write {path}: {e}")))
}

/// The rows a filter leaves, in stored order. Shared by the list and its export so the file can
/// never hold a different set of operations than the screen it was taken from.
fn filtered_transactions(
    store: &Store,
    account_ids: &[String],
    filter: &TransactionFilter,
) -> UiResult<Vec<Transaction>> {
    let from = filter.from.as_deref().map(parse_date).transpose()?;
    let to = filter.to.as_deref().map(parse_date).transpose()?;
    Ok(store
        .transactions_for_accounts(account_ids, to)?
        .into_iter()
        .filter(|t| from.is_none_or(|d| t.date >= d))
        .filter(|t| filter.account_id.as_ref().is_none_or(|id| &t.account_id == id))
        .filter(|t| {
            filter
                .security_id
                .as_ref()
                .is_none_or(|id| t.security_id.as_ref() == Some(id))
        })
        .filter(|t| filter.kind.is_none_or(|k| t.kind == k))
        .collect())
}

#[tauri::command]
pub fn transactions_list(state: State<AppState>, filter: TransactionFilter) -> UiResult<TransactionsData> {
    let store = state.store()?;
    let portfolio = state.scoped_portfolio(&store)?;
    let base = portfolio.base_currency.clone();

    let accounts = store.list_accounts()?;
    let securities = store.list_securities()?;

    let filtered = filtered_transactions(&store, &portfolio.account_ids, &filter)?;

    let pairs: Vec<(String, String)> = filtered
        .iter()
        .map(|t| (t.currency.clone(), base.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let upto = filtered.iter().map(|t| t.date).max().unwrap_or_else(today);
    let rates = store.rate_cache(&pairs, upto)?;

    let monthly_net = transactions_net_by_month(&filtered, &base, &rates).map_err(UiError::from)?;
    let yearly_net = transactions_net_by_year(&monthly_net);
    let total_net = yearly_net.iter().map(|y| y.net_base).sum();

    let mut rows: Vec<TransactionRow> = Vec::with_capacity(filtered.len());
    for t in filtered {
        rows.push(TransactionRow {
            symbol: t
                .security_id
                .as_ref()
                .and_then(|id| securities.iter().find(|s| &s.id == id))
                .map(|s| s.symbol.clone()),
            account_name: accounts
                .iter()
                .find(|a| a.id == t.account_id)
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            amount_base: transaction_amount_base(&t, &base, &rates)?,
            net_base: transaction_net_base(&t, &base, &rates)?,
            transaction: t,
        });
    }

    rows.reverse();
    Ok(TransactionsData {
        base_currency: base,
        rows,
        monthly_net,
        yearly_net,
        total_net,
    })
}

fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}

#[derive(Debug, Deserialize)]
pub struct TransactionInput {
    pub id: Option<String>,
    pub account_id: String,
    pub security_id: Option<String>,
    pub kind: TransactionKind,
    pub date: String,
    pub quantity: Option<String>,
    pub price: Option<String>,
    pub amount: Option<String>,
    pub fees: Option<String>,
    pub taxes: Option<String>,
    pub currency: String,
    /// Only when the charge was billed somewhere other than the operation itself.
    pub fee_currency: Option<String>,
    pub tax_currency: Option<String>,
    pub fx_rate_to_base: Option<String>,
    pub note: Option<String>,
}

/// A charge currency is recorded only when it differs from the operation's own.
fn charge_currency(input: Option<String>, currency: &str) -> Option<String> {
    input
        .map(|c| sq_core::money::normalize_currency(&c))
        .filter(|c| !c.is_empty() && c != currency)
}

/// Parses one wire input into a transaction. Shared with the plan commit, which writes the same
/// rows the editor writes — a plan must not invent a second way of spelling a purchase.
pub(crate) fn from_input(input: TransactionInput) -> UiResult<Transaction> {
    let date = parse_date(&input.date)?;
    let currency = super::portfolio::require_currency(&input.currency)?;

    let quantity = decimal(input.quantity.as_deref(), "quantity")?.unwrap_or_default();
    let price = decimal(input.price.as_deref(), "price")?.unwrap_or_default();
    let amount = if input.kind.affects_quantity() {
        quantity * price
    } else {
        decimal(input.amount.as_deref(), "amount")?.unwrap_or_default()
    };

    Ok(Transaction {
        id: input.id.clone().unwrap_or_else(sq_core::model::new_id),
        account_id: input.account_id,
        security_id: input.security_id.filter(|s| !s.is_empty()),
        kind: input.kind,
        date,
        quantity,
        price,
        amount,
        fees: decimal(input.fees.as_deref(), "commission")?.unwrap_or_default(),
        taxes: decimal(input.taxes.as_deref(), "tax")?.unwrap_or_default(),
        fee_currency: charge_currency(input.fee_currency, &currency),
        tax_currency: charge_currency(input.tax_currency, &currency),
        currency,
        fx_rate_to_base: decimal(input.fx_rate_to_base.as_deref(), "fx rate")?,
        link_id: None,
        // The broker's own identifier belongs to an imported row and is never typed by hand.
        external_id: None,
        note: input.note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty()),
        // Nothing the user writes is a lens's rewrite of something else.
        scoped_from: None,
    })
}

#[tauri::command]
pub fn transaction_save(
    app: AppHandle,
    state: State<AppState>,
    input: TransactionInput,
) -> UiResult<Transaction> {
    let existing = input.id.clone();
    let transaction = from_input(input)?;

    let store = state.store()?;

    let link_id = match &existing {
        Some(id) => store
            .transactions_for_account(&transaction.account_id)?
            .into_iter()
            .find(|t| &t.id == id)
            .and_then(|t| t.link_id),
        None => None,
    };

    let transaction = Transaction {
        link_id,
        ..transaction
    };
    store.save_transaction(&transaction)?;
    drop(store);

    // A transaction may bring the first trade of an instrument or a currency nothing quoted yet.
    crate::jobs::fetch_missing(&app, &state);
    emit_changed(&app, "transactions")?;
    Ok(transaction)
}

#[tauri::command]
pub fn transaction_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_transaction(&id)?;
    emit_changed(&app, "transactions")
}

pub(crate) fn decimal(value: Option<&str>, what: &str) -> UiResult<Option<Decimal>> {
    let Some(raw) = value.map(str::trim).filter(|v| !v.is_empty()) else {
        return Ok(None);
    };
    let normalized = raw.replace(',', ".");
    Decimal::from_str(&normalized)
        .map(Some)
        .map_err(|e| UiError::invalid(format!("invalid value in field \"{what}\": {raw:?} ({e})")))
}
