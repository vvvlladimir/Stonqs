//! Profiles: independent sets of everything the app stores — database, settings, import layouts —
//! one folder each under `profiles/<id>/` in the app's data directory. `profiles.json` beside
//! that folder lists them and remembers which one was open last. See ADR-0047.
//!
//! Plain filesystem work over an explicitly passed root, so it is tested on a temporary folder;
//! swapping the open profile inside the running app is `AppState::open_profile`.

use crate::error::{UiError, UiResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const REGISTRY: &str = "profiles.json";
const FOLDER: &str = "profiles";
const DATABASE: &str = "portfolio.db";

/// The id the data of a single-profile install moves to. Fixed rather than random so that an
/// interrupted move finishes into the same folder on the next start.
pub const ADOPTED_ID: &str = "default";

/// What a single-profile install kept directly in the data directory. Every file the host writes
/// sits beside the database (`settings::path_for`, `import_templates::path_for`), so this list is
/// the whole of it — a file added there must be added here too.
const LEGACY_FILES: &[&str] = &[
    "portfolio.db",
    "portfolio.db-wal",
    "portfolio.db-shm",
    "settings.json",
    "import_templates.json",
    "import_presets_hidden.json",
];

/// Longest name kept, in characters: a picker line, not a paragraph.
const NAME_MAX: usize = 60;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    /// The user's own words, like a portfolio's name; seeded in English for the first profile.
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Registry {
    profiles: Vec<Profile>,
    #[serde(default)]
    last: Option<String>,
}

pub struct Profiles {
    root: PathBuf,
}

impl Profiles {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Profiles { root: root.into() }
    }

    pub fn db_path(&self, id: &str) -> PathBuf {
        self.root.join(FOLDER).join(id).join(DATABASE)
    }

    /// Makes sure at least one profile exists and returns the one to open: the last one used,
    /// or the first. On the first start after profiles arrived, the data already in the data
    /// directory becomes the first profile instead of being left behind.
    pub fn ensure(&self, first_name: &str) -> UiResult<Profile> {
        let mut registry = self.read()?;
        if registry.profiles.is_empty() {
            let folder = self.root.join(FOLDER).join(ADOPTED_ID);
            std::fs::create_dir_all(&folder).map_err(io)?;
            // Each rename is atomic, and one already done is skipped, so a start interrupted
            // halfway finishes the move the next time — the registry is written only after.
            for file in LEGACY_FILES {
                let from = self.root.join(file);
                if from.exists() {
                    std::fs::rename(&from, folder.join(file)).map_err(io)?;
                }
            }
            registry.profiles.push(Profile {
                id: ADOPTED_ID.into(),
                name: first_name.into(),
                created_at: now(),
            });
            registry.last = Some(ADOPTED_ID.into());
            self.write(&registry)?;
        }
        let chosen = registry
            .last
            .as_ref()
            .and_then(|last| registry.profiles.iter().find(|p| &p.id == last))
            .unwrap_or(&registry.profiles[0]);
        Ok(chosen.clone())
    }

    pub fn list(&self) -> UiResult<Vec<Profile>> {
        Ok(self.read()?.profiles)
    }

    pub fn find(&self, id: &str) -> UiResult<Profile> {
        self.read()?
            .profiles
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| UiError::not_found(format!("profile {id}")))
    }

    /// A new, empty profile. It is not opened: creating one is not a request to leave this one.
    pub fn create(&self, name: &str) -> UiResult<Profile> {
        let mut registry = self.read()?;
        let name = valid_name(&registry, name, None)?;
        let profile = Profile {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            created_at: now(),
        };
        std::fs::create_dir_all(self.root.join(FOLDER).join(&profile.id)).map_err(io)?;
        registry.profiles.push(profile.clone());
        self.write(&registry)?;
        Ok(profile)
    }

    pub fn rename(&self, id: &str, name: &str) -> UiResult<Profile> {
        let mut registry = self.read()?;
        let name = valid_name(&registry, name, Some(id))?;
        let profile = registry
            .profiles
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| UiError::not_found(format!("profile {id}")))?;
        profile.name = name;
        let renamed = profile.clone();
        self.write(&registry)?;
        Ok(renamed)
    }

    /// Deletes a profile and its folder — every portfolio, quote and setting in it. The open
    /// profile is refused (its database is open), and so is the last one: the app always has
    /// somewhere to be.
    pub fn delete(&self, id: &str, open: &str) -> UiResult<()> {
        let mut registry = self.read()?;
        if id == open {
            return Err(UiError::invalid("the open profile cannot be deleted"));
        }
        if !registry.profiles.iter().any(|p| p.id == id) {
            return Err(UiError::not_found(format!("profile {id}")));
        }
        if registry.profiles.len() == 1 {
            return Err(UiError::invalid("the last profile cannot be deleted"));
        }
        // Out of the list first: a folder that failed to go is wasted space, a listed profile
        // without its folder would open as an empty one.
        registry.profiles.retain(|p| p.id != id);
        self.write(&registry)?;
        let folder = self.root.join(FOLDER).join(id);
        if folder.exists() {
            std::fs::remove_dir_all(folder).map_err(io)?;
        }
        Ok(())
    }

    /// Remembered so the next start opens the same profile.
    pub fn set_last(&self, id: &str) -> UiResult<()> {
        let mut registry = self.read()?;
        registry.last = Some(id.to_string());
        self.write(&registry)
    }

    fn read(&self) -> UiResult<Registry> {
        let path = self.root.join(REGISTRY);
        match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| UiError::internal(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Registry::default()),
            Err(e) => Err(io(e)),
        }
    }

    /// Written aside and renamed over: a half-written list would lose every profile at once.
    fn write(&self, registry: &Registry) -> UiResult<()> {
        std::fs::create_dir_all(&self.root).map_err(io)?;
        let json = serde_json::to_vec_pretty(registry).map_err(|e| UiError::internal(e.to_string()))?;
        let temp = self.root.join(format!("{REGISTRY}.tmp"));
        std::fs::write(&temp, json).map_err(io)?;
        std::fs::rename(&temp, self.root.join(REGISTRY)).map_err(io)
    }
}

/// Trimmed, not empty, not too long, and not another profile's name — a picker of two lines
/// saying the same thing offers no choice.
fn valid_name(registry: &Registry, name: &str, except: Option<&str>) -> UiResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(UiError::invalid("a profile needs a name"));
    }
    let name: String = name.chars().take(NAME_MAX).collect();
    let taken = registry
        .profiles
        .iter()
        .any(|p| Some(p.id.as_str()) != except && p.name.to_lowercase() == name.to_lowercase());
    if taken {
        return Err(UiError::invalid("another profile has this name"));
    }
    Ok(name)
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn io(e: std::io::Error) -> UiError {
    UiError::internal(e.to_string())
}

/// The folder a profile's database lives in; everything else of it sits beside the database.
pub fn folder_of(db_path: &Path) -> &Path {
    db_path.parent().unwrap_or(db_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("vs-profiles-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    /// A single-profile install keeps its data: the files move into the first profile's folder
    /// and nothing is left in the data directory itself.
    #[test]
    fn existing_data_becomes_the_first_profile() {
        let root = temp_root("adopt");
        std::fs::write(root.join("portfolio.db"), b"db").unwrap();
        std::fs::write(root.join("settings.json"), b"{}").unwrap();

        let profiles = Profiles::new(&root);
        let first = profiles.ensure("Default").unwrap();
        assert_eq!(first.id, ADOPTED_ID);
        assert_eq!(std::fs::read(profiles.db_path(ADOPTED_ID)).unwrap(), b"db");
        assert!(
            folder_of(&profiles.db_path(ADOPTED_ID))
                .join("settings.json")
                .exists()
        );
        assert!(!root.join("portfolio.db").exists());

        // A second start changes nothing.
        assert_eq!(profiles.ensure("Default").unwrap(), first);
        assert_eq!(profiles.list().unwrap().len(), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    /// The last profile opened is the one the next start opens.
    #[test]
    fn the_last_opened_profile_is_reopened() {
        let root = temp_root("last");
        let profiles = Profiles::new(&root);
        profiles.ensure("Default").unwrap();
        let second = profiles.create("Family").unwrap();
        assert_eq!(
            profiles.ensure("Default").unwrap().id,
            ADOPTED_ID,
            "creating does not open"
        );

        profiles.set_last(&second.id).unwrap();
        assert_eq!(profiles.ensure("Default").unwrap(), second);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn names_are_trimmed_and_unique() {
        let root = temp_root("names");
        let profiles = Profiles::new(&root);
        profiles.ensure("Default").unwrap();
        assert!(profiles.create("   ").is_err());
        assert!(
            profiles.create(" default ").is_err(),
            "case-insensitive duplicate"
        );
        let family = profiles.create("  Family ").unwrap();
        assert_eq!(family.name, "Family");
        assert!(
            profiles.rename(&family.id, "Family").is_ok(),
            "its own name is not taken"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// The open profile and the last one are refused; anything else goes with its folder.
    #[test]
    fn delete_refuses_the_open_and_the_last() {
        let root = temp_root("delete");
        let profiles = Profiles::new(&root);
        let first = profiles.ensure("Default").unwrap();
        assert!(
            profiles.delete(&first.id, "elsewhere").is_err(),
            "the last one stays"
        );

        let second = profiles.create("Family").unwrap();
        assert!(
            profiles.delete(&second.id, &second.id).is_err(),
            "the open one stays"
        );
        profiles.delete(&second.id, &first.id).unwrap();
        assert!(!folder_of(&profiles.db_path(&second.id)).exists());
        assert_eq!(profiles.list().unwrap(), vec![first]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
