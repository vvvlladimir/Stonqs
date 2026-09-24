//! Plugins: what the app can be extended with, one folder each under `plugins/<id>/` in the app's
//! data directory. See ADR-0070.
//!
//! Beside the profiles rather than inside one: a profile is everything a *portfolio* owns
//! (ADR-0047), and a theme that vanished when the user switched profile would be a bug nobody
//! could explain. What belongs to a profile is what a plugin *stores*, which no theme does.
//!
//! Plain filesystem work over an explicitly passed root, so it is tested on a temporary folder.
//! This build honours data content — themes and broker layouts — plus one kind of compute, the
//! file reader (ADR-0073); the rest of a manifest is read without being acted on, so a package
//! built for a later version is listed rather than rejected.

pub mod reader;

use crate::error::{UiError, UiResult};
use serde::{Deserialize, Serialize};
use sq_core::import::BrokerPreset;
use std::path::{Path, PathBuf};

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
    #[serde(flatten)]
    pub status: Status,
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
                },
                Err(detail) => PluginInfo {
                    id: id.clone(),
                    name: id,
                    version: String::new(),
                    themes: Vec::new(),
                    layouts: Vec::new(),
                    readers: Vec::new(),
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
        {
            return Err(UiError::invalid(format!(
                "plugin {} declares nothing this build can use: expected `provides.themes`, \
                 `provides.layouts` or `provides.readers`",
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
