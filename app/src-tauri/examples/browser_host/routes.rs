//! The commands the browser host answers, each by calling the real command function — the same
//! code the desktop app runs, only reached over HTTP instead of Tauri's IPC.
//!
//! A command taking `AppHandle` is compiled for the desktop runtime and cannot be called here, so
//! most writes are `DESKTOP_ONLY`; so is anything that reads a path, the keychain or the network.
//! `tests/browser_host.rs` checks that every registered command is in exactly one of the two lists.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sq_app_lib::commands::{
    accounts, ai, alerts, allocation, attributes, corporate_actions, dashboard, demo, goals, import,
    inflation, payments, performance, periods, plans, plugins, portfolio, positions, profiles, reports,
    securities, sources, trades, transactions, watchlists,
};
use sq_app_lib::error::UiResult;
use sq_app_lib::state::AppState;
use sq_app_lib::{import_templates, jobs, scope, settings};
use tauri::State;

/// What the host answered: the command's value, or its `UiError` as the desktop app serializes it.
pub type Reply = Result<Value, Value>;

/// Tauri renames a command's snake_case parameters to camelCase on the wire; so does the frontend.
fn camel(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut upper = false;
    for c in name.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// An absent argument is `null`, which an `Option` parameter reads as `None` — as Tauri does.
fn arg<T: DeserializeOwned>(args: &Value, name: &str) -> Result<T, Value> {
    let value = args.get(camel(name)).cloned().unwrap_or(Value::Null);
    serde_json::from_value(value)
        .map_err(|e| serde_json::json!({ "code": "invalid", "message": format!("argument {name}: {e}") }))
}

fn reply<T: Serialize>(result: UiResult<T>) -> Reply {
    match result {
        Ok(value) => serde_json::to_value(value).map_err(|e| Value::String(e.to_string())),
        Err(error) => Err(serde_json::to_value(error).unwrap_or(Value::Null)),
    }
}

macro_rules! routes {
    (
        reads { $($read:path { $($read_arg:ident),* };)* }
        others { $($other:path { $($other_arg:ident),* };)* }
    ) => {
        /// Every command routed here, by its wire name.
        #[allow(dead_code, reason = "read by tests/browser_host.rs, which shares this file")]
        pub const ROUTED: &[&str] = &[$(last_segment(stringify!($read)),)* $(last_segment(stringify!($other))),*];

        /// Commands whose answer depends on nothing but the stored data and their arguments, so the
        /// host may answer a repeat from memory until any other command runs.
        pub const READS: &[&str] = &[$(last_segment(stringify!($read))),*];

        /// `None` when the command is not routed at all.
        pub fn answer(state: &State<'_, AppState>, command: &str, args: &Value) -> Option<Reply> {
            $(
                if command == last_segment(stringify!($read)) {
                    let call = || -> Reply { reply($read(state.clone() $(, arg(args, stringify!($read_arg))?)*)) };
                    return Some(call());
                }
            )*
            $(
                if command == last_segment(stringify!($other)) {
                    let call = || -> Reply { reply($other(state.clone() $(, arg(args, stringify!($other_arg))?)*)) };
                    return Some(call());
                }
            )*
            None
        }
    };
}

const fn last_segment(path: &str) -> &str {
    let bytes = path.as_bytes();
    let mut start = bytes.len();
    while start > 0 && bytes[start - 1] != b':' && bytes[start - 1] != b' ' {
        start -= 1;
    }
    let (_, tail) = bytes.split_at(start);
    match std::str::from_utf8(tail) {
        Ok(name) => name,
        Err(_) => panic!("a command path is ASCII"),
    }
}

/// `jobs::quote_providers` is the one command returning a bare value rather than a `UiResult`.
fn quote_providers(state: State<'_, AppState>) -> UiResult<Vec<String>> {
    Ok(jobs::quote_providers(state))
}

routes! {
    reads {
        dashboard::app_status {};
        dashboard::dashboard_summary { date, source };
        portfolio::portfolio_get {};
        portfolio::portfolio_gaps {};
        plugins::plugins_list {};
        plugins::plugin_theme_css { plugin, theme };
        plugins::plugin_taxonomy_csv { plugin, set };
        plugins::plugin_reads { reads, date, from, to, source };
        plugins::plugin_state_get { plugin };
        profiles::profiles_list {};
        accounts::accounts_list {};
        accounts::accounts_total {};
        accounts::account_groups_list {};
        scope::scope_get {};
        securities::securities_list {};
        attributes::attribute_defs_list {};
        attributes::attributes_import_preview { content, config };
        attributes::attributes_export_csv {};
        corporate_actions::corporate_actions_list { security_id };
        alerts::alerts_list { security_id };
        alerts::alert_crossings_list { limit };
        alerts::alerts_unseen {};
        alerts::security_events_list { security_id };
        securities::quotes_range { security_id, from, to };
        positions::positions_at { date, source };
        positions::positions_cost_basis { date, source };
        positions::position_return { security_id, from, to, source };
        positions::position_returns { from, to, source };
        transactions::transactions_list { filter };
        transactions::transactions_export { filter };
        transactions::transfer_suggestions {};
        periods::period_ranges { as_of };
        periods::periods_get {};
        performance::performance_summary { from, to, source };
        performance::performance_breakdown { from, to, period, source };
        trades::trades_summary { from, to, by, source };
        payments::payments_grid { from, to, period, source };
        payments::dividends_expected { months, source };
        performance::risk_report { from, to, risk_free_rate, window_days, source };
        performance::benchmark_compare { security_id, from, to, source };
        performance::benchmark_series { security_id, from, to, source };
        inflation::inflation_status {};
        inflation::real_performance { from, to, source };
        inflation::inflation_series { from, to, source };
        allocation::allocation { cut, taxonomy_id, date, source };
        allocation::allocation_members { taxonomy_id, node_id, date, source };
        allocation::allocation_tree { taxonomy_id, date, source };
        allocation::taxonomies_list {};
        allocation::taxonomy_import_preview { content, config, name };
        allocation::taxonomy_group_preview { attribute_id, into };
        allocation::taxonomy_export_csv { taxonomy_id };
        allocation::targets_list {};
        allocation::rebalance_plan { target_id, date, cash_to_invest, allow_sell, source };
        reports::reports_summary { from, to, source };
        reports::income_summary { from, to, kind, source };
        reports::income_taxonomy { taxonomy_id, from, to, kind, source };
        reports::report_export { section, from, to };
        import_templates::import_templates_list {};
        quote_providers {};
        jobs::data_coverage {};
        plans::plans_list {};
        plans::plan_due { plan_id, as_of };
        plans::plan_projection { months };
        plans::fire_projection { annual_spending, withdrawal_rate, expected_return, contribution };
        goals::goals_list { date };
        goals::limits_list { date };
        watchlists::watchlists_list {};
        watchlists::watchlist_rows { id, from, to };
        settings::settings_get {};
        sources::market_sources_list {};
        sources::market_custom_list {};
        ai::ai_key_status { provider };
        ai::ai_chats_list {};
        ai::ai_chat_export { id };
        ai::ai_messages_list { chat_id };
        ai::ai_providers_list {};
        ai::ai_grants_list { chat_id };
        ai::ai_usage_totals {};
    }
    // Writes, and reads of state a write in this list changes (the import file, the refresh).
    others {
        plugins::plugin_state_save { plugin, document };
        alerts::alerts_mark_seen {};
        periods::period_save { period };
        periods::period_delete { id };
        periods::periods_restore {};
        import::import_file_info {};
        import::import_clear {};
        import::import_prices_load { name, content };
        import::import_prices_preview { config, mapping };
        import_templates::import_template_save { name, config, mapping };
        import_templates::import_template_delete { id };
        import_templates::import_presets_restore {};
        jobs::market_refresh_cancel {};
        jobs::refresh_status {};
        settings::settings_save { settings };
        settings::ui_state_save { ui };
        sources::market_source_switch { source, on };
        sources::market_sources_confirm {};
        sources::market_custom_save { source };
        sources::market_custom_delete { id };
        ai::ai_tool_decide { request_id, decision };
        ai::ai_cancel {};
        demo::demo_seed {};
    }
}

/// Registered commands the browser host does not answer, and why in one word each.
pub const DESKTOP_ONLY: &[(&str, &str)] = &[
    // Take `AppHandle` (they emit `data:changed` or start a refresh).
    ("portfolio_save", "handle"),
    ("setup_portfolio", "handle"),
    ("profile_open", "handle"),
    ("account_save", "handle"),
    ("account_delete", "handle"),
    ("account_group_save", "handle"),
    ("account_group_delete", "handle"),
    ("scope_set", "handle"),
    ("attribute_def_save", "handle"),
    ("attribute_def_delete", "handle"),
    ("attributes_import_commit", "handle"),
    ("security_save", "handle"),
    ("security_delete", "handle"),
    ("corporate_action_save", "handle"),
    ("corporate_action_delete", "handle"),
    ("alert_save", "handle"),
    ("alert_delete", "handle"),
    ("alerts_take_notifications", "handle"),
    ("security_event_save", "handle"),
    ("security_event_delete", "handle"),
    ("security_set_listing", "handle"),
    ("transaction_save", "handle"),
    ("transaction_delete", "handle"),
    ("transfer_link", "handle"),
    ("inflation_region_set", "handle"),
    ("taxonomy_save", "handle"),
    ("taxonomy_delete", "handle"),
    ("taxonomy_import_commit", "handle"),
    ("taxonomy_group_commit", "handle"),
    ("taxonomy_node_save", "handle"),
    ("taxonomy_node_delete", "handle"),
    ("classification_save", "handle"),
    ("classification_delete", "handle"),
    ("taxonomy_exclude", "handle"),
    ("target_save", "handle"),
    ("target_delete", "handle"),
    ("import_prices_commit", "handle"),
    ("market_refresh", "handle"),
    ("plan_save", "handle"),
    ("plan_delete", "handle"),
    ("plan_commit", "handle"),
    ("goal_save", "handle"),
    ("goal_delete", "handle"),
    ("limit_save", "handle"),
    ("limit_delete", "handle"),
    ("watchlist_save", "handle"),
    ("watchlist_delete", "handle"),
    ("securities_adopt_source", "handle"),
    ("ai_chat_create", "handle"),
    ("ai_chat_delete", "handle"),
    ("ai_chat_rename", "handle"),
    ("ai_chat_set_mode", "handle"),
    ("ai_chat_set_effort", "handle"),
    ("ai_chat_set_model", "handle"),
    ("ai_chat_set_provider", "handle"),
    ("ai_send", "handle"),
    ("ai_brief", "handle"),
    ("dev_alert_simulate", "handle"),
    // `async`: run through `off_thread` on Tauri's own runtime.
    ("plugin_install", "async"),
    ("profile_delete", "async"),
    ("profile_unlock", "async"),
    ("profile_set_password", "async"),
    ("profile_remove_password", "async"),
    ("security_identify", "async"),
    ("security_listings", "async"),
    ("transactions_export_save", "async"),
    ("import_load", "async"),
    ("import_load_path", "async"),
    ("import_preview", "async"),
    ("import_commit", "async"),
    ("security_search", "async"),
    ("security_resolve", "async"),
    ("security_profile", "async"),
    ("import_resolve_symbol", "async"),
    ("market_custom_test", "async"),
    // A path on this machine: the browser has files as bytes, and nothing here should write.
    ("attributes_import_preview_path", "path"),
    ("attributes_import_commit_path", "path"),
    ("attributes_export_save", "path"),
    ("taxonomy_import_preview_path", "path"),
    ("taxonomy_import_commit_path", "path"),
    ("taxonomy_export_save", "path"),
    ("performance_sheet_save", "path"),
    ("report_save", "path"),
    ("notices_save", "path"),
    ("import_prices_load_path", "path"),
    ("ai_chat_export_save", "path"),
    // The keychain or the vault, which a throwaway profile must not reach.
    ("profile_create", "profiles"),
    ("profile_rename", "profiles"),
    ("profile_lock", "profiles"),
    ("profile_remember", "keychain"),
    ("ai_key_save", "keychain"),
    ("ai_key_delete", "keychain"),
    ("market_key_save", "keychain"),
    ("market_key_delete", "keychain"),
    ("plugin_remove", "plugins"),
    // The network.
    ("ai_models_list", "network"),
];
