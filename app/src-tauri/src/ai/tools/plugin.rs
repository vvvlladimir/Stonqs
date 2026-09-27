//! A plugin's assistant tool (ADR-0085). Not a `Tool` in `CATALOGUE`: that table is compiled in,
//! and these arrive with a package. What is the same is everything the model and the user see —
//! a strict schema with `reason` added, a card before it runs, an answer fenced as data.
//!
//! The body is a stranger's WASM component, handed the reads its manifest declared in the plugin
//! bridge's own field names (`lib/pluginBridge.ts`), so a tool and a widget of one package read
//! one shape. `the_projection_is_the_bridges` pins the two copies together.

use super::args::{period_property, period_summary, resolve_period};
use super::{AiResult, Params, ToolContext, add_required, tool, with_reason};
use crate::plugins::{LoadedTool, Read};
use serde_json::{Value, json};
use sq_core::market::DateRange;

/// What the model is offered. The description says whose tool it is, because its figures are
/// the plugin's own (ADR-0082) and the model must not present them as the app's.
pub fn definition(t: &LoadedTool) -> (String, String, Value) {
    let mut schema = t.schema.clone();
    if t.periodic {
        add_required(&mut schema, "period", period_property());
    }
    let description = format!(
        "From the {} plugin, not the app: its figures are the plugin's own, say so when you use \
         them. {}",
        t.info.plugin_name, t.description
    );
    (t.model_name.clone(), description, with_reason(schema))
}

/// The values behind the consent card: whose code runs, and over which window.
pub fn summary(t: &LoadedTool, context: &ToolContext, args: &Value) -> Params {
    let mut params = if t.periodic {
        period_summary(context, args)
    } else {
        Params::new()
    };
    params.insert("plugin".into(), t.info.plugin_name.clone());
    params.insert("tool".into(), t.info.name.clone());
    params
}

pub fn run(t: &LoadedTool, context: &ToolContext, args: &Value) -> AiResult<Value> {
    let range = if t.periodic {
        Some(resolve_period(context, args)?)
    } else {
        None
    };
    // The module is handed what the model asked for, minus what the app added for itself.
    let mut own = args.clone();
    if let Some(map) = own.as_object_mut() {
        map.remove(super::REASON);
        map.remove("period");
    }
    let data = data(context, &t.info.reads, range)?;
    let answer = crate::plugins::tool::call(&t.module, &own, &data).map_err(tool)?;
    Ok(json!({ "plugin": t.info.plugin_name, "tool": t.info.name, "answer": answer }))
}

/// The declared reads and nothing else, in the bridge's field names. Amounts are the full
/// decimals, as a widget gets them: rounding is the plugin's to do for its own answer.
pub(crate) fn data(context: &ToolContext, reads: &[Read], range: Option<DateRange>) -> AiResult<Value> {
    let mut out = serde_json::Map::new();
    if let Some(range) = range {
        out.insert(
            "period".into(),
            json!({ "from": range.from.to_string(), "to": range.to.to_string() }),
        );
    }
    for read in reads {
        let (key, value) = match (read, range) {
            (Read::Valuation, _) => ("valuation", valuation(context)?),
            (Read::Positions, _) => ("positions", positions(context)?),
            (Read::Performance, Some(range)) => ("performance", performance(context, range)?),
            (Read::Transactions, Some(range)) => ("transactions", transactions(context, range)?),
            // A period read without a period cannot be installed (`ToolDef::periodic`).
            (Read::Performance | Read::Transactions, None) => continue,
        };
        out.insert(key.into(), value);
    }
    Ok(Value::Object(out))
}

fn valuation(context: &ToolContext) -> AiResult<Value> {
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let v = analytics.valuation_at(context.today).map_err(tool)?;
    Ok(json!({
        "date": context.today.to_string(),
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

fn positions(context: &ToolContext) -> AiResult<Value> {
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let base = analytics.base_currency();
    let holdings = analytics.holdings_at(context.today).map_err(tool)?;
    let valuation =
        sq_core::calc::value_holdings(&holdings, base, context.today, context.store, context.store)
            .map_err(tool)?;
    let changes = sq_core::calc::day_changes(&holdings, base, context.today, context.store, context.store)
        .map_err(tool)?;
    let securities = context.store.list_securities().map_err(tool)?;
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
                // The same share `positions_at` states; an empty scope has no weights at all.
                "weight": if total.is_zero() {
                    "0".to_string()
                } else {
                    (p.market_value_base / total).to_string()
                },
                "day_change": changes.positions.get(&p.security_id).map(|c| c.change.to_string()),
            })
        })
        .collect();

    Ok(json!({
        "date": context.today.to_string(),
        "base_currency": base,
        "total_value": total.to_string(),
        "rows": rows,
    }))
}

fn performance(context: &ToolContext, range: DateRange) -> AiResult<Value> {
    let analytics = context.scope.analytics(context.store).map_err(tool)?;
    let series = analytics.series(range).map_err(tool)?;
    let summary = sq_core::calc::period_summary(&series);
    let twr = analytics.twr(range.from, range.to).map_err(tool)?;
    Ok(json!({
        "from": range.from.to_string(),
        "to": range.to.to_string(),
        "base_currency": analytics.base_currency(),
        "twr": twr.to_string(),
        "twr_annualized": sq_core::calc::annualize(twr, range.from, range.to).map(|r| r.to_string()),
        // No sign change in the flows is no rate at all, not a failed read.
        "xirr": analytics.xirr(range.to).ok().map(|r| r.to_string()),
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

/// The scope's own rows, as the Transactions screen lists them — not the rewritten legs the
/// return figures run on, because a plugin reading operations is reading the ledger.
fn transactions(context: &ToolContext, range: DateRange) -> AiResult<Value> {
    let base = context.scope.portfolio.base_currency.clone();
    let mut rows = context
        .store
        .transactions_for_accounts(&context.scope.accounts, Some(range.to))
        .map_err(tool)?;
    rows.retain(|t| t.date >= range.from);
    let pairs: Vec<(String, String)> = rows
        .iter()
        .map(|t| (t.currency.clone(), base.clone()))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    let rates = context.store.rate_cache(&pairs, range.to).map_err(tool)?;
    let accounts = context.store.list_accounts().map_err(tool)?;
    let securities = context.store.list_securities().map_err(tool)?;

    let mut out = Vec::with_capacity(rows.len());
    for t in rows.iter().rev() {
        out.push(json!({
            "id": t.id,
            "date": t.date.to_string(),
            "kind": t.kind,
            "account": accounts.iter().find(|a| a.id == t.account_id).map(|a| a.name.as_str()).unwrap_or_default(),
            "symbol": t.security_id.as_ref().and_then(|id| securities.iter().find(|s| &s.id == id)).map(|s| s.symbol.as_str()),
            "amount": t.amount.to_string(),
            "currency": t.currency,
            "amount_base": sq_core::calc::transaction_amount_base(t, &base, &rates).map_err(tool)?.to_string(),
            "net_base": sq_core::calc::transaction_net_base(t, &base, &rates).map_err(tool)?.to_string(),
            "note": t.note,
        }));
    }
    Ok(json!({ "base_currency": base, "rows": out }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ScopeSelection;
    use sq_core::storage::Store;

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

    /// A widget and a tool of one package read one shape: a field added to the bridge and not
    /// here — or here and not there — fails this rather than a stranger's plugin.
    #[test]
    fn the_projection_is_the_bridges() {
        let today = chrono::Local::now().date_naive();
        let store = Store::open_in_memory().unwrap();
        let mut portfolio = sq_core::model::Portfolio::new("Demo", "EUR");
        store.save_portfolio(&portfolio).unwrap();
        crate::demo::seed(&store, &mut portfolio, today).unwrap();
        let accounts = store.list_accounts().unwrap().into_iter().map(|a| a.id).collect();
        let scope = ScopeSelection { portfolio, accounts };
        let context = ToolContext {
            store: &store,
            scope: &scope,
            today,
            quotes_source: None,
            changed: &|_| {},
        };
        let range = DateRange::new(today - chrono::Days::new(90), today);
        let all = [
            Read::Valuation,
            Read::Positions,
            Read::Performance,
            Read::Transactions,
        ];
        let data = data(&context, &all, Some(range)).unwrap();

        assert_eq!(keys(&data["valuation"]), bridge_fields("BridgeValuation"));
        assert_eq!(keys(&data["positions"]), bridge_fields("BridgePositions"));
        assert_eq!(
            keys(&data["positions"]["rows"][0]),
            bridge_fields("BridgePosition")
        );
        let mut performance = bridge_fields("BridgePerformance");
        performance.sort();
        assert_eq!(keys(&data["performance"]), performance);
        assert_eq!(keys(&data["performance"]["series"][0]), ["date", "flow", "value"]);
        assert_eq!(keys(&data["transactions"]), bridge_fields("BridgeTransactions"));
        assert_eq!(
            keys(&data["transactions"]["rows"][0]),
            bridge_fields("BridgeTransaction")
        );
    }

    /// A package installed before the install check demanded `required` still reaches the model
    /// with every property required — one left out and a strict provider refuses every chat.
    #[test]
    fn a_schema_without_required_still_requires_what_the_app_adds() {
        let loaded = LoadedTool {
            info: crate::plugins::ToolInfo {
                key: "p/t".into(),
                name: "T".into(),
                plugin: "p".into(),
                plugin_name: "P".into(),
                reads: vec![Read::Performance],
            },
            model_name: "plugin_p_t".into(),
            description: String::new(),
            schema: json!({ "type": "object", "additionalProperties": false, "properties": {} }),
            periodic: true,
            module: std::path::PathBuf::new(),
        };
        let (_, _, schema) = definition(&loaded);
        let mut required: Vec<&str> = schema["required"]
            .as_array()
            .expect("required is there")
            .iter()
            .filter_map(Value::as_str)
            .collect();
        required.sort();
        let mut properties: Vec<&str> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        properties.sort();
        assert_eq!(required, properties);
        assert_eq!(required, ["period", "reason"]);
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
        let context = ToolContext {
            store: &store,
            scope: &scope,
            today: chrono::Local::now().date_naive(),
            quotes_source: None,
            changed: &|_| {},
        };
        let data = data(&context, &[Read::Valuation], None).unwrap();
        assert_eq!(keys(&data), ["valuation"]);
    }
}
