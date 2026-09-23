//! Saved CSV import mappings: the user's own, stored beside the database, and the broker
//! layouts shipped with the app. Both are the same thing to the wizard — a layout with a
//! name — so they arrive in one list, the user's first.

use crate::error::{UiError, UiResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sq_core::import::{ImportMapping, ParseConfig, PresetMatch, best_match, builtin_presets};
use std::path::{Path, PathBuf};
use tauri::State;

/// Where a layout came from. A shipped one can be removed too, which is why removal is
/// recorded rather than performed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemplateSource {
    #[default]
    User,
    Builtin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTemplate {
    pub name: String,
    pub config: ParseConfig,
    pub mapping: ImportMapping,
    #[serde(default)]
    pub source: TemplateSource,
    /// How a file is recognised as this layout's. Absent means "by the columns it maps",
    /// which is what a layout saved from a real file already describes.
    #[serde(default, rename = "match", skip_serializing_if = "Option::is_none")]
    pub match_rule: Option<PresetMatch>,
}

impl ImportTemplate {
    fn rule(&self) -> PresetMatch {
        self.match_rule
            .clone()
            .unwrap_or_else(|| PresetMatch::of_columns(self.mapping.columns.values().cloned()))
    }
}

/// The layout a file belongs to, if exactly one does. The user's own come first in the list,
/// so a layout saved over a shipped name is the one that answers.
pub fn match_for(
    db_path: &Path,
    headers: &[String],
    file_name: Option<&str>,
    head: &str,
) -> Option<ImportTemplate> {
    let templates = listing(db_path);
    let name = best_match(
        templates.iter().map(|t| (t.name.as_str(), t.rule())),
        headers,
        file_name,
        head,
    )?
    .to_string();
    templates.into_iter().find(|t| t.name == name)
}

pub fn path_for(db_path: &Path) -> PathBuf {
    db_path.with_file_name("import_templates.json")
}

fn hidden_path_for(db_path: &Path) -> PathBuf {
    db_path.with_file_name("import_presets_hidden.json")
}

/// Load the user's own templates; invalid files are treated as an empty list.
pub fn load(db_path: &Path) -> Vec<ImportTemplate> {
    let mut templates: Vec<ImportTemplate> = std::fs::read(path_for(db_path))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    for template in &mut templates {
        template.source = TemplateSource::User;
    }
    templates
}

/// Names of shipped presets the user removed.
fn load_hidden(db_path: &Path) -> Vec<String> {
    std::fs::read(hidden_path_for(db_path))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn store_hidden(db_path: &Path, hidden: &[String]) -> UiResult<()> {
    let json = serde_json::to_vec_pretty(hidden).map_err(|e| UiError::internal(e.to_string()))?;
    std::fs::write(hidden_path_for(db_path), json).map_err(|e| UiError::internal(e.to_string()))
}

fn store(db_path: &Path, templates: &[ImportTemplate]) -> UiResult<()> {
    let json = serde_json::to_vec_pretty(templates).map_err(|e| UiError::internal(e.to_string()))?;
    std::fs::write(path_for(db_path), json).map_err(|e| UiError::internal(e.to_string()))
}

/// The user's layouts first, then the shipped ones. A shipped preset the user has saved
/// over by name is not listed twice: the user's own wins.
pub fn listing(db_path: &Path) -> Vec<ImportTemplate> {
    let mut out = load(db_path);
    out.sort_by(|a, b| a.name.cmp(&b.name));
    let hidden = load_hidden(db_path);
    for preset in builtin_presets() {
        if hidden.contains(&preset.name) || out.iter().any(|t| t.name == preset.name) {
            continue;
        }
        out.push(ImportTemplate {
            name: preset.name.clone(),
            config: preset.config.clone(),
            mapping: preset.mapping(),
            source: TemplateSource::Builtin,
            match_rule: preset.match_rule.clone(),
        });
    }
    out
}

/// Save a template, replacing an existing template with the same name.
pub fn save_template(
    db_path: &Path,
    name: &str,
    config: ParseConfig,
    mapping: ImportMapping,
) -> UiResult<Vec<ImportTemplate>> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(UiError::invalid("a layout must have a name"));
    }
    let mut templates = load(db_path);
    templates.retain(|t| t.name != name);
    templates.push(ImportTemplate {
        name,
        config,
        mapping,
        source: TemplateSource::User,
        // A layout saved from a real file already describes it: the columns it maps are that
        // file's header row, so it recognises the next export without a rule of its own.
        match_rule: None,
    });
    templates.sort_by(|a, b| a.name.cmp(&b.name));
    store(db_path, &templates)?;
    Ok(listing(db_path))
}

/// Removes what the user sees under this name. Their own copy goes for good; a shipped one
/// is only written down as removed, so `import_presets_restore` can bring it back.
pub fn delete_template(db_path: &Path, name: &str) -> UiResult<Vec<ImportTemplate>> {
    let mut templates = load(db_path);
    let had_own = templates.iter().any(|t| t.name == name);
    templates.retain(|t| t.name != name);
    store(db_path, &templates)?;

    if !had_own && builtin_presets().iter().any(|p| p.name == name) {
        let mut hidden = load_hidden(db_path);
        if !hidden.iter().any(|h| h == name) {
            hidden.push(name.to_string());
            store_hidden(db_path, &hidden)?;
        }
    }
    Ok(listing(db_path))
}

/// Brings back every shipped preset the user removed. Their own layouts are untouched.
pub fn restore_presets(db_path: &Path) -> UiResult<Vec<ImportTemplate>> {
    store_hidden(db_path, &[])?;
    Ok(listing(db_path))
}

#[tauri::command]
pub fn import_templates_list(state: State<AppState>) -> UiResult<Vec<ImportTemplate>> {
    Ok(listing(&state.db_path()?))
}

#[tauri::command]
pub fn import_template_save(
    state: State<AppState>,
    name: String,
    config: ParseConfig,
    mapping: ImportMapping,
) -> UiResult<Vec<ImportTemplate>> {
    save_template(&state.db_path()?, &name, config, mapping)
}

#[tauri::command]
pub fn import_template_delete(state: State<AppState>, name: String) -> UiResult<Vec<ImportTemplate>> {
    delete_template(&state.db_path()?, &name)
}

#[tauri::command]
pub fn import_presets_restore(state: State<AppState>) -> UiResult<Vec<ImportTemplate>> {
    restore_presets(&state.db_path()?)
}
