//! Plugins: list what is installed, install a folder the user picked, remove one, and hand over
//! the stylesheet of the theme in use. See ADR-0070.
//!
//! A theme reads no portfolio data, so nothing here takes the store — and a locked profile still
//! has its colours.

use crate::error::UiResult;
use crate::plugins::{PluginInfo, ThemeInfo};
use crate::state::AppState;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct PluginList {
    pub plugins: Vec<PluginInfo>,
    /// The themes that can actually be applied, already addressed as the preference stores them.
    pub themes: Vec<ThemeInfo>,
    /// The plugin API this build speaks, so the list can say what a refused package wanted.
    pub api: u32,
}

#[tauri::command]
pub fn plugins_list(state: State<AppState>) -> UiResult<PluginList> {
    Ok(PluginList {
        plugins: state.plugins.list()?,
        themes: state.plugins.themes()?,
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
