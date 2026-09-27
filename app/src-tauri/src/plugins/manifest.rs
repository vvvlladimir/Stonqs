//! The package format: `plugin.json` and the content it declares.

use super::{ToolDef, WidgetDef, widget::ScreenDef};
use crate::error::{UiError, UiResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub(super) const MANIFEST: &str = "plugin.json";

/// The plugin API this build speaks; a package naming another is listed, not loaded.
pub const API: u32 = 1;

/// An id is a folder name on three platforms and a vault key, so it is deliberately dull.
pub(super) fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
}

/// Unknown fields are ignored, so a package for a later build is listed rather than refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub api: u32,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub provides: Provides,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provides {
    #[serde(default)]
    pub themes: Vec<ThemeDef>,
    #[serde(default)]
    pub layouts: Vec<LayoutDef>,
    #[serde(default)]
    pub readers: Vec<ReaderDef>,
    #[serde(default)]
    pub taxonomies: Vec<TaxonomyDef>,
    #[serde(default)]
    pub dictionaries: Vec<DictionaryDef>,
    #[serde(default)]
    pub writers: Vec<WriterDef>,
    #[serde(default)]
    pub widgets: Vec<WidgetDef>,
    #[serde(default)]
    pub screens: Vec<ScreenDef>,
    #[serde(default)]
    pub tools: Vec<ToolDef>,
}

/// A WASM component writing the app's transaction file as another format (ADR-0080).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriterDef {
    pub id: String,
    /// The plugin's own words, shown in the export.
    pub name: String,
    /// The `.wasm` component.
    pub file: String,
    /// A `stonqs.transactions` document.
    pub sample: String,
    /// What `sample` must be written as, byte for byte.
    pub expected: String,
    /// The saved file's ending, without the dot (`journal`).
    pub extension: String,
}

/// Operation wordings per language, read after the shipped keywords and never instead of them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryDef {
    pub id: String,
    /// `{ "words": [{ "word", "kind" }] }`; the first word a wording contains wins.
    pub file: String,
    /// A file only this dictionary makes readable.
    pub sample: String,
}

/// A ready classification tree in the taxonomy import's own CSV.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonomyDef {
    pub id: String,
    /// The new tree's name; the user's data afterwards, never translated.
    pub name: String,
    pub file: String,
}

/// A WASM component reading a file into the app's transaction file (ADR-0086).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderDef {
    pub id: String,
    /// The `.wasm` component.
    pub file: String,
    /// A redacted file of the kind this reader exists for.
    pub sample: String,
    /// What `sample` must read as: a `stonqs.transactions` document.
    pub expected: String,
    /// Lower-case endings with the dot (`.pdf`); empty = any file the shipped readers skipped.
    #[serde(default)]
    pub extensions: Vec<String>,
}

/// A broker layout in `presets/brokers.json` shape, plus the export it must read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutDef {
    pub id: String,
    pub file: String,
    /// A redacted export this layout must recognise and read without a question.
    pub sample: String,
}

/// A stylesheet over the app's custom properties; what it leaves out comes from `base`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeDef {
    pub id: String,
    pub name: String,
    pub file: String,
    #[serde(default)]
    pub base: Base,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Base {
    Light,
    #[default]
    Dark,
}

impl Provides {
    /// Nothing this build can use: the shape a misspelled `provides` takes.
    pub(super) fn is_empty(&self) -> bool {
        self.themes.is_empty()
            && self.layouts.is_empty()
            && self.readers.is_empty()
            && self.taxonomies.is_empty()
            && self.dictionaries.is_empty()
            && self.writers.is_empty()
            && self.widgets.is_empty()
            && self.screens.is_empty()
            && self.tools.is_empty()
    }

    /// Every file the content names besides the manifest: exactly what an install copies.
    pub(super) fn files(&self) -> Vec<&str> {
        let mut files: Vec<&str> = Vec::new();
        files.extend(self.themes.iter().map(|d| d.file.as_str()));
        for d in &self.layouts {
            files.extend([d.file.as_str(), d.sample.as_str()]);
        }
        for d in &self.readers {
            files.extend([d.file.as_str(), d.sample.as_str(), d.expected.as_str()]);
        }
        for d in &self.writers {
            files.extend([d.file.as_str(), d.sample.as_str(), d.expected.as_str()]);
        }
        files.extend(self.widgets.iter().map(|d| d.file.as_str()));
        files.extend(self.screens.iter().map(|d| d.file.as_str()));
        for d in &self.tools {
            files.extend([
                d.file.as_str(),
                d.schema.as_str(),
                d.sample.as_str(),
                d.expected.as_str(),
            ]);
        }
        files.extend(self.taxonomies.iter().map(|d| d.file.as_str()));
        for d in &self.dictionaries {
            files.extend([d.file.as_str(), d.sample.as_str()]);
        }
        files
    }
}

pub(super) fn read_manifest(folder: &Path) -> Result<Manifest, String> {
    let text = std::fs::read_to_string(folder.join(MANIFEST)).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// A file name from a manifest, resolved inside the package; `../` and absolute paths are refused.
pub(super) fn safe_join(folder: &Path, name: &str) -> UiResult<PathBuf> {
    let candidate = Path::new(name);
    let sane = candidate
        .components()
        .all(|c| matches!(c, std::path::Component::Normal(_)));
    if name.is_empty() || !sane {
        return Err(UiError::invalid(format!(
            "{name:?} is not a file inside the plugin"
        )));
    }
    Ok(folder.join(candidate))
}

/// Content ids are keys after `<plugin id>/`, so they must be dull and unique within their kind.
pub(super) fn check_ids(provides: &Provides) -> UiResult<()> {
    let kinds: [(&str, Vec<&str>); 9] = [
        ("theme", provides.themes.iter().map(|d| d.id.as_str()).collect()),
        ("layout", provides.layouts.iter().map(|d| d.id.as_str()).collect()),
        ("reader", provides.readers.iter().map(|d| d.id.as_str()).collect()),
        (
            "classification set",
            provides.taxonomies.iter().map(|d| d.id.as_str()).collect(),
        ),
        (
            "dictionary",
            provides.dictionaries.iter().map(|d| d.id.as_str()).collect(),
        ),
        ("writer", provides.writers.iter().map(|d| d.id.as_str()).collect()),
        ("widget", provides.widgets.iter().map(|d| d.id.as_str()).collect()),
        ("screen", provides.screens.iter().map(|d| d.id.as_str()).collect()),
        ("tool", provides.tools.iter().map(|d| d.id.as_str()).collect()),
    ];
    for (kind, ids) in kinds {
        for (i, id) in ids.iter().enumerate() {
            if !valid_id(id) {
                return Err(UiError::invalid(format!(
                    "{kind} id {id:?} is not lowercase letters, digits, dot, dash or underscore"
                )));
            }
            if ids[..i].contains(id) {
                return Err(UiError::invalid(format!("two of its {kind}s are called {id:?}")));
            }
        }
    }
    Ok(())
}
