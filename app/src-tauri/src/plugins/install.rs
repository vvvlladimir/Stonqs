//! Installing and removing packages. Both run under `Plugins::writing`.

use super::info::{PluginInfo, tool_model_name};
use super::manifest::{MANIFEST, Manifest, Provides, check_ids, read_manifest, safe_join, valid_id};
use super::{Plugins, io, sandbox, verify};
use crate::error::{UiError, UiResult};
use std::path::Path;

impl Plugins {
    /// Installs the folder the user picked, copying only the manifest and the files it names.
    pub fn install(&self, source: &Path) -> UiResult<PluginInfo> {
        let _writing = self.writing.lock().unwrap_or_else(|e| e.into_inner());
        let installed = self.install_locked(source);
        self.refresh();
        installed
    }

    fn install_locked(&self, source: &Path) -> UiResult<PluginInfo> {
        let manifest = read_manifest(source).map_err(UiError::invalid)?;
        if !valid_id(&manifest.id) {
            return Err(UiError::invalid(format!(
                "plugin id {:?} is not lowercase letters, digits, dot, dash or underscore",
                manifest.id
            )));
        }
        // Far more often a misspelled `provides` than a package for a later build.
        if manifest.provides.is_empty() {
            return Err(UiError::invalid(format!(
                "plugin {} declares nothing this build can use: expected `provides.themes`, \
                 `provides.layouts`, `provides.readers`, `provides.writers`, \
                 `provides.widgets`, `provides.screens`, `provides.tools`, \
                 `provides.taxonomies` or `provides.dictionaries`",
                manifest.id
            )));
        }
        check_ids(&manifest.provides)?;
        verify::regular_files(source, &manifest.provides)?;
        self.check_tool_names(&manifest)?;
        verify::content(source, &manifest.provides)?;

        let target = self.swap_in(source, &manifest)?;
        sandbox::forget(&target);
        Ok(PluginInfo::from_manifest(manifest))
    }

    /// Copies into a staging folder and renames it over the installed one, so a copy that fails
    /// half way leaves the old version as it was.
    fn swap_in(&self, source: &Path, manifest: &Manifest) -> UiResult<std::path::PathBuf> {
        let folder = self.folder();
        clear_leftovers(&folder);
        let staging = folder.join(format!(".staging-{}", uuid::Uuid::new_v4().simple()));
        if let Err(e) = copy_package(source, &staging, &manifest.provides) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(e);
        }
        let target = self.folder_of(&manifest.id);
        let old = folder.join(format!(".old-{}", uuid::Uuid::new_v4().simple()));
        let replacing = target.exists();
        if replacing && let Err(e) = std::fs::rename(&target, &old) {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(io(e));
        }
        if let Err(e) = std::fs::rename(&staging, &target) {
            if replacing {
                let _ = std::fs::rename(&old, &target);
            }
            let _ = std::fs::remove_dir_all(&staging);
            return Err(io(e));
        }
        if replacing {
            // Out of the list's sight already; one that cannot go now goes at the next install.
            let _ = std::fs::remove_dir_all(&old);
        }
        Ok(target)
    }

    /// `.` and `-` both become `_` in a model name, so two packages' tools can collide; the loser
    /// would vanish and a session grant under that name would pass to the winner.
    fn check_tool_names(&self, manifest: &Manifest) -> UiResult<()> {
        let mut taken: Vec<(String, String)> = Vec::new();
        for plugin in self.list()?.into_iter().filter(|p| p.id != manifest.id) {
            for def in &plugin.provides.tools {
                taken.push((tool_model_name(&plugin.id, &def.id), plugin.id.clone()));
            }
        }
        for def in &manifest.provides.tools {
            let name = tool_model_name(&manifest.id, &def.id);
            if name.len() > 64 {
                return Err(UiError::invalid(format!(
                    "tool {}: {name} is longer than the 64 characters a provider accepts",
                    def.id
                )));
            }
            if let Some((_, owner)) = taken.iter().find(|(other, _)| *other == name) {
                let whose = if *owner == manifest.id {
                    "another tool of this package".to_string()
                } else {
                    format!("a tool of plugin {owner}")
                };
                return Err(UiError::invalid(format!(
                    "tool {}: the assistant would call it {name}, which is already {whose}",
                    def.id
                )));
            }
            taken.push((name, manifest.id.clone()));
        }
        Ok(())
    }

    /// Removes a plugin's folder. What it stored in the profile stays.
    pub fn remove(&self, id: &str) -> UiResult<()> {
        let _writing = self.writing.lock().unwrap_or_else(|e| e.into_inner());
        if !valid_id(id) {
            return Err(UiError::not_found(format!("plugin {id}")));
        }
        let folder = self.folder_of(id);
        if !folder.exists() {
            return Err(UiError::not_found(format!("plugin {id}")));
        }
        let removed = std::fs::remove_dir_all(&folder).map_err(io);
        self.refresh();
        sandbox::forget(&folder);
        removed
    }
}

fn copy_package(source: &Path, to: &Path, provides: &Provides) -> UiResult<()> {
    std::fs::create_dir_all(to).map_err(io)?;
    std::fs::copy(source.join(MANIFEST), to.join(MANIFEST)).map_err(io)?;
    for file in provides.files() {
        let from = safe_join(source, file)?;
        let dest = safe_join(to, file)?;
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent).map_err(io)?;
        }
        std::fs::copy(&from, &dest)
            .map_err(|e| UiError::invalid(format!("{file} is named by the manifest and missing: {e}")))?;
    }
    Ok(())
}

/// What an interrupted install left behind. Called under `writing`, so none is still in use.
fn clear_leftovers(folder: &Path) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(".staging-") || name.starts_with(".old-") {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}
