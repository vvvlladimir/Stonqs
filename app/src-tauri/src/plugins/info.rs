//! What the host tells the UI about installed plugins.

use super::manifest::{API, Base, Manifest, Provides};
use super::{Read, Size, reader};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Why a plugin is not in use. A code, never a sentence: the wording is the frontend's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Status {
    Ok,
    /// Built for another plugin API than this build speaks.
    Api {
        wants: u32,
        speaks: u32,
    },
    /// The folder is there and its manifest is not readable.
    Broken {
        detail: String,
    },
    /// The folder's name is not its manifest's id; nothing is offered until it is reinstalled.
    Misplaced {
        manifest_id: String,
    },
}

pub(super) fn status_of(manifest: &Manifest) -> Status {
    if manifest.api == API {
        Status::Ok
    } else {
        Status::Api {
            wants: manifest.api,
            speaks: API,
        }
    }
}

/// One installed plugin as the list shows it, `provides` flattened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(flatten)]
    pub provides: Provides,
    #[serde(flatten)]
    pub status: Status,
}

impl PluginInfo {
    pub(super) fn from_manifest(manifest: Manifest) -> Self {
        PluginInfo {
            status: status_of(&manifest),
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            provides: manifest.provides,
        }
    }

    /// A folder offering nothing; `id` is the folder's own name, so removing it removes that folder.
    pub(super) fn unusable(id: &str, name: String, status: Status) -> Self {
        PluginInfo {
            id: id.to_string(),
            name,
            version: String::new(),
            provides: Provides::default(),
            status,
        }
    }
}

/// Every `key` below is `<plugin id>/<content id>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaxonomySetInfo {
    pub key: String,
    pub name: String,
    pub plugin: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WriterInfo {
    pub key: String,
    pub name: String,
    pub plugin: String,
    pub extension: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolInfo {
    pub key: String,
    pub name: String,
    pub plugin: String,
    pub plugin_name: String,
    pub reads: Vec<Read>,
}

/// A tool ready for the model, built per message so a mid-chat install is seen by the next one.
#[derive(Debug, Clone)]
pub struct LoadedTool {
    pub info: ToolInfo,
    /// `plugin_<plugin>_<tool>`, unique in the catalogue.
    pub model_name: String,
    pub description: String,
    pub schema: serde_json::Value,
    pub periodic: bool,
    pub module: PathBuf,
}

/// Providers accept `[a-zA-Z0-9_-]{1,64}`; the prefix keeps it apart from the app's own tools.
pub fn tool_model_name(plugin: &str, tool: &str) -> String {
    let clean = |s: &str| s.replace(['.', '-'], "_");
    format!("plugin_{}_{}", clean(plugin), clean(tool))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScreenInfo {
    pub key: String,
    pub name: String,
    pub description: String,
    pub plugin: String,
    /// Always shown in the screen's header (ADR-0082).
    pub plugin_name: String,
    pub reads: Vec<Read>,
    pub periodic: bool,
    pub storage: bool,
}

/// The first path segment of a `stonqs-plugin` request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    Widget,
    Screen,
}

impl PageKind {
    pub fn parse(segment: &str) -> Option<Self> {
        match segment {
            "widget" => Some(PageKind::Widget),
            "screen" => Some(PageKind::Screen),
            _ => None,
        }
    }
}

/// The largest document a plugin may keep (ADR-0084).
pub const STATE_LIMIT: usize = 256 * 1024;

/// A board stores its type as `plugin:<key>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WidgetInfo {
    pub key: String,
    pub name: String,
    pub description: String,
    pub plugin: String,
    /// Every tile it draws carries it (ADR-0082).
    pub plugin_name: String,
    pub reads: Vec<Read>,
    pub periodic: bool,
    pub size: Size,
    pub min: Size,
}

/// `UiState::theme` holds `plugin:<key>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThemeInfo {
    pub key: String,
    pub name: String,
    pub plugin: String,
    pub base: Base,
}

/// A sealed file's password, for the one reader that asked. No `Debug`; wiped on drop.
#[derive(Deserialize)]
pub struct Unlock {
    /// `<plugin id>/<reader id>`, as `UiError::FileProtected` named it.
    pub reader: String,
    pub password: String,
}

impl Drop for Unlock {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.password.zeroize();
    }
}

/// What the installed readers made of one file.
#[derive(Debug, Default)]
pub struct FileReading {
    /// The reader that claimed it and its reading; `None` leaves the file to the app's own.
    pub read: Option<(String, reader::Reading)>,
    /// Readers that broke over the file and were passed over.
    pub skipped: Vec<reader::SkippedReader>,
}
