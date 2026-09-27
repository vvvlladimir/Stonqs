//! Plugins: what the app can be extended with, one folder each under `plugins/<id>/` in the app's
//! data directory. See ADR-0070.
//!
//! Beside the profiles rather than inside one: a profile is everything a *portfolio* owns
//! (ADR-0047), and a theme that vanished when the user switched profile would be a bug nobody
//! could explain. What belongs to a profile is what a plugin *stores*, which no theme does.
//!
//! Plain filesystem work over an explicitly passed root, so it is tested on a temporary folder.
//! This build honours data content — themes, broker layouts, classification sets and operation
//! dictionaries — plus two kinds of compute over one sandbox, the file reader (ADR-0086) and the
//! file writer (ADR-0080), and two kinds of UI, the dashboard widget (ADR-0083) and the screen
//! (ADR-0084); the rest of a
//! manifest is read without being acted on, so a package built for a later version is listed
//! rather than rejected.

pub mod reader;
pub mod reads;
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
/// transaction file (ADR-0086). It ships the sample it was written against **and** what that
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
    /// The folder's name is not the id its manifest gives — moved or copied by hand. Every file
    /// of a plugin is found through its id, so nothing of it is offered until it is reinstalled.
    Misplaced {
        manifest_id: String,
    },
}

/// One installed plugin as the list shows it: what its manifest provides, flattened, so a new kind
/// of content is a field of `Provides` and nothing here.
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
    /// A folder listed for what is wrong with it and offering nothing. `id` is the folder's own
    /// name, so removing it removes that folder.
    fn unusable(id: &str, name: String, status: Status) -> Self {
        PluginInfo {
            id: id.to_string(),
            name,
            version: String::new(),
            provides: Provides::default(),
            status,
        }
    }
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

/// The password the user typed for a sealed file, and the reader that asked for it. Handed to
/// that reader alone: another plugin's reader has no business with it. Never stored, never
/// printed (no `Debug`), and wiped from memory when dropped.
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
    /// The reader that claimed it, and what it read; `None` leaves the file to the app's own.
    pub read: Option<(String, reader::Reading)>,
    /// Readers that broke over the file and were passed over.
    pub skipped: Vec<reader::SkippedReader>,
}

pub struct Plugins {
    root: PathBuf,
    /// Held by whatever rewrites the plugins folder. Install runs off the main thread, so two of
    /// them — or an install and a remove — could otherwise interleave on one folder.
    writing: std::sync::Mutex<()>,
    /// What `list` last read off the disk. An import reads the layouts, the words and the
    /// readers, the assistant the tools, on every step — from this, not from every manifest again,
    /// so one import sees one set of plugins from its preview to its commit. Dropped by whatever
    /// changes the folder, and by `refresh`, which the plugin list calls.
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

    /// Everything installed, in a stable order, each with why it is or is not in use.
    pub fn list(&self) -> UiResult<Vec<PluginInfo>> {
        let mut listed = self.listed.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(plugins) = listed.as_ref() {
            return Ok(plugins.clone());
        }
        let plugins = self.scan()?;
        *listed = Some(plugins.clone());
        Ok(plugins)
    }

    /// Reads the folder again on the next `list`: a package edited on disk by hand is seen once
    /// the plugin list is opened.
    pub fn refresh(&self) {
        *self.listed.lock().unwrap_or_else(|e| e.into_inner()) = None;
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
                Ok(manifest) => PluginInfo {
                    status: status_of(&manifest),
                    id: manifest.id,
                    name: manifest.name,
                    version: manifest.version,
                    provides: manifest.provides,
                },
                Err(detail) => PluginInfo::unusable(&id, id.clone(), Status::Broken { detail }),
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
                p.provides.themes.into_iter().map(move |theme| ThemeInfo {
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
            for layout in plugin.provides.layouts {
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
            for def in plugin.provides.dictionaries {
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
    /// every installed plugin in turn. `not-mine` moves on to the next, and so does a module that
    /// broke before answering — recorded in `skipped`, because one broken package must not refuse
    /// every file the user opens. A reader that claimed the file and then said it is malformed is
    /// an error rather than a fall-through: the reader after it would be reading a file somebody
    /// has already said is theirs.
    ///
    /// A reader that answers `needs-password` stops the import as `UiError::FileProtected`, naming
    /// itself; the load is asked again with `unlock`, whose password reaches that reader only.
    pub fn read_file(&self, name: &str, bytes: &[u8], unlock: Option<&Unlock>) -> UiResult<FileReading> {
        let ending = name
            .rsplit_once('.')
            .map(|(_, end)| format!(".{}", end.to_lowercase()));
        let mut skipped = Vec::new();
        for plugin in self.list()?.into_iter().filter(|p| p.status == Status::Ok) {
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

    /// The export formats on offer, from plugins this build can load.
    pub fn writers(&self) -> UiResult<Vec<WriterInfo>> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|p| p.status == Status::Ok)
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

    /// Writes a transaction document through one installed writer, named `<plugin id>/<writer id>`.
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

    /// The dashboard widgets on offer, from plugins this build can load.
    pub fn widgets(&self) -> UiResult<Vec<WidgetInfo>> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|p| p.status == Status::Ok)
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

    /// The assistant tools on offer, loaded for a message. A tool whose files no longer read is
    /// left out rather than failing the chat, and a name already taken keeps its first owner.
    pub fn tools(&self) -> UiResult<Vec<LoadedTool>> {
        let mut out: Vec<LoadedTool> = Vec::new();
        for plugin in self.list()?.into_iter().filter(|p| p.status == Status::Ok) {
            let folder = self.folder_of(&plugin.id);
            for def in plugin.provides.tools {
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

    /// A page and the policy it must be served under, as the `stonqs-plugin` scheme answers
    /// `/<widget|screen>/<plugin id>/<id>`.
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

    /// Whether a loadable plugin declared a screen that keeps a document: the state commands
    /// answer nobody else, so a package cannot store what its manifest never admitted to.
    pub fn keeps_state(&self, plugin: &str) -> UiResult<bool> {
        let (_, manifest) = self.loaded(plugin)?;
        Ok(manifest.provides.screens.iter().any(|s| s.storage))
    }

    /// The folder and manifest of a plugin this build can load, or why not. The one lookup by id:
    /// the id must be a dull one, the folder must hold the manifest of that id (a folder renamed by
    /// hand answers to neither name), and the manifest must speak this build's API.
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

    /// A theme's stylesheet. Read on demand rather than at startup: only one is ever applied.
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

    /// The classification sets on offer, from plugins this build can load.
    pub fn taxonomy_sets(&self) -> UiResult<Vec<TaxonomySetInfo>> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|p| p.status == Status::Ok)
            .flat_map(|p| {
                p.provides.taxonomies.into_iter().map(move |set| TaxonomySetInfo {
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
        let (folder, manifest) = self.loaded(plugin)?;
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
        // A package that brings nothing this build can use is a typo far more often than it is a
        // package for a later version — `provides` misspelled, or a content kind this build does
        // not know. A missing required field already fails above; an optional one would otherwise
        // install in silence and leave the user looking for a theme that was never declared.
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
        check_regular_files(source, &manifest.provides)?;
        self.check_tool_names(&manifest)?;

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

        // A reinstall replaces: the id is the identity, and two copies of one plugin is not a
        // state the list could explain. The new copy is written beside the old one and swapped in
        // by renaming, so a copy that fails half way leaves the installed version as it was.
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
            // Already out of the list's sight; one that cannot go now goes at the next install.
            let _ = std::fs::remove_dir_all(&old);
        }
        sandbox::forget(&target);
        Ok(PluginInfo {
            status: status_of(&manifest),
            id: manifest.id,
            name: manifest.name,
            version: manifest.version,
            provides: manifest.provides,
        })
    }

    /// The names the model will call this package's tools by must fit a provider, and be taken by
    /// nobody else: `.` and `-` both become `_`, so two ids can meet in one name, and the loser
    /// would vanish from the chat while a session grant kept under that name passed to the winner.
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

    /// Removes a plugin and its folder. What it stored in the profile is not touched here.
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

impl Provides {
    /// Nothing this build can use: the shape a misspelled `provides` takes.
    fn is_empty(&self) -> bool {
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

    /// Every file the content names, besides the manifest: what an install copies and nothing else.
    fn files(&self) -> Vec<&str> {
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

/// Every file the package names must be a file of the package: a symbolic link would have the
/// checks read, and the install copy, whatever it points at on the user's disk.
fn check_regular_files(source: &Path, provides: &Provides) -> UiResult<()> {
    for file in std::iter::once(MANIFEST).chain(provides.files()) {
        let path = safe_join(source, file)?;
        // A missing file is reported by the check or the copy that needs it, naming it.
        if let Ok(meta) = std::fs::symlink_metadata(&path)
            && !meta.file_type().is_file()
        {
            return Err(UiError::invalid(format!(
                "{file} is not a plain file inside the plugin"
            )));
        }
    }
    Ok(())
}

/// Copies the manifest and the files it names from `source` into a new folder `to`.
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

/// Removes what an interrupted install left: a half-written copy or an old one never deleted. Only
/// ever called under `Plugins::writing`, so none of them belongs to an install still running.
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

/// Every id a package gives its content is a key after `<plugin id>/`, a board's stored widget type
/// and a tool's name at a provider — so it is as dull as the plugin's own, and unique within its
/// kind, since a second one of the same id would never be found.
fn check_ids(provides: &Provides) -> UiResult<()> {
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
mod tests;
