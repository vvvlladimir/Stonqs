//! Plugins: list what is installed, install a folder the user picked, remove one, and hand over
//! the files of the content in use — a theme's stylesheet, a classification set's CSV.
//! See ADR-0070.
//!
//! Nothing here takes the store, so a locked profile still has its colours. A set's CSV goes back
//! to the frontend and in again through `taxonomy_import_preview`, which is the point: a set from
//! a plugin has no path into the portfolio of its own.

use crate::error::UiResult;
use crate::plugins::{PluginInfo, TaxonomySetInfo, ThemeInfo, WidgetInfo, WriterInfo};
use crate::state::AppState;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct PluginList {
    pub plugins: Vec<PluginInfo>,
    /// The themes that can actually be applied, already addressed as the preference stores them.
    pub themes: Vec<ThemeInfo>,
    /// The classification sets that can actually be created, addressed as a command names them.
    pub taxonomy_sets: Vec<TaxonomySetInfo>,
    /// The export formats that can actually be written, addressed as `transactions_export_save`
    /// names them.
    pub writers: Vec<WriterInfo>,
    /// The dashboard widgets that can actually be placed, addressed as a board stores their type.
    pub widgets: Vec<WidgetInfo>,
    /// The plugin API this build speaks, so the list can say what a refused package wanted.
    pub api: u32,
}

#[tauri::command]
pub fn plugins_list(state: State<AppState>) -> UiResult<PluginList> {
    Ok(PluginList {
        plugins: state.plugins.list()?,
        themes: state.plugins.themes()?,
        taxonomy_sets: state.plugins.taxonomy_sets()?,
        writers: state.plugins.writers()?,
        widgets: state.plugins.widgets()?,
        api: crate::plugins::API,
    })
}

/// Installs from a folder the user chose in the file picker. A path, not bytes: this is the one
/// thing that is a directory rather than a file, and the picker hands its path over.
#[tauri::command]
pub fn plugin_install(state: State<AppState>, path: PathBuf) -> UiResult<PluginInfo> {
    state.plugins.install(&path)
}

#[tauri::command]
pub fn plugin_remove(state: State<AppState>, id: String) -> UiResult<()> {
    state.plugins.remove(&id)
}

/// The stylesheet of one theme. Read on demand: only the chosen one is ever loaded.
#[tauri::command]
pub fn plugin_theme_css(state: State<AppState>, plugin: String, theme: String) -> UiResult<String> {
    state.plugins.theme_css(&plugin, &theme)
}

/// A classification set's CSV. Read on demand and handed over as bytes, so the wizard that
/// previews and commits it is the one every taxonomy file goes through.
#[tauri::command]
pub fn plugin_taxonomy_csv(state: State<AppState>, plugin: String, set: String) -> UiResult<Vec<u8>> {
    state.plugins.taxonomy_csv(&plugin, &set)
}

/// The scheme a widget page is served from (ADR-0083). Its own origin, its own policy, and a
/// frame the app's IPC is never injected into.
pub const WIDGET_SCHEME: &str = "stonqs-plugin";

/// Answers `/<plugin id>/<widget id>` with the widget's page. The frontend builds the address with
/// `convertFileSrc`, which encodes the slash; an id is `[a-z0-9._-]`, so that is all it encodes.
pub fn widget_page(
    app: &tauri::AppHandle,
    request: &tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    use tauri::Manager;
    use tauri::http::{Response, StatusCode, header};

    let path = request
        .uri()
        .path()
        .trim_start_matches('/')
        .replace("%2F", "/")
        .replace("%2f", "/");
    let page = path
        .split_once('/')
        .ok_or_else(|| crate::error::UiError::not_found(path.clone()))
        .and_then(|(plugin, widget)| app.state::<AppState>().plugins.widget_page(plugin, widget));
    let built = match page {
        Ok((html, csp)) => Response::builder()
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .header(header::CONTENT_SECURITY_POLICY, csp)
            .body(html.into_bytes()),
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header(header::CONTENT_SECURITY_POLICY, "default-src 'none'")
            .body(Vec::new()),
    };
    built.unwrap_or_else(|_| Response::new(Vec::new()))
}
