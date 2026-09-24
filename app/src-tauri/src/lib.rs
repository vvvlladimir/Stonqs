//! Tauri host built on top of [`sq_core`].

// A bare `unwrap()` outside tests is a crash in somebody's portfolio. An invariant that really
// cannot fail is written as `expect("why")`, so the reason survives into the panic message.
#![cfg_attr(not(test), deny(clippy::unwrap_used))]

pub mod ai;
pub mod commands;
pub mod dbfile;
pub mod demo;
pub mod error;
pub mod events;
pub mod import_templates;
pub mod jobs;
pub mod plugins;
pub mod profiles;
pub mod scope;
pub mod secrets;
pub mod settings;
pub mod state;
pub mod vault;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        // The dialog plugin is only here for file pickers and save locations.
        .plugin(tauri_plugin_dialog::init())
        // Notifications for fired alerts; the frontend asks for permission and writes the text.
        .plugin(tauri_plugin_notification::init());

    // In-app updates, desktop only — a store updates the mobile builds. The frontend asks and
    // decides; the plugin fetches, checks the signature against the key in tauri.conf.json,
    // installs, and `process` restarts into the new version.
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init());

    builder
        .setup(|app| {
            let state = AppState::bootstrap(app)?;
            // A locked profile refreshes once it is unlocked (`profile_unlock`), not before.
            let due = !state.is_locked() && state.settings()?.due(chrono::Utc::now());
            app.manage(state);

            // Refresh on startup when the throttle allows it.
            if due {
                let handle = app.handle().clone();
                let state = handle.state::<AppState>();
                jobs::start(&handle, &state, jobs::RefreshMode::CatchUp);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::dashboard::app_status,
            commands::dashboard::dashboard_summary,
            commands::portfolio::portfolio_get,
            commands::portfolio::portfolio_save,
            commands::portfolio::setup_portfolio,
            commands::plugins::plugins_list,
            commands::plugins::plugin_install,
            commands::plugins::plugin_remove,
            commands::plugins::plugin_theme_css,
            commands::profiles::profiles_list,
            commands::profiles::profile_create,
            commands::profiles::profile_rename,
            commands::profiles::profile_delete,
            commands::profiles::profile_open,
            commands::profiles::profile_unlock,
            commands::profiles::profile_lock,
            commands::profiles::profile_set_password,
            commands::profiles::profile_remove_password,
            commands::profiles::profile_remember,
            commands::accounts::accounts_list,
            commands::accounts::accounts_total,
            commands::accounts::account_save,
            commands::accounts::account_delete,
            commands::accounts::account_groups_list,
            commands::accounts::account_group_save,
            commands::accounts::account_group_delete,
            scope::scope_get,
            scope::scope_set,
            commands::securities::securities_list,
            commands::attributes::attribute_defs_list,
            commands::attributes::attribute_def_save,
            commands::attributes::attribute_def_delete,
            commands::attributes::attributes_import_preview,
            commands::attributes::attributes_import_preview_path,
            commands::attributes::attributes_import_commit,
            commands::attributes::attributes_import_commit_path,
            commands::attributes::attributes_export_csv,
            commands::attributes::attributes_export_save,
            commands::securities::security_save,
            commands::securities::security_delete,
            commands::corporate_actions::corporate_actions_list,
            commands::corporate_actions::corporate_action_save,
            commands::corporate_actions::corporate_action_delete,
            commands::alerts::alerts_list,
            commands::alerts::alert_save,
            commands::alerts::alert_delete,
            commands::alerts::alert_crossings_list,
            commands::alerts::alerts_unseen,
            commands::alerts::alerts_mark_seen,
            commands::alerts::alerts_take_notifications,
            commands::alerts::security_events_list,
            commands::alerts::security_event_save,
            commands::alerts::security_event_delete,
            commands::securities::security_identify,
            commands::securities::quotes_range,
            commands::listings::security_listings,
            commands::listings::security_set_listing,
            commands::positions::positions_at,
            commands::positions::positions_cost_basis,
            commands::positions::position_return,
            commands::positions::position_returns,
            commands::transactions::transactions_list,
            commands::transactions::transactions_export,
            commands::transactions::transactions_export_save,
            commands::transactions::transaction_save,
            commands::transactions::transaction_delete,
            commands::transactions::transfer_suggestions,
            commands::transactions::transfer_link,
            commands::periods::period_ranges,
            commands::periods::periods_get,
            commands::periods::period_save,
            commands::periods::period_delete,
            commands::periods::periods_restore,
            commands::performance::performance_summary,
            commands::performance::performance_breakdown,
            commands::performance::performance_sheet_save,
            commands::trades::trades_summary,
            commands::payments::payments_grid,
            commands::payments::dividends_expected,
            commands::performance::risk_report,
            commands::performance::benchmark_compare,
            commands::performance::benchmark_series,
            commands::inflation::inflation_status,
            commands::inflation::inflation_region_set,
            commands::inflation::real_performance,
            commands::inflation::inflation_series,
            commands::allocation::allocation,
            commands::allocation::allocation_members,
            commands::allocation::allocation_tree,
            commands::allocation::taxonomies_list,
            commands::allocation::taxonomy_save,
            commands::allocation::taxonomy_delete,
            commands::allocation::taxonomy_import_preview,
            commands::allocation::taxonomy_import_preview_path,
            commands::allocation::taxonomy_import_commit,
            commands::allocation::taxonomy_import_commit_path,
            commands::allocation::taxonomy_group_preview,
            commands::allocation::taxonomy_group_commit,
            commands::allocation::taxonomy_export_csv,
            commands::allocation::taxonomy_export_save,
            commands::allocation::taxonomy_node_save,
            commands::allocation::taxonomy_node_delete,
            commands::allocation::classification_save,
            commands::allocation::classification_delete,
            commands::allocation::taxonomy_exclude,
            commands::allocation::targets_list,
            commands::allocation::target_save,
            commands::allocation::target_delete,
            commands::allocation::rebalance_plan,
            commands::reports::reports_summary,
            commands::reports::income_summary,
            commands::reports::income_taxonomy,
            commands::reports::report_export,
            commands::reports::report_save,
            commands::import::import_load,
            commands::import::import_load_path,
            commands::import::import_prices_load_path,
            commands::import::import_file_info,
            commands::import::import_clear,
            commands::import::import_preview,
            commands::import::import_commit,
            commands::import::import_prices_load,
            commands::import::import_prices_preview,
            commands::import::import_prices_commit,
            commands::lookup::security_search,
            commands::lookup::security_resolve,
            commands::lookup::security_profile,
            commands::lookup::import_resolve_symbol,
            import_templates::import_templates_list,
            import_templates::import_template_save,
            import_templates::import_template_delete,
            import_templates::import_presets_restore,
            jobs::market_refresh,
            jobs::market_refresh_cancel,
            jobs::refresh_status,
            jobs::quote_providers,
            jobs::data_coverage,
            commands::plans::plans_list,
            commands::plans::plan_save,
            commands::plans::plan_delete,
            commands::plans::plan_due,
            commands::plans::plan_commit,
            commands::plans::plan_projection,
            commands::plans::fire_projection,
            commands::goals::goals_list,
            commands::goals::goal_save,
            commands::goals::goal_delete,
            commands::goals::limits_list,
            commands::goals::limit_save,
            commands::goals::limit_delete,
            commands::watchlists::watchlists_list,
            commands::watchlists::watchlist_save,
            commands::watchlists::watchlist_delete,
            commands::watchlists::watchlist_rows,
            settings::settings_get,
            settings::settings_save,
            settings::ui_state_save,
            commands::ai::ai_key_save,
            commands::sources::market_sources_list,
            commands::sources::market_source_switch,
            commands::sources::market_key_save,
            commands::sources::market_key_delete,
            commands::sources::market_custom_list,
            commands::sources::market_custom_save,
            commands::sources::market_custom_delete,
            commands::sources::market_custom_test,
            commands::ai::ai_key_delete,
            commands::ai::ai_key_status,
            commands::ai::ai_chats_list,
            commands::ai::ai_chat_create,
            commands::ai::ai_chat_delete,
            commands::ai::ai_chat_export,
            commands::ai::ai_chat_export_save,
            commands::ai::ai_messages_list,
            commands::ai::ai_chat_rename,
            commands::ai::ai_chat_set_mode,
            commands::ai::ai_chat_set_effort,
            commands::ai::ai_chat_set_model,
            commands::ai::ai_chat_set_provider,
            commands::ai::ai_providers_list,
            commands::ai::ai_models_list,
            commands::ai::ai_grants_list,
            commands::ai::ai_tool_decide,
            commands::ai::ai_cancel,
            commands::ai::ai_send,
            commands::ai::ai_brief,
            commands::ai::ai_usage_totals,
            commands::demo::demo_seed,
            commands::dev::dev_alert_simulate,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start the application");
}
