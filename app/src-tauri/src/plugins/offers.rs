//! What the installed plugins offer the rest of the app.

use super::info::{
    FileReading, LoadedTool, PageKind, ScreenInfo, TaxonomySetInfo, ThemeInfo, ToolInfo, Unlock, WidgetInfo,
    WriterInfo, tool_model_name,
};
use super::manifest::safe_join;
use super::{Plugins, io, reader, widget, writer};
use crate::error::{UiError, UiResult};
use serde::de::DeserializeOwned;
use sq_core::import::{BrokerPreset, KindWords};

/// A file a package names, parsed; `None` for one that no longer reads, which is skipped.
fn parse_json<T: DeserializeOwned>(folder: &std::path::Path, file: &str) -> Option<T> {
    let bytes = std::fs::read(safe_join(folder, file).ok()?).ok()?;
    serde_json::from_slice(&bytes).ok()
}

impl Plugins {
    pub fn themes(&self) -> UiResult<Vec<ThemeInfo>> {
        Ok(self
            .usable()?
            .flat_map(|p| {
                p.provides.themes.into_iter().map(move |theme| ThemeInfo {
                    key: format!("{}/{}", p.id, theme.id),
                    name: theme.name,
                    plugin: p.id.clone(),
                    base: theme.base,
                })
            })
            .collect())
    }

    /// Read on demand: only one theme is ever applied.
    pub fn theme_css(&self, plugin: &str, theme: &str) -> UiResult<String> {
        let (folder, manifest) = self.loaded(plugin)?;
        let def = manifest
            .provides
            .themes
            .into_iter()
            .find(|t| t.id == theme)
            .ok_or_else(|| UiError::not_found(format!("theme {plugin}/{theme}")))?;
        std::fs::read_to_string(safe_join(&folder, &def.file)?).map_err(io)
    }

    /// Every layout, keyed `<plugin>/<layout>`; one that no longer parses is skipped.
    pub fn layouts(&self) -> UiResult<Vec<(String, BrokerPreset)>> {
        let mut out = Vec::new();
        for plugin in self.usable()? {
            let folder = self.folder_of(&plugin.id);
            for layout in plugin.provides.layouts {
                if let Some(preset) = parse_json::<BrokerPreset>(&folder, &layout.file) {
                    out.push((format!("{}/{}", plugin.id, layout.id), preset));
                }
            }
        }
        Ok(out)
    }

    /// Every plugin's words as one dictionary, in the list's order.
    pub fn kind_words(&self) -> UiResult<KindWords> {
        let mut words = KindWords::empty();
        for plugin in self.usable()? {
            let folder = self.folder_of(&plugin.id);
            for def in plugin.provides.dictionaries {
                if let Some(parsed) = parse_json::<KindWords>(&folder, &def.file) {
                    words.extend(parsed);
                }
            }
        }
        Ok(words)
    }

    /// The first reader offered this ending that claims the file (ADR-0086, ADR-0087).
    /// `NotMine` and a broken module move on; `malformed` and `needs-password` stop the import.
    pub fn read_file(&self, name: &str, bytes: &[u8], unlock: Option<&Unlock>) -> UiResult<FileReading> {
        let ending = name
            .rsplit_once('.')
            .map(|(_, end)| format!(".{}", end.to_lowercase()));
        let mut skipped = Vec::new();
        for plugin in self.usable()? {
            let folder = self.folder_of(&plugin.id);
            for def in plugin.provides.readers {
                let offered = def.extensions.is_empty()
                    || ending
                        .as_deref()
                        .is_some_and(|end| def.extensions.iter().any(|x| x.eq_ignore_ascii_case(end)));
                let Ok(module) = safe_join(&folder, &def.file) else {
                    continue;
                };
                if !offered || !module.exists() {
                    continue;
                }
                let id = format!("{}/{}", plugin.id, def.id);
                let password = unlock.filter(|u| u.reader == id).map(|u| u.password.as_str());
                match reader::read(&module, bytes, name, password) {
                    Ok(mut reading) => {
                        for warning in &mut reading.warnings {
                            warning.plugin = id.clone();
                        }
                        return Ok(FileReading {
                            read: Some((id, reading)),
                            skipped,
                        });
                    }
                    Err(reader::Refusal::NotMine) => continue,
                    Err(reader::Refusal::Broken(detail)) => {
                        skipped.push(reader::SkippedReader { plugin: id, detail })
                    }
                    Err(reader::Refusal::NeedsPassword) => {
                        return Err(UiError::FileProtected {
                            message: format!("reader {id} needs the file's password"),
                            tried: password.is_some(),
                            reader: id,
                        });
                    }
                    Err(refusal) => return Err(refusal.into_error(&id)),
                }
            }
        }
        Ok(FileReading { read: None, skipped })
    }

    pub fn writers(&self) -> UiResult<Vec<WriterInfo>> {
        Ok(self
            .usable()?
            .flat_map(|p| {
                p.provides.writers.into_iter().map(move |def| WriterInfo {
                    key: format!("{}/{}", p.id, def.id),
                    name: def.name,
                    plugin: p.id.clone(),
                    extension: def.extension,
                })
            })
            .collect())
    }

    /// Writes a transaction document through the writer `<plugin id>/<writer id>`.
    pub fn write(&self, key: &str, canonical: &str) -> UiResult<Vec<u8>> {
        let (plugin, writer) = key
            .split_once('/')
            .ok_or_else(|| UiError::not_found(format!("writer {key}")))?;
        let (folder, manifest) = self.loaded(plugin)?;
        let def = manifest
            .provides
            .writers
            .into_iter()
            .find(|w| w.id == writer)
            .ok_or_else(|| UiError::not_found(format!("writer {key}")))?;
        writer::write(&safe_join(&folder, &def.file)?, canonical).map_err(|message| UiError::Writer {
            plugin: key.to_string(),
            message,
        })
    }

    pub fn widgets(&self) -> UiResult<Vec<WidgetInfo>> {
        Ok(self
            .usable()?
            .flat_map(|p| {
                p.provides.widgets.into_iter().map(move |def| WidgetInfo {
                    key: format!("{}/{}", p.id, def.id),
                    name: def.name,
                    description: def.description,
                    plugin: p.id.clone(),
                    plugin_name: p.name.clone(),
                    reads: def.reads,
                    periodic: def.periodic,
                    size: def.size,
                    min: def.min,
                })
            })
            .collect())
    }

    pub fn screens(&self) -> UiResult<Vec<ScreenInfo>> {
        Ok(self
            .usable()?
            .flat_map(|p| {
                p.provides.screens.into_iter().map(move |def| ScreenInfo {
                    key: format!("{}/{}", p.id, def.id),
                    name: def.name,
                    description: def.description,
                    plugin: p.id.clone(),
                    plugin_name: p.name.clone(),
                    reads: def.reads,
                    periodic: def.periodic,
                    storage: def.storage,
                })
            })
            .collect())
    }

    /// A page and its CSP, as the `stonqs-plugin` scheme answers `/<widget|screen>/<plugin>/<id>`.
    pub fn page(&self, kind: PageKind, plugin: &str, id: &str) -> UiResult<(String, String)> {
        let (folder, manifest) = self.loaded(plugin)?;
        let file = match kind {
            PageKind::Widget => manifest
                .provides
                .widgets
                .into_iter()
                .find(|w| w.id == id)
                .map(|w| w.file),
            PageKind::Screen => manifest
                .provides
                .screens
                .into_iter()
                .find(|w| w.id == id)
                .map(|w| w.file),
        }
        .ok_or_else(|| UiError::not_found(format!("{kind:?} {plugin}/{id}")))?;
        let module = std::fs::read_to_string(safe_join(&folder, &file)?).map_err(io)?;
        Ok(widget::page(&module, &uuid::Uuid::new_v4().simple().to_string()))
    }

    /// Only a plugin whose manifest declared `storage` on a screen may keep a document.
    pub fn keeps_state(&self, plugin: &str) -> UiResult<bool> {
        let (_, manifest) = self.loaded(plugin)?;
        Ok(manifest.provides.screens.iter().any(|s| s.storage))
    }

    /// Tools for one message. One whose files no longer read is left out; a taken name keeps its
    /// first owner.
    pub fn tools(&self) -> UiResult<Vec<LoadedTool>> {
        let mut out: Vec<LoadedTool> = Vec::new();
        for plugin in self.usable()? {
            let folder = self.folder_of(&plugin.id);
            for def in plugin.provides.tools {
                let model_name = tool_model_name(&plugin.id, &def.id);
                let Ok(module) = safe_join(&folder, &def.file) else {
                    continue;
                };
                let Some(schema) = parse_json(&folder, &def.schema) else {
                    continue;
                };
                if out.iter().any(|t| t.model_name == model_name) {
                    continue;
                }
                out.push(LoadedTool {
                    periodic: def.periodic(),
                    info: ToolInfo {
                        key: format!("{}/{}", plugin.id, def.id),
                        name: def.name,
                        plugin: plugin.id.clone(),
                        plugin_name: plugin.name.clone(),
                        reads: def.reads,
                    },
                    model_name,
                    description: def.description,
                    schema,
                    module,
                });
            }
        }
        Ok(out)
    }

    pub fn taxonomy_sets(&self) -> UiResult<Vec<TaxonomySetInfo>> {
        Ok(self
            .usable()?
            .flat_map(|p| {
                p.provides.taxonomies.into_iter().map(move |set| TaxonomySetInfo {
                    key: format!("{}/{}", p.id, set.id),
                    name: set.name,
                    plugin: p.id.clone(),
                })
            })
            .collect())
    }

    /// A set's CSV, fed to the same preview as any taxonomy file.
    pub fn taxonomy_csv(&self, plugin: &str, set: &str) -> UiResult<Vec<u8>> {
        let (folder, manifest) = self.loaded(plugin)?;
        let def = manifest
            .provides
            .taxonomies
            .into_iter()
            .find(|t| t.id == set)
            .ok_or_else(|| UiError::not_found(format!("classification set {plugin}/{set}")))?;
        std::fs::read(safe_join(&folder, &def.file)?).map_err(io)
    }
}
