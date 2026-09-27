//! The one projection of a plugin's declared reads (ADR-0083/0085/0088), in the plugin API's own
//! field names; `the_projection_is_the_bridges` pins it to `lib/pluginBridge.ts`.

use super::Read;
use crate::error::UiResult;
use crate::state::ScopeSelection;
use chrono::NaiveDate;
use serde_json::{Value, json};
use sq_core::market::DateRange;
use sq_core::storage::Store;

/// The declared reads and nothing else, for `scope` on `date` and — for a read over a period —
/// over `range`. Amounts are the full decimals, as strings: rounding is the plugin's to do.
pub fn project(
    store: &Store,
    scope: &ScopeSelection,
    reads: &[Read],
    date: NaiveDate,
    range: Option<DateRange>,
) -> UiResult<Value> {
    let mut out = serde_json::Map::new();
    if let Some(range) = range {
        out.insert(
            "period".into(),
            json!({ "from": range.from.to_string(), "to": range.to.to_string() }),
        );
    }
    for read in reads {
        let (key, value) = match (read, range) {
            (Read::Valuation, _) => ("valuation", valuation(store, scope, date)?),
            (Read::Positions, _) => ("positions", positions(store, scope, date)?),
            (Read::Performance, Some(range)) => ("performance", performance(store, scope, range)?),
            (Read::Transactions, Some(range)) => ("transactions", transactions(store, scope, range)?),
            // A period read without a period cannot be installed; asked anyway, it is left out.
            (Read::Performance | Read::Transactions, None) => continue,
        };
        out.insert(key.into(), value);
    }
    Ok(Value::Object(out))
}

fn valuation(store: &Store, scope: &ScopeSelection, date: NaiveDate) -> UiResult<Value> {
    let analytics = scope.analytics(store)?;
    let v = analytics.valuation_at(date)?;
    Ok(json!({
        "date": date.to_string(),
        "base_currency": analytics.base_currency(),
        "total_value": v.total_value_base.to_string(),
        "securities_value": v.securities_value_base.to_string(),
        "cash": v.cash_base.to_string(),
        "cost_basis": v.cost_basis_base.to_string(),
        "unrealized_result": v.unrealized_pnl_base.to_string(),
        "realized_result": v.realized_pnl_base.to_string(),
        "dividends": v.dividends_base.to_string(),
        "interest": v.interest_base.to_string(),
        "fees": v.fees_base.to_string(),
        "taxes": v.taxes_base.to_string(),
    }))
}

fn positions(store: &Store, scope: &ScopeSelection, date: NaiveDate) -> UiResult<Value> {
    let analytics = scope.analytics(store)?;
    let base = analytics.base_currency();
    let holdings = analytics.holdings_at(date)?;
    let valuation = sq_core::calc::value_holdings(&holdings, base, date, store, store)?;
    let changes = sq_core::calc::day_changes(&holdings, base, date, store, store)?;
    let securities = store.list_securities()?;
    let total = valuation.total_value_base;

    let rows: Vec<Value> = valuation
        .positions
        .iter()
        .map(|p| {
            let security = securities.iter().find(|s| s.id == p.security_id);
            json!({
                "symbol": security.map(|s| s.symbol.as_str()).unwrap_or_default(),
                "name": security.map(|s| s.name.as_str()).unwrap_or_default(),
                "currency": p.currency,
                "quantity": p.quantity.to_string(),
                "price": p.price.to_string(),
                "value": p.market_value_base.to_string(),
                "cost_basis": p.cost_basis_base.to_string(),
                "unrealized_result": p.unrealized_pnl_base.to_string(),
                // The same share `positions_at` states.
                "weight": sq_core::calc::weight(p.market_value_base, total).to_string(),
                "day_change": changes.positions.get(&p.security_id).map(|c| c.change.to_string()),
            })
        })
        .collect();

    Ok(json!({
        "date": date.to_string(),
        "base_currency": base,
        "total_value": total.to_string(),
        "rows": rows,
    }))
}

fn performance(store: &Store, scope: &ScopeSelection, range: DateRange) -> UiResult<Value> {
    let analytics = scope.analytics(store)?;
    let series = analytics.series(range)?;
    let summary = sq_core::calc::period_summary(&series);
    let twr = analytics.twr(range.from, range.to)?;
    // As `performance_summary` reads it: flows with no sign change have no rate at all, while a
    // missing price is still an error rather than a quiet null.
    let xirr = match analytics.xirr(range.to) {
        Ok(rate) => Some(rate.to_string()),
        Err(sq_core::Error::Math(_)) => None,
        Err(e) => return Err(e.into()),
    };
    Ok(json!({
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "base_currency": analytics.base_currency(),
        "twr": twr.to_string(),
        "twr_annualized": sq_core::calc::annualize(twr, range.from, range.to).map(|r| r.to_string()),
        "xirr": xirr,
        "start_value": summary.start_value_base.to_string(),
        "end_value": summary.end_value_base.to_string(),
        "net_flow": summary.net_flow_base.to_string(),
        "earned": summary.delta_base.to_string(),
        "series": series
            .dates
            .iter()
            .zip(&series.total_value_base)
            .zip(&series.external_flow_base)
            .map(|((date, value), flow)| json!({
                "date": date.to_string(),
                "value": value.to_string(),
                "flow": flow.to_string(),
            }))
            .collect::<Vec<_>>(),
    }))
}

/// The scope's own rows, as the Transactions screen lists them (`calc::journal_rows`) — not the
/// rewritten legs the return figures run on, because a plugin reading operations reads the ledger.
fn transactions(store: &Store, scope: &ScopeSelection, range: DateRange) -> UiResult<Value> {
    let base = scope.portfolio.base_currency.clone();
    let mut rows = store.transactions_for_accounts(&scope.accounts, Some(range.to))?;
    rows.retain(|t| t.date >= range.from);
    let pairs: Vec<(String, String)> = rows
        .iter()
        .map(|t| (t.currency.clone(), base.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let rates = store.rate_cache(&pairs, range.to)?;
    let listed = sq_core::calc::journal_rows(
        rows,
        &store.list_accounts()?,
        &store.list_securities()?,
        &base,
        &rates,
    )?;
    let out: Vec<Value> = listed
        .into_iter()
        .map(|r| {
            json!({
                "id": r.transaction.id,
                "date": r.transaction.date.to_string(),
                "kind": r.transaction.kind,
                "account": r.account_name,
                "symbol": r.symbol,
                "amount": r.transaction.amount.to_string(),
                "currency": r.transaction.currency,
                "amount_base": r.amount_base.to_string(),
                "net_base": r.net_base.to_string(),
                "note": r.transaction.note,
            })
        })
        .collect();
    Ok(json!({ "base_currency": base, "rows": out }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The field names of one interface in `lib/pluginBridge.ts`, read back from the source the
    /// way `guide::SCREEN_IDS` is read back from `nav.tsx`.
    fn bridge_fields(name: &str) -> Vec<String> {
        let source = std::fs::read_to_string("../src/lib/pluginBridge.ts").expect("pluginBridge.ts");
        let (_, after) = source
            .split_once(&format!("export interface {name} {{"))
            .unwrap_or_else(|| panic!("{name} in pluginBridge.ts"));
        let (body, _) = after.split_once("\n}").expect("the interface ends");
        let mut fields: Vec<String> = body
            .lines()
            .map(str::trim)
            .filter(|l| !l.starts_with("/*") && !l.starts_with('*') && !l.starts_with("//"))
            .filter_map(|l| {
                l.split_once(':')
                    .map(|(k, _)| k.trim_end_matches('?').to_string())
            })
            .filter(|k| !k.is_empty() && !k.contains(' '))
            .collect();
        fields.sort();
        fields
    }

    fn keys(value: &Value) -> Vec<String> {
        let mut keys: Vec<String> = value.as_object().expect("an object").keys().cloned().collect();
        keys.sort();
        keys
    }

    /// What the host builds is what the frontend's types say a plugin is handed: a field added
    /// here and not there — or there and not here — fails this rather than a stranger's plugin.
    #[test]
    fn the_projection_is_the_bridges() {
        let today = chrono::Local::now().date_naive();
        let store = Store::open_in_memory().unwrap();
        let mut portfolio = sq_core::model::Portfolio::new("Demo", "EUR");
        store.save_portfolio(&portfolio).unwrap();
        crate::demo::seed(&store, &mut portfolio, today).unwrap();
        let accounts = store.list_accounts().unwrap().into_iter().map(|a| a.id).collect();
        let scope = ScopeSelection { portfolio, accounts };
        let range = DateRange::new(today - chrono::Days::new(90), today);
        let all = [
            Read::Valuation,
            Read::Positions,
            Read::Performance,
            Read::Transactions,
        ];
        let data = project(&store, &scope, &all, today, Some(range)).unwrap();

        assert_eq!(keys(&data["valuation"]), bridge_fields("BridgeValuation"));
        assert_eq!(keys(&data["positions"]), bridge_fields("BridgePositions"));
        assert_eq!(
            keys(&data["positions"]["rows"][0]),
            bridge_fields("BridgePosition")
        );
        assert_eq!(keys(&data["performance"]), bridge_fields("BridgePerformance"));
        assert_eq!(keys(&data["performance"]["series"][0]), ["date", "flow", "value"]);
        assert_eq!(keys(&data["transactions"]), bridge_fields("BridgeTransactions"));
        assert_eq!(
            keys(&data["transactions"]["rows"][0]),
            bridge_fields("BridgeTransaction")
        );
        assert_eq!(keys(&data["period"]), ["from", "to"]);
    }

    #[test]
    fn only_the_declared_reads_are_handed_over() {
        let store = Store::open_in_memory().unwrap();
        let portfolio = sq_core::model::Portfolio::new("Test", "EUR");
        store.save_portfolio(&portfolio).unwrap();
        let scope = ScopeSelection {
            portfolio,
            accounts: Vec::new(),
        };
        let today = chrono::Local::now().date_naive();
        let data = project(&store, &scope, &[Read::Valuation], today, None).unwrap();
        assert_eq!(keys(&data), ["valuation"]);
    }
}
