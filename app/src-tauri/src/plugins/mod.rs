//! Plugins: what the app can be extended with, one folder each under `plugins/<id>/` in the app's
//! data directory. See ADR-0070.
//!
//! Beside the profiles rather than inside one: a profile is everything a *portfolio* owns
//! (ADR-0047), and a theme that vanished when the user switched profile would be a bug nobody
//! could explain. What belongs to a profile is what a plugin *stores*, which no theme does.
//!
//! Plain filesystem work over an explicitly passed root, so it is tested on a temporary folder.
//! This build honours data content — themes, broker layouts, classification sets and operation
//! dictionaries — plus two kinds of compute over one sandbox, the file reader (ADR-0073) and the
//! file writer (ADR-0080), and two kinds of UI, the dashboard widget (ADR-0083) and the screen
//! (ADR-0084); the rest of a
//! manifest is read without being acted on, so a package built for a later version is listed
//! rather than rejected.

pub mod reader;
mod sandbox;
pub mod tool;
pub mod widget;
pub mod writer;

use crate::error::{UiError, UiResult};
use serde::{Deserialize, Serialize};
use sq_core::import::{BrokerPreset, KindWords};
use std::path::{Path, PathBuf};
pub use tool::ToolDef;
pub use widget::{Read, ScreenDef, Size, WidgetDef};

const FOLDER: &str = "plugins";
const MANIFEST: &str = "plugin.json";

/// The plugin API this build speaks. A package naming another is listed and not loaded: half a
/// plugin is worse than none, and silence would look like the app itself failing.
pub const API: u32 = 1;

/// An id is a folder name on three platforms and a key in the vault, so it is deliberately dull.
fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'))
}

/// What a package says it is. Unknown fields are kept out of the way rather than refused: the
/// manifest is one format for three kinds of content, and this build reads the first kind.
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

/// A file writer: a WASM component that turns the app's own transaction file into another format
/// (ADR-0080). It ships a sample document and the exact bytes that sample must become, for the
/// reason a reader ships its expectation — a wrong file looks like a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriterDef {
    pub id: String,
    /// What the export offers it as. The plugin's own words, like a classification set's name.
    pub name: String,
    /// The component, as a `.wasm` file inside the package.
    pub file: String,
    /// A `stonqs.transactions` document.
    pub sample: String,
    /// What `sample` must be written as, byte for byte.
    pub expected: String,
    /// The ending a saved file gets, without the dot (`journal`).
    pub extension: String,
}

/// Operation wordings for a language the shipped keywords do not speak: per language, never per
/// broker, like the shipped table. Read after it and never instead of it (`KindWords`), and fed to
/// the preview rather than into a layout, so removing the plugin removes the words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictionaryDef {
    pub id: String,
    /// `{ "words": [{ "word", "kind" }] }`, in order — the first word a wording contains wins.
    pub file: String,
    /// A file whose every wording the dictionary answers and the app alone does not.
    pub sample: String,
}

/// A ready classification tree: the same CSV the taxonomy import reads, so the file *is* the
/// data and there is nothing for a separate expectation to state. It goes through
/// `taxonomy_import_preview` like any other, which is why installing one adds no second path
/// into the portfolio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonomyDef {
    pub id: String,
    /// What the tree is called when it is created. The user renames it freely afterwards — a
    /// classification's name is their data, never a label the app translates.
    pub name: String,
    pub file: String,
}

/// A file reader: a WASM component that turns bytes this app cannot read into the app's own
/// transaction file (ADR-0073). It ships the sample it was written against **and** what that
/// sample must come out as, because a layout that misreads a column shows up in the wizard while
/// a reader that misreads one produces a document that looks perfectly correct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReaderDef {
    pub id: String,
    /// The component, as a `.wasm` file inside the package.
    pub file: String,
    /// A redacted file of the kind this reader exists for.
    pub sample: String,
    /// What `sample` must read as: a `stonqs.transactions` document.
    pub expected: String,
    /// The file endings this reader is offered, lower case and with the dot (`.pdf`). Empty means
    /// every file that the shipped readers did not already recognise — honest for a reader of a
    /// format with no ending of its own, and expensive enough that a reader should say.
    #[serde(default)]
    pub extensions: Vec<String>,
}

/// A broker layout: the same JSON a shipped preset is written in, plus the sample it was made
/// from. The sample is **required** — a layout nobody tried against a real export is exactly what
/// the shipped ones were before they had fixtures, and a stranger's untried layout is worse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutDef {
    pub id: String,
    /// The layout file, holding one preset in `presets/brokers.json` shape.
    pub file: String,
    /// A redacted export this layout must recognise and read without a question.
    pub sample: String,
}

/// A theme is a stylesheet that redefines the app's own custom properties, loaded only while it
/// is the chosen one — so it says `:root` and needs to know no selector of ours. `base` is the
/// built-in scheme it is a variation of: what it does not redefine comes from there.
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
}

/// One installed plugin as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
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
    #[serde(flatten)]
    pub status: Status,
}

/// One classification set on offer, addressed the way a command names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaxonomySetInfo {
    /// `<plugin id>/<set id>`.
    pub key: String,
    pub name: String,
    pub plugin: String,
}

/// One export format on offer, addressed the way a command names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WriterInfo {
    /// `<plugin id>/<writer id>`.
    pub key: String,
    pub name: String,
    pub plugin: String,
    pub extension: String,
}

/// One assistant tool on offer, as the plugin list shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolInfo {
    /// `<plugin id>/<tool id>`.
    pub key: String,
    pub name: String,
    pub plugin: String,
    pub plugin_name: String,
    pub reads: Vec<Read>,
}

/// A tool ready to be offered to the model: what it is called there, what it takes, and the
/// module that answers. Built fresh for each message, so an install mid-chat is seen by the next.
#[derive(Debug, Clone)]
pub struct LoadedTool {
    pub info: ToolInfo,
    /// The name the model calls it by (`plugin_<plugin>_<tool>`), unique in the catalogue.
    pub model_name: String,
    pub description: String,
    pub schema: serde_json::Value,
    pub periodic: bool,
    pub module: PathBuf,
}

/// What the model calls a plugin's tool. Providers accept `[a-zA-Z0-9_-]{1,64}`, and the prefix
/// keeps it apart from the app's own catalogue, whose names never start with it.
pub fn tool_model_name(plugin: &str, tool: &str) -> String {
    let clean = |s: &str| s.replace(['.', '-'], "_");
    format!("plugin_{}_{}", clean(plugin), clean(tool))
}

/// One screen on offer, addressed the way the navigation hint names it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScreenInfo {
    /// `<plugin id>/<screen id>`.
    pub key: String,
    pub name: String,
    pub description: String,
    pub plugin: String,
    /// The plugin's own name, which the screen's header always shows (ADR-0082).
    pub plugin_name: String,
    pub reads: Vec<Read>,
    pub periodic: bool,
    pub storage: bool,
}

/// Which kind of page the `stonqs-plugin` scheme is asked for: the first segment of its path.
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

/// One dashboard widget on offer, addressed the way a board stores its type after `plugin:`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WidgetInfo {
    /// `<plugin id>/<widget id>`.
    pub key: String,
    pub name: String,
    pub description: String,
    pub plugin: String,
    /// The plugin's own name: every tile it draws carries it (ADR-0082).
    pub plugin_name: String,
    pub reads: Vec<Read>,
    pub periodic: bool,
    pub size: Size,
    pub min: Size,
}

/// One installed theme, addressed the way the stored preference addresses it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThemeInfo {
    /// `<plugin id>/<theme id>` — what `UiState::theme` holds after `plugin:`.
    pub key: String,
    pub name: String,
    pub plugin: String,
    pub base: Base,
}

pub struct Plugins {
    root: PathBuf,
}

fn io(e: std::io::Error) -> UiError {
    UiError::internal(e.to_string())
}

impl Plugins {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Plugins { root: root.into() }
    }

    fn folder(&self) -> PathBuf {
        self.root.join(FOLDER)
    }

    fn folder_of(&self, id: &str) -> PathBuf {
        self.folder().join(id)
    }

    /// Everything installed, in a stable order, each with why it is or is not in use.
    pub fn list(&self) -> UiResult<Vec<PluginInfo>> {
        let folder = self.folder();
        if !folder.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for entry in std::fs::read_dir(&folder).map_err(io)? {
            let path = entry.map_err(io)?.path();
            if !path.is_dir() {
                continue;
            }
            let id = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.push(match read_manifest(&path) {
                Ok(manifest) => PluginInfo {
                    status: status_of(&manifest),
                    id: manifest.id,
                    name: manifest.name,
                    version: manifest.version,
                    themes: manifest.provides.themes,
                    layouts: manifest.provides.layouts,
                    readers: manifest.provides.readers,
                    taxonomies: manifest.provides.taxonomies,
                    dictionaries: manifest.provides.dictionaries,
                    writers: manifest.provides.writers,
                    widgets: manifest.provides.widgets,
                    screens: manifest.provides.screens,
                    tools: manifest.provides.tools,
                },
                Err(detail) => PluginInfo {
                    id: id.clone(),
                    name: id,
                    version: String::new(),
                    themes: Vec::new(),
                    layouts: Vec::new(),
                    readers: Vec::new(),
                    taxonomies: Vec::new(),
                    dictionaries: Vec::new(),
                    writers: Vec::new(),
                    widgets: Vec::new(),
                    screens: Vec::new(),
                    tools: Vec::new(),
                    status: Status::Broken { detail },
                },
            });
        }
        out.sort_by_key(|plugin| plugin.name.to_lowercase());
        Ok(out)
    }

    /// The themes on offer: only from plugins this build can load, because a theme that cannot
    /// be applied has no business in a picker.
    pub fn themes(&self) -> UiResult<Vec<ThemeInfo>> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|p| p.status == Status::Ok)
            .flat_map(|p| {
                p.themes.into_iter().map(move |theme| ThemeInfo {
                    key: format!("{}/{}", p.id, theme.id),
                    name: theme.name,
                    plugin: p.id.clone(),
                    base: theme.base,
                })
            })
            .collect())
    }

    /// Every layout a loadable plugin brings, parsed, each with the plugin it came from. A
    /// layout that no longer parses is skipped rather than failing the list: the wizard's other
    /// layouts are not this one's business.
    pub fn layouts(&self) -> UiResult<Vec<(String, BrokerPreset)>> {
        let mut out = Vec::new();
        for plugin in self.list()?.into_iter().filter(|p| p.status == Status::Ok) {
            let folder = self.folder_of(&plugin.id);
            for layout in plugin.layouts {
                let Ok(path) = safe_join(&folder, &layout.file) else {
                    continue;
                };
                let parsed = std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| serde_json::from_str::<BrokerPreset>(&text).ok());
                if let Some(preset) = parsed {
                    out.push((format!("{}/{}", plugin.id, layout.id), preset));
                }
            }
        }
        Ok(out)
    }

    /// Every loadable plugin's words as one dictionary, in the list's order. A file that no longer
    /// parses is skipped, like a layout: the other plugins' words are not its business.
    pub fn kind_words(&self) -> UiResult<KindWords> {
        let mut words = KindWords::empty();
        for plugin in self.list()?.into_iter().filter(|p| p.status == Status::Ok) {
            let folder = self.folder_of(&plugin.id);
            for def in plugin.dictionaries {
                let Ok(path) = safe_join(&folder, &def.file) else {
                    continue;
                };
                let parsed = std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|text| serde_json::from_str::<KindWords>(&text).ok());
                if let Some(parsed) = parsed {
                    words.extend(parsed);
                }
            }
        }
        Ok(words)
    }

    /// The first loadable reader that claims this file, and what it read.
    ///
    /// Narrowed by the file's ending before anything is run: nothing hands twenty megabytes to
    /// every installed plugin in turn. `not-mine` moves on to the next; a reader that claimed the
    /// file and then failed is an error rather than a fall-through, because the reader after it
    /// would be reading a file somebody has already said is not theirs.
    pub fn read_file(&self, name: &str, bytes: &[u8]) -> UiResult<Option<(String, reader::Reading)>> {
        let ending = name
            .rsplit_once('.')
            .map(|(_, end)| format!(".{}", end.to_lowercase()));
        for plugin in self.list()?.into_iter().filter(|p| p.status == Status::Ok) {
            let folder = self.folder_of(&plugin.id);
            for def in plugin.readers {
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
                match reader::read(&module, bytes, name, None) {
                    Ok(mut reading) => {
                        for warning in &mut reading.warnings {
                            warning.plugin = id.clone();
                        }
                        return Ok(Some((id, reading)));
                    }
                    Err(reader::Refusal::NotMine) => continue,
                    Err(refusal) => return Err(refusal.into_error(&id)),
                }
            }
        }
        Ok(None)
    }

    /// The export formats on offer, from plugins this build can load.
    pub fn writers(&self) -> UiResult<Vec<WriterInfo>> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|p| p.status == Status::Ok)
            .flat_map(|p| {
                p.writers.into_iter().map(move |def| WriterInfo {
                    key: format!("{}/{}", p.id, def.id),
                    name: def.name,
                    plugin: p.id.clone(),
                    extension: def.extension,
                })
            })
            .collect())
    }

    /// Writes a transaction document through one installed writer, named `<plugin id>/<writer id>`.
    pub fn write(&self, key: &str, canonical: &str) -> UiResult<Vec<u8>> {
        let (plugin, writer) = key
            .split_once('/')
            .filter(|(plugin, _)| valid_id(plugin))
            .ok_or_else(|| UiError::not_found(format!("writer {key}")))?;
        let folder = self.folder_of(plugin);
        let manifest = read_manifest(&folder).map_err(UiError::not_found)?;
        if status_of(&manifest) != Status::Ok {
            return Err(UiError::invalid(format!(
                "plugin {plugin} is built for plugin API {}",
                manifest.api
            )));
        }
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

    /// The dashboard widgets on offer, from plugins this build can load.
    pub fn widgets(&self) -> UiResult<Vec<WidgetInfo>> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|p| p.status == Status::Ok)
            .flat_map(|p| {
                p.widgets.into_iter().map(move |def| WidgetInfo {
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

    /// The assistant tools on offer, loaded for a message. A tool whose files no longer read is
    /// left out rather than failing the chat, and a name already taken keeps its first owner.
    pub fn tools(&self) -> UiResult<Vec<LoadedTool>> {
        let mut out: Vec<LoadedTool> = Vec::new();
        for plugin in self.list()?.into_iter().filter(|p| p.status == Status::Ok) {
            let folder = self.folder_of(&plugin.id);
            for def in plugin.tools {
                let model_name = tool_model_name(&plugin.id, &def.id);
                let (Ok(module), Ok(schema)) =
                    (safe_join(&folder, &def.file), safe_join(&folder, &def.schema))
                else {
                    continue;
                };
                let Some(schema) = std::fs::read(schema)
                    .ok()
                    .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                else {
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

    /// The screens on offer, from plugins this build can load, in the list's order.
    pub fn screens(&self) -> UiResult<Vec<ScreenInfo>> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|p| p.status == Status::Ok)
            .flat_map(|p| {
                p.screens.into_iter().map(move |def| ScreenInfo {
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

    /// A page and the policy it must be served under, as the `stonqs-plugin` scheme answers
    /// `/<widget|screen>/<plugin id>/<id>`.
    pub fn page(&self, kind: PageKind, plugin: &str, id: &str) -> UiResult<(String, String)> {
        let folder = self.loadable(plugin)?;
        let manifest = read_manifest(&folder).map_err(UiError::not_found)?;
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

    /// Whether a loadable plugin declared a screen that keeps a document: the state commands
    /// answer nobody else, so a package cannot store what its manifest never admitted to.
    pub fn keeps_state(&self, plugin: &str) -> UiResult<bool> {
        let folder = self.loadable(plugin)?;
        let manifest = read_manifest(&folder).map_err(UiError::not_found)?;
        Ok(manifest.provides.screens.iter().any(|s| s.storage))
    }

    /// The folder of a plugin this build can load, or why not.
    fn loadable(&self, plugin: &str) -> UiResult<PathBuf> {
        if !valid_id(plugin) {
            return Err(UiError::not_found(format!("plugin {plugin}")));
        }
        let folder = self.folder_of(plugin);
        let manifest = read_manifest(&folder).map_err(UiError::not_found)?;
        if status_of(&manifest) != Status::Ok {
            return Err(UiError::invalid(format!(
                "plugin {plugin} is built for plugin API {}",
                manifest.api
            )));
        }
        Ok(folder)
    }

    /// A theme's stylesheet. Read on demand rather than at startup: only one is ever applied.
    pub fn theme_css(&self, plugin: &str, theme: &str) -> UiResult<String> {
        if !valid_id(plugin) {
            return Err(UiError::not_found(format!("plugin {plugin}")));
        }
        let folder = self.folder_of(plugin);
        let manifest = read_manifest(&folder).map_err(UiError::not_found)?;
        if status_of(&manifest) != Status::Ok {
            return Err(UiError::invalid(format!(
                "plugin {plugin} is built for plugin API {}",
                manifest.api
            )));
        }
        let def = manifest
            .provides
            .themes
            .into_iter()
            .find(|t| t.id == theme)
            .ok_or_else(|| UiError::not_found(format!("theme {plugin}/{theme}")))?;
        std::fs::read_to_string(safe_join(&folder, &def.file)?).map_err(io)
    }

    /// The classification sets on offer, from plugins this build can load.
    pub fn taxonomy_sets(&self) -> UiResult<Vec<TaxonomySetInfo>> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|p| p.status == Status::Ok)
            .flat_map(|p| {
                p.taxonomies.into_iter().map(move |set| TaxonomySetInfo {
                    key: format!("{}/{}", p.id, set.id),
                    name: set.name,
                    plugin: p.id.clone(),
                })
            })
            .collect())
    }

    /// A set's CSV, read on demand. It is handed to the same preview every taxonomy file goes
    /// through, so a set from a plugin has no path of its own into the portfolio.
    pub fn taxonomy_csv(&self, plugin: &str, set: &str) -> UiResult<Vec<u8>> {
        if !valid_id(plugin) {
            return Err(UiError::not_found(format!("plugin {plugin}")));
        }
        let folder = self.folder_of(plugin);
        let manifest = read_manifest(&folder).map_err(UiError::not_found)?;
        if status_of(&manifest) != Status::Ok {
            return Err(UiError::invalid(format!(
                "plugin {plugin} is built for plugin API {}",
                manifest.api
            )));
        }
        let def = manifest
            .provides
            .taxonomies
            .into_iter()
            .find(|t| t.id == set)
            .ok_or_else(|| UiError::not_found(format!("classification set {plugin}/{set}")))?;
        std::fs::read(safe_join(&folder, &def.file)?).map_err(io)
    }

    /// Installs the folder the user picked. Only the manifest and the files it names are copied:
    /// a package is what it declares, and whatever else sits beside it is not ours to carry in.
    pub fn install(&self, source: &Path) -> UiResult<PluginInfo> {
        let manifest = read_manifest(source).map_err(UiError::invalid)?;
        if !valid_id(&manifest.id) {
            return Err(UiError::invalid(format!(
                "plugin id {:?} is not lowercase letters, digits, dot, dash or underscore",
                manifest.id
            )));
        }
        // A package that brings nothing this build can use is a typo far more often than it is a
        // package for a later version — `provides` misspelled, or a content kind this build does
        // not know. A missing required field already fails above; an optional one would otherwise
        // install in silence and leave the user looking for a theme that was never declared.
        if manifest.provides.themes.is_empty()
            && manifest.provides.layouts.is_empty()
            && manifest.provides.readers.is_empty()
            && manifest.provides.taxonomies.is_empty()
            && manifest.provides.dictionaries.is_empty()
            && manifest.provides.writers.is_empty()
            && manifest.provides.widgets.is_empty()
            && manifest.provides.screens.is_empty()
            && manifest.provides.tools.is_empty()
        {
            return Err(UiError::invalid(format!(
                "plugin {} declares nothing this build can use: expected `provides.themes`, \
                 `provides.layouts`, `provides.readers`, `provides.writers`, \
                 `provides.widgets`, `provides.screens`, `provides.tools`, \
                 `provides.taxonomies` or `provides.dictionaries`",
                manifest.id
            )));
        }

        // A layout proves itself before it is installed, against the sample the package carries.
        // The shipped layouts answer to the same check in `core/tests/fixtures/presets/`; this is
        // that promise applied to a layout nobody in this repository has seen.
        for layout in &manifest.provides.layouts {
            let preset = std::fs::read_to_string(safe_join(source, &layout.file)?)
                .map_err(|e| UiError::invalid(format!("{}: {e}", layout.file)))?;
            let sample = std::fs::read(safe_join(source, &layout.sample)?)
                .map_err(|e| UiError::invalid(format!("{}: {e}", layout.sample)))?;
            crate::import_templates::check_layout(&layout.id, &preset, &sample, &layout.sample)?;
        }

        // And a reader proves itself the same way, against a stronger expectation: what its own
        // sample must come out as. A reader nobody can check is a reader nobody can trust with
        // somebody's ledger.
        for def in &manifest.provides.readers {
            let sample = std::fs::read(safe_join(source, &def.sample)?)
                .map_err(|e| UiError::invalid(format!("{}: {e}", def.sample)))?;
            let expected = std::fs::read_to_string(safe_join(source, &def.expected)?)
                .map_err(|e| UiError::invalid(format!("{}: {e}", def.expected)))?;
            reader::check(
                &def.id,
                &safe_join(source, &def.file)?,
                &sample,
                &def.sample,
                &expected,
            )?;
        }

        // A writer proves itself the way a reader does, turned round: its sample written must be
        // exactly the bytes the package says it becomes.
        for def in &manifest.provides.writers {
            // It becomes a save dialog's filter, so it is an ending and nothing else.
            if def.extension.is_empty()
                || def.extension.len() > 12
                || !def.extension.chars().all(|c| c.is_ascii_alphanumeric())
            {
                return Err(UiError::invalid(format!(
                    "writer {}: extension {:?} is not letters and digits without the dot",
                    def.id, def.extension
                )));
            }
            let sample = std::fs::read(safe_join(source, &def.sample)?)
                .map_err(|e| UiError::invalid(format!("{}: {e}", def.sample)))?;
            let expected = std::fs::read(safe_join(source, &def.expected)?)
                .map_err(|e| UiError::invalid(format!("{}: {e}", def.expected)))?;
            writer::check(&def.id, &safe_join(source, &def.file)?, &sample, &expected)?;
        }

        // A widget has no answer to be compared with — it draws — so what is checked is its shape.
        for def in &manifest.provides.widgets {
            let module = std::fs::read(safe_join(source, &def.file)?)
                .map_err(|e| UiError::invalid(format!("{}: {e}", def.file)))?;
            widget::check(def, &module)?;
        }
        for def in &manifest.provides.screens {
            let module = std::fs::read(safe_join(source, &def.file)?)
                .map_err(|e| UiError::invalid(format!("{}: {e}", def.file)))?;
            widget::check_screen(def, &module)?;
        }

        // An assistant tool proves itself the way a reader does: its own sample, answered exactly.
        // Its schema is checked too, because a schema a provider refuses breaks every chat, not
        // just this tool.
        for def in &manifest.provides.tools {
            let name = tool_model_name(&manifest.id, &def.id);
            if name.len() > 64 {
                return Err(UiError::invalid(format!(
                    "tool {}: {name} is longer than the 64 characters a provider accepts",
                    def.id
                )));
            }
            let read = |file: &str| {
                std::fs::read(safe_join(source, file)?).map_err(|e| UiError::invalid(format!("{file}: {e}")))
            };
            tool::check(
                def,
                &safe_join(source, &def.file)?,
                &read(&def.schema)?,
                &read(&def.sample)?,
                &read(&def.expected)?,
            )?;
        }

        // A classification set proves itself the same way, and needs no expectation of its own:
        // the file *is* the data, so an expectation would be a copy of it. What it must show is
        // that it reads as a tree at all and leaves nothing invalid behind.
        for def in &manifest.provides.taxonomies {
            let csv = std::fs::read(safe_join(source, &def.file)?)
                .map_err(|e| UiError::invalid(format!("{}: {e}", def.file)))?;
            check_taxonomy(&def.id, &csv)?;
        }

        // A dictionary proves itself against a sample only it can read: one the app already
        // reads by itself shows nothing about the words the package brings.
        for def in &manifest.provides.dictionaries {
            let words = std::fs::read_to_string(safe_join(source, &def.file)?)
                .map_err(|e| UiError::invalid(format!("{}: {e}", def.file)))?;
            let sample = std::fs::read(safe_join(source, &def.sample)?)
                .map_err(|e| UiError::invalid(format!("{}: {e}", def.sample)))?;
            crate::import_templates::check_dictionary(&def.id, &words, &sample)?;
        }

        let target = self.folder_of(&manifest.id);
        // A reinstall replaces: the id is the identity, and two copies of one plugin is not a
        // state the list could explain.
        if target.exists() {
            std::fs::remove_dir_all(&target).map_err(io)?;
        }
        std::fs::create_dir_all(&target).map_err(io)?;
        std::fs::copy(source.join(MANIFEST), target.join(MANIFEST)).map_err(io)?;
        let files = manifest
            .provides
            .themes
            .iter()
            .map(|theme| theme.file.clone())
            .chain(
                manifest
                    .provides
                    .layouts
                    .iter()
                    .flat_map(|layout| [layout.file.clone(), layout.sample.clone()]),
            )
            .chain(
                manifest
                    .provides
                    .readers
                    .iter()
                    .flat_map(|def| [def.file.clone(), def.sample.clone(), def.expected.clone()]),
            )
            .chain(
                manifest
                    .provides
                    .writers
                    .iter()
                    .flat_map(|def| [def.file.clone(), def.sample.clone(), def.expected.clone()]),
            )
            .chain(manifest.provides.widgets.iter().map(|def| def.file.clone()))
            .chain(manifest.provides.screens.iter().map(|def| def.file.clone()))
            .chain(manifest.provides.tools.iter().flat_map(|def| {
                [
                    def.file.clone(),
                    def.schema.clone(),
                    def.sample.clone(),
                    def.expected.clone(),
                ]
            }))
            .chain(manifest.provides.taxonomies.iter().map(|def| def.file.clone()))
            .chain(
                manifest
                    .provides
                    .dictionaries
                    .iter()
                    .flat_map(|def| [def.file.clone(), def.sample.clone()]),
            );
        for file in files {
            let from = safe_join(source, &file)?;
            let to = safe_join(&target, &file)?;
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(io)?;
            }
            std::fs::copy(&from, &to)
                .map_err(|e| UiError::invalid(format!("{file} is named by the manifest and missing: {e}")))?;
        }
        Ok(PluginInfo {
            status: status_of(&manifest),
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            themes: manifest.provides.themes,
            layouts: manifest.provides.layouts,
            readers: manifest.provides.readers,
            taxonomies: manifest.provides.taxonomies,
            dictionaries: manifest.provides.dictionaries,
            writers: manifest.provides.writers,
            widgets: manifest.provides.widgets,
            screens: manifest.provides.screens,
            tools: manifest.provides.tools,
        })
    }

    /// Removes a plugin and its folder. What it stored in the profile is not touched here.
    pub fn remove(&self, id: &str) -> UiResult<()> {
        if !valid_id(id) {
            return Err(UiError::not_found(format!("plugin {id}")));
        }
        let folder = self.folder_of(id);
        if !folder.exists() {
            return Err(UiError::not_found(format!("plugin {id}")));
        }
        std::fs::remove_dir_all(folder).map_err(io)
    }
}

/// What a classification set must prove before the package carrying it is installed. No store is
/// touched and no instrument is matched — the securities list is empty on purpose, because a set
/// is checked for being a tree, not for fitting this portfolio.
fn check_taxonomy(id: &str, csv: &[u8]) -> UiResult<()> {
    use sq_core::import::{ParseConfig, Severity, build_taxonomy_preview, detect_taxonomy_config, parse_csv};

    let parsed = parse_csv(csv, &ParseConfig::default())
        .map_err(|e| UiError::invalid(format!("classification set {id}: {e}")))?;
    let config = detect_taxonomy_config(&parsed);
    let preview = build_taxonomy_preview(&parsed, &config, &[], None);

    if preview.nodes.is_empty() {
        return Err(UiError::invalid(format!(
            "classification set {id} reads as no tree at all"
        )));
    }
    if let Some(problem) = preview.problems.iter().find(|p| p.severity == Severity::Error) {
        return Err(UiError::invalid(format!(
            "classification set {id} does not read: {}",
            problem.message
        )));
    }
    Ok(())
}

fn status_of(manifest: &Manifest) -> Status {
    if manifest.api == API {
        Status::Ok
    } else {
        Status::Api {
            wants: manifest.api,
            speaks: API,
        }
    }
}

fn read_manifest(folder: &Path) -> Result<Manifest, String> {
    let text = std::fs::read_to_string(folder.join(MANIFEST)).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// A path a manifest names, resolved inside the package. A file name is data from a stranger,
/// so `../` and an absolute path are refused rather than followed.
fn safe_join(folder: &Path, name: &str) -> UiResult<PathBuf> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn package(dir: &Path, id: &str, api: u32) -> PathBuf {
        let source = dir.join(format!("src-{id}"));
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            source.join(MANIFEST),
            format!(
                r#"{{"id":"{id}","api":{api},"name":"Midnight","version":"1.0.0",
                     "provides":{{"themes":[{{"id":"midnight","name":"Midnight",
                     "file":"midnight.css","base":"dark"}}]}}}}"#
            ),
        )
        .unwrap();
        std::fs::write(source.join("midnight.css"), ":root { --bg: #000; }").unwrap();
        source
    }

    fn temp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("stonqs-plugins-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_installed_plugin_is_listed_with_its_theme() {
        let dir = temp();
        let plugins = Plugins::new(&dir);
        assert!(plugins.list().unwrap().is_empty(), "nothing is installed yet");

        plugins
            .install(&package(&dir, "com.example.midnight", API))
            .unwrap();

        let listed = plugins.list().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].status, Status::Ok);
        assert_eq!(plugins.themes().unwrap()[0].key, "com.example.midnight/midnight");
        assert_eq!(
            plugins.theme_css("com.example.midnight", "midnight").unwrap(),
            ":root { --bg: #000; }"
        );
    }

    #[test]
    fn a_plugin_built_for_another_api_is_listed_and_not_offered() {
        let dir = temp();
        let plugins = Plugins::new(&dir);
        plugins
            .install(&package(&dir, "com.example.future", API + 1))
            .unwrap();

        let listed = plugins.list().unwrap();
        assert_eq!(
            listed[0].status,
            Status::Api {
                wants: API + 1,
                speaks: API
            },
            "a package from a later build is listed, with why"
        );
        assert!(
            plugins.themes().unwrap().is_empty(),
            "a theme that cannot be applied is not in the picker"
        );
        assert!(plugins.theme_css("com.example.future", "midnight").is_err());
    }

    #[test]
    fn a_package_that_declares_nothing_this_build_can_use_is_refused() {
        let dir = temp();
        let source = dir.join("typo");
        std::fs::create_dir_all(&source).unwrap();
        // `provides` misspelled: every required field is there, so nothing else would catch it.
        std::fs::write(
            source.join(MANIFEST),
            r#"{"id":"com.example.typo","api":1,"name":"Typo","provides":{"themez":[]}}"#,
        )
        .unwrap();

        let plugins = Plugins::new(&dir);
        let failure = plugins.install(&source).unwrap_err();
        assert!(format!("{failure:?}").contains("declares nothing"), "{failure:?}");
        assert!(plugins.list().unwrap().is_empty(), "nothing was written");
    }

    #[test]
    fn a_manifest_naming_a_file_outside_the_package_is_refused() {
        let dir = temp();
        let source = dir.join("escape");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            source.join(MANIFEST),
            r#"{"id":"com.example.escape","api":1,"name":"Escape",
                "provides":{"themes":[{"id":"t","name":"T","file":"../../secrets.json"}]}}"#,
        )
        .unwrap();

        let plugins = Plugins::new(&dir);
        assert!(plugins.install(&source).is_err());
    }

    #[test]
    fn reinstalling_replaces_rather_than_doubles() {
        let dir = temp();
        let plugins = Plugins::new(&dir);
        let source = package(&dir, "com.example.midnight", API);
        plugins.install(&source).unwrap();
        std::fs::write(source.join("midnight.css"), ":root { --bg: #111; }").unwrap();
        plugins.install(&source).unwrap();

        assert_eq!(plugins.list().unwrap().len(), 1);
        assert_eq!(
            plugins.theme_css("com.example.midnight", "midnight").unwrap(),
            ":root { --bg: #111; }"
        );
    }

    #[test]
    fn a_removed_plugin_takes_its_folder_with_it() {
        let dir = temp();
        let plugins = Plugins::new(&dir);
        plugins
            .install(&package(&dir, "com.example.midnight", API))
            .unwrap();
        plugins.remove("com.example.midnight").unwrap();

        assert!(plugins.list().unwrap().is_empty());
        assert!(plugins.remove("com.example.midnight").is_err(), "already gone");
    }

    #[test]
    fn a_folder_with_an_unreadable_manifest_is_shown_as_broken() {
        let dir = temp();
        let folder = dir.join(FOLDER).join("com.example.broken");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(MANIFEST), "{ not json").unwrap();

        let listed = Plugins::new(&dir).list().unwrap();
        assert!(matches!(listed[0].status, Status::Broken { .. }));
    }
}
