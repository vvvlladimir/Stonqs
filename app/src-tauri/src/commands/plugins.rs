//! Plugins: list what is installed, install a folder the user picked, remove one, and hand over
//! the files of the content in use — a theme's stylesheet, a classification set's CSV.
//! See ADR-0070.
//!
//! Nothing here takes the store, so a locked profile still has its colours. A set's CSV goes back
//! to the frontend and in again through `taxonomy_import_preview`, which is the point: a set from
//! a plugin has no path into the portfolio of its own.

use crate::error::UiResult;
use crate::plugins::{PluginInfo, TaxonomySetInfo, ThemeInfo};
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
    /// The plugin API this build speaks, so the list can say what a refused package wanted.
    pub api: u32,
}

#[tauri::command]
pub fn plugins_list(state: State<AppState>) -> UiResult<PluginList> {
    Ok(PluginList {
        plugins: state.plugins.list()?,
        themes: state.plugins.themes()?,
        taxonomy_sets: state.plugins.taxonomy_sets()?,
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
