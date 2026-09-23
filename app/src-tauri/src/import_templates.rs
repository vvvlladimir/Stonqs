//! Saved CSV import mappings: the user's own, stored beside the database, and the broker
//! layouts shipped with the app. Both are the same thing to the wizard — a layout with a
//! name — so they arrive in one list, the user's first.

use crate::error::{UiError, UiResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use sq_core::import::{ImportMapping, ImportService, ParseConfig, PresetMatch, best_match, builtin_presets};
use sq_core::model::Account;
use sq_core::storage::Store;
use std::path::{Path, PathBuf};
use tauri::State;

/// Where a layout came from. A shipped one can be removed too, which is why removal is
/// recorded rather than performed; one from a plugin is removed by removing the plugin.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TemplateSource {
    #[default]
    User,
    Builtin,
    Plugin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTemplate {
    /// What names this layout everywhere but on screen: `user:`, `builtin:` or a plugin's own
    /// `<plugin id>/<layout id>`. A name is what the user reads and two sources may print the
    /// same one, so identity cannot be it (ADR-0070).
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub config: ParseConfig,
    pub mapping: ImportMapping,
    #[serde(default)]
    pub source: TemplateSource,
    /// The plugin a `PLUGIN` layout came from, so the list can say what to remove instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
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

/// The id a layout the user saved is known by. Derived from the name rather than stored: a saved
/// layout has always been keyed by its name, and inventing ids for what is already on disk would
/// be a migration for nothing.
fn user_id(name: &str) -> String {
    format!("user:{name}")
}

fn builtin_id(name: &str) -> String {
    format!("builtin:{name}")
}

/// The layout a file belongs to, if exactly one does. The user's own come first in the list,
/// so a layout saved over a shipped name is the one that answers.
pub fn match_for(
    db_path: &Path,
    plugins: &crate::plugins::Plugins,
    headers: &[String],
    file_name: Option<&str>,
    head: &str,
) -> Option<ImportTemplate> {
    let templates = listing(db_path, plugins);
    let id = best_match(
        templates.iter().map(|t| (t.id.as_str(), t.rule())),
        headers,
        file_name,
        head,
    )?
    .to_string();
    templates.into_iter().find(|t| t.id == id)
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
        template.id = user_id(&template.name);
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

/// The user's layouts first, then the plugins', then the shipped ones. One name is listed once:
/// the user's own wins over a plugin's, and a plugin's over a shipped one, because that is the
/// order of who chose it.
pub fn listing(db_path: &Path, plugins: &crate::plugins::Plugins) -> Vec<ImportTemplate> {
    let mut out = load(db_path);
    out.sort_by(|a, b| a.name.cmp(&b.name));

    for (id, preset) in plugins.layouts().unwrap_or_default() {
        if out.iter().any(|t| t.name == preset.name) {
            continue;
        }
        let plugin = id.split('/').next().unwrap_or_default().to_string();
        out.push(ImportTemplate {
            id,
            name: preset.name.clone(),
            config: preset.config.clone(),
            mapping: preset.mapping(),
            source: TemplateSource::Plugin,
            plugin: Some(plugin),
            match_rule: preset.match_rule.clone(),
        });
    }

    let hidden = load_hidden(db_path);
    for preset in builtin_presets() {
        if hidden.contains(&preset.name) || out.iter().any(|t| t.name == preset.name) {
            continue;
        }
        out.push(ImportTemplate {
            id: builtin_id(&preset.name),
            name: preset.name.clone(),
            config: preset.config.clone(),
            mapping: preset.mapping(),
            source: TemplateSource::Builtin,
            plugin: None,
            match_rule: preset.match_rule.clone(),
        });
    }
    out
}

/// Save a template, replacing an existing template with the same name.
pub fn save_template(
    db_path: &Path,
    plugins: &crate::plugins::Plugins,
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
        id: user_id(&name),
        name,
        config,
        mapping,
        source: TemplateSource::User,
        plugin: None,
        // A layout saved from a real file already describes it: the columns it maps are that
        // file's header row, so it recognises the next export without a rule of its own.
        match_rule: None,
    });
    templates.sort_by(|a, b| a.name.cmp(&b.name));
    store(db_path, &templates)?;
    Ok(listing(db_path, plugins))
}

/// Removes the layout with this id. The user's own copy goes for good; a shipped one is only
/// written down as removed, so `import_presets_restore` can bring it back. A plugin's is refused:
/// the plugin is what installed it, and removing half a package is not a state to leave behind.
pub fn delete_template(
    db_path: &Path,
    plugins: &crate::plugins::Plugins,
    id: &str,
) -> UiResult<Vec<ImportTemplate>> {
    if let Some(name) = id.strip_prefix("user:") {
        let mut templates = load(db_path);
        templates.retain(|t| t.name != name);
        store(db_path, &templates)?;
        return Ok(listing(db_path, plugins));
    }

    if let Some(name) = id.strip_prefix("builtin:") {
        let mut hidden = load_hidden(db_path);
        if !hidden.iter().any(|h| h == name) {
            hidden.push(name.to_string());
            store_hidden(db_path, &hidden)?;
        }
        return Ok(listing(db_path, plugins));
    }

    Err(UiError::invalid(format!(
        "layout {id} belongs to a plugin; remove the plugin instead"
    )))
}

/// Brings back every shipped preset the user removed. Their own layouts are untouched.
pub fn restore_presets(db_path: &Path, plugins: &crate::plugins::Plugins) -> UiResult<Vec<ImportTemplate>> {
    store_hidden(db_path, &[])?;
    Ok(listing(db_path, plugins))
}

/// What a layout must prove before a package carrying it is installed: it recognises the sample
/// the package ships, and that sample reads without leaving a question for the user. The shipped
/// layouts answer the same two questions in `core/tests/fixtures/presets/`, plus a third — the
/// operations they produce — which needs an expectation a stranger's package has no reason to
/// carry in the app's own format.
pub fn check_layout(id: &str, preset_json: &str, sample: &[u8], sample_name: &str) -> UiResult<()> {
    let preset: sq_core::import::BrokerPreset =
        serde_json::from_str(preset_json).map_err(|e| UiError::invalid(format!("layout {id}: {e}")))?;

    let parsed = sq_core::import::parse_file(sample, &preset.config)
        .map_err(|e| UiError::invalid(format!("layout {id}: its own sample does not parse: {e}")))?;
    let head = String::from_utf8_lossy(&sample[..sample.len().min(2048)]).into_owned();
    if preset
        .match_rule()
        .score(&parsed.headers, Some(sample_name), &head)
        .is_none()
    {
        return Err(UiError::invalid(format!(
            "layout {id} does not recognise the sample it ships"
        )));
    }

    // An in-memory database, so nothing here can touch the open portfolio.
    let store = Store::open_in_memory().map_err(|e| UiError::internal(e.to_string()))?;
    let cash = Account::deposit("Check", "EUR");
    store
        .save_account(&cash)
        .map_err(|e| UiError::internal(e.to_string()))?;
    let depot = Account::securities("Check", "EUR", &cash.id);
    store
        .save_account(&depot)
        .map_err(|e| UiError::internal(e.to_string()))?;

    let mapping = preset.mapping().with_account(&depot.id);
    let preview = ImportService::new(&store)
        .preview(sample, &preset.config, Some(&mapping), &[])
        .map_err(|e| UiError::invalid(format!("layout {id}: its own sample does not read: {e}")))?;

    let unknown = preview.unknown_kinds();
    if !unknown.is_empty() {
        return Err(UiError::invalid(format!(
            "layout {id} leaves {} wording(s) of its own sample unmapped: {}",
            unknown.len(),
            unknown.join(", ")
        )));
    }
    if preview.summary.invalid > 0 {
        return Err(UiError::invalid(format!(
            "layout {id} reads {} row(s) of its own sample as invalid",
            preview.summary.invalid
        )));
    }
    Ok(())
}

#[tauri::command]
pub fn import_templates_list(state: State<AppState>) -> UiResult<Vec<ImportTemplate>> {
    Ok(listing(&state.db_path()?, &state.plugins))
}

#[tauri::command]
pub fn import_template_save(
    state: State<AppState>,
    name: String,
    config: ParseConfig,
    mapping: ImportMapping,
) -> UiResult<Vec<ImportTemplate>> {
    save_template(&state.db_path()?, &state.plugins, &name, config, mapping)
}

#[tauri::command]
pub fn import_template_delete(state: State<AppState>, id: String) -> UiResult<Vec<ImportTemplate>> {
    delete_template(&state.db_path()?, &state.plugins, &id)
}

#[tauri::command]
pub fn import_presets_restore(state: State<AppState>) -> UiResult<Vec<ImportTemplate>> {
    restore_presets(&state.db_path()?, &state.plugins)
}
