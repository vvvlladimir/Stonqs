//! Plugins: list what is installed, install a folder the user picked, remove one, and hand over
//! the files of the content in use — a theme's stylesheet, a classification set's CSV.
//! See ADR-0070.
//!
//! Nothing here takes the store, so a locked profile still has its colours. A set's CSV goes back
//! to the frontend and in again through `taxonomy_import_preview`, which is the point: a set from
//! a plugin has no path into the portfolio of its own.

use crate::error::{UiError, UiResult};
use crate::plugins::{PluginInfo, ScreenInfo, TaxonomySetInfo, ThemeInfo, ToolInfo, WidgetInfo, WriterInfo};
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
    /// The screens that can actually be opened, addressed as the navigation hint names them.
    pub screens: Vec<ScreenInfo>,
    /// The assistant tools that are offered to the model.
    pub tools: Vec<ToolInfo>,
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
        screens: state.plugins.screens()?,
        tools: state.plugins.tools()?.into_iter().map(|t| t.info).collect(),
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

/// A plugin's one document in the open profile, or null before it saved one (ADR-0084).
#[tauri::command]
pub fn plugin_state_get(state: State<AppState>, plugin: String) -> UiResult<Option<serde_json::Value>> {
    if !state.plugins.keeps_state(&plugin)? {
        return Err(UiError::invalid(format!("plugin {plugin} declares no storage")));
    }
    let store = state.store()?;
    let Some(text) = store.plugin_state(&plugin)? else {
        return Ok(None);
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| UiError::internal(e.to_string()))
}

/// Replaces a plugin's document. Its own data, not the portfolio's, so nothing is asked: it can
/// change no operation, account or figure, and nothing in the app reads it.
#[tauri::command]
pub fn plugin_state_save(
    state: State<AppState>,
    plugin: String,
    document: serde_json::Value,
) -> UiResult<()> {
    if !state.plugins.keeps_state(&plugin)? {
        return Err(UiError::invalid(format!("plugin {plugin} declares no storage")));
    }
    let text = document.to_string();
    if text.len() > crate::plugins::STATE_LIMIT {
        return Err(UiError::invalid(format!(
            "plugin {plugin} tried to keep {} bytes; the limit is {}",
            text.len(),
            crate::plugins::STATE_LIMIT
        )));
    }
    state.store()?.save_plugin_state(&plugin, &text)?;
    Ok(())
}

/// The scheme a plugin page is served from (ADR-0083). Its own origin, its own policy, and a
/// frame the app's IPC is never injected into.
pub const PAGE_SCHEME: &str = "stonqs-plugin";

/// Answers `/<widget|screen>/<plugin id>/<id>` with that page. The frontend builds the address with
/// `convertFileSrc`, which encodes the slashes; an id is `[a-z0-9._-]`, so that is all it encodes.
pub fn page(
    app: &tauri::AppHandle,
    request: &tauri::http::Request<Vec<u8>>,
) -> tauri::http::Response<Vec<u8>> {
    use crate::plugins::PageKind;
    use tauri::Manager;
    use tauri::http::{Response, StatusCode, header};

    let path = request
        .uri()
        .path()
        .trim_start_matches('/')
        .replace("%2F", "/")
        .replace("%2f", "/");
    let mut parts = path.splitn(3, '/');
    let page = match (parts.next().and_then(PageKind::parse), parts.next(), parts.next()) {
        (Some(kind), Some(plugin), Some(id)) => app.state::<AppState>().plugins.page(kind, plugin, id),
        _ => Err(UiError::not_found(path.clone())),
    };
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
