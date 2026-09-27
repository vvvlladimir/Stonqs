//! Installed plugins, one folder each under `plugins/<id>/` beside the profiles, so a theme
//! survives switching profile (ADR-0070). Plain filesystem work over an explicitly passed root.

mod info;
mod install;
mod manifest;
mod offers;
pub mod reader;
pub mod reads;
mod sandbox;
pub mod tool;
mod verify;
pub mod widget;
pub mod writer;

use crate::error::{UiError, UiResult};
use std::path::PathBuf;

pub use info::{
    FileReading, LoadedTool, PageKind, PluginInfo, STATE_LIMIT, ScreenInfo, Status, TaxonomySetInfo,
    ThemeInfo, ToolInfo, Unlock, WidgetInfo, WriterInfo, tool_model_name,
};
pub use manifest::{
    API, Base, DictionaryDef, LayoutDef, Manifest, Provides, ReaderDef, TaxonomyDef, ThemeDef, WriterDef,
};
pub use tool::ToolDef;
pub use widget::{Read, ScreenDef, Size, WidgetDef};

use info::status_of;
#[cfg(test)]
use manifest::MANIFEST;
use manifest::{read_manifest, valid_id};

const FOLDER: &str = "plugins";

pub struct Plugins {
    root: PathBuf,
    /// Held by install and remove, which run off the main thread and must not interleave.
    writing: std::sync::Mutex<()>,
    /// What `list` last read, so one import sees one set of plugins from preview to commit.
    /// Dropped by whatever changes the folder, and by `refresh`.
    listed: std::sync::Mutex<Option<Vec<PluginInfo>>>,
}

fn io(e: std::io::Error) -> UiError {
    UiError::internal(e.to_string())
}

impl Plugins {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Plugins {
            root: root.into(),
            writing: std::sync::Mutex::new(()),
            listed: std::sync::Mutex::new(None),
        }
    }

    fn folder(&self) -> PathBuf {
        self.root.join(FOLDER)
    }

    fn folder_of(&self, id: &str) -> PathBuf {
        self.folder().join(id)
    }

    /// Everything installed, sorted by name, each with why it is or is not in use.
    pub fn list(&self) -> UiResult<Vec<PluginInfo>> {
        let mut listed = self.listed.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(plugins) = listed.as_ref() {
            return Ok(plugins.clone());
        }
        let plugins = self.scan()?;
        *listed = Some(plugins.clone());
        Ok(plugins)
    }

    /// The next `list` reads the folder again, so a package edited by hand is seen.
    pub fn refresh(&self) {
        *self.listed.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// The plugins this build can load: only they offer anything.
    fn usable(&self) -> UiResult<impl Iterator<Item = PluginInfo>> {
        Ok(self.list()?.into_iter().filter(|p| p.status == Status::Ok))
    }

    fn scan(&self) -> UiResult<Vec<PluginInfo>> {
        let folder = self.folder();
        if !folder.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&folder).map_err(io)? {
            let path = entry.map_err(io)?.path();
            let id = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            // A dot folder is an install in progress, or what an interrupted one left behind.
            if !path.is_dir() || id.starts_with('.') {
                continue;
            }
            out.push(match read_manifest(&path) {
                Ok(manifest) if manifest.id != id => PluginInfo::unusable(
                    &id,
                    manifest.name,
                    Status::Misplaced {
                        manifest_id: manifest.id,
                    },
                ),
                Ok(manifest) => PluginInfo::from_manifest(manifest),
                Err(detail) => PluginInfo::unusable(&id, id.clone(), Status::Broken { detail }),
            });
        }
        out.sort_by_key(|plugin| plugin.name.to_lowercase());
        Ok(out)
    }

    /// The one lookup by id: a dull id, a folder holding that id's manifest, this build's API.
    fn loaded(&self, plugin: &str) -> UiResult<(PathBuf, Manifest)> {
        if !valid_id(plugin) {
            return Err(UiError::not_found(format!("plugin {plugin}")));
        }
        let folder = self.folder_of(plugin);
        let manifest = read_manifest(&folder).map_err(UiError::not_found)?;
        if manifest.id != plugin {
            return Err(UiError::not_found(format!("plugin {plugin}")));
        }
        if status_of(&manifest) != Status::Ok {
            return Err(UiError::invalid(format!(
                "plugin {plugin} is built for plugin API {}",
                manifest.api
            )));
        }
        Ok((folder, manifest))
    }
}

#[cfg(test)]
mod tests;
