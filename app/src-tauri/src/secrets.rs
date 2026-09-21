//! The open profile's password and provider keys, as `AppState` holds them. See ADR-0048.
//!
//! Three states: a profile without a password (a plain database; no key can be saved — except the
//! device-wide keys the first profile had before passwords existed, which it still uses and can
//! delete), a protected profile that is locked (its database is encrypted and not open at all),
//! and a protected profile that is open (keys and the database key come from the vault in
//! memory). See ADR-0048 and ADR-0049.

use crate::ai::{catalog, keys as legacy};
use crate::error::{UiError, UiResult};
use crate::state::AppState;
use crate::vault::{self, Keys, Unlocked};
use std::sync::MutexGuard;
use std::sync::atomic::Ordering;
use zeroize::Zeroizing;

impl AppState {
    pub(crate) fn vault(&self) -> UiResult<MutexGuard<'_, Option<Unlocked>>> {
        self.vault
            .lock()
            .map_err(|_| UiError::internal("the vault state is poisoned"))
    }

    fn folder(&self) -> UiResult<std::path::PathBuf> {
        Ok(crate::profiles::folder_of(&self.db_path()?).to_path_buf())
    }

    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }

    pub fn is_remembered(&self) -> bool {
        self.remembered.load(Ordering::Relaxed)
    }

    /// The database key of the open profile, when it is protected and unlocked.
    pub(crate) fn db_key(&self) -> UiResult<Option<Zeroizing<[u8; 32]>>> {
        Ok(self
            .vault()?
            .as_ref()
            .and_then(|open| open.db_key().map(|k| Zeroizing::new(*k))))
    }

    /// Opens the vault and, with its key, the database — encrypting it first if it is still plain
    /// (a vault from before ADR-0049, or an encryption cut short).
    pub fn unlock(&self, password: &str, remember: bool) -> UiResult<()> {
        let id = self.profile_id()?;
        let _only = self.db_exclusive()?;
        let mut open = vault::unlock(&self.folder()?, &id, password)?;
        let key = open.ensure_db_key()?;
        let store = crate::dbfile::open(&self.db_path()?, Some(&key))?;
        if remember {
            open.remember()?;
            self.remembered.store(true, Ordering::Relaxed);
        }
        self.install(store)?;
        *self.vault()? = Some(open);
        self.locked.store(false, Ordering::Relaxed);
        Ok(())
    }

    /// Closes the database and forgets the keys in memory. A device that remembers the profile
    /// opens it again at the next start; that is what remembering means.
    pub fn lock(&self) -> UiResult<()> {
        if !vault::is_protected(&self.folder()?) {
            return Err(UiError::PasswordRequired {
                message: "a profile without a password cannot be locked".into(),
            });
        }
        self.locked.store(true, Ordering::Relaxed);
        *self.vault()? = None;
        self.install(sq_core::storage::Store::open_in_memory()?)
    }

    /// Gives the profile a password — which encrypts its database — or changes it. The first
    /// profile's device-wide keys move into the new vault and leave the keychain.
    pub fn set_password(&self, current: Option<&str>, password: &str) -> UiResult<()> {
        let id = self.profile_id()?;
        let folder = self.folder()?;
        if let Some(open) = self.vault()?.as_ref() {
            return open.change_password(current.unwrap_or_default(), password);
        }
        if vault::is_protected(&folder) {
            return Err(locked());
        }
        let _only = self.db_exclusive()?;
        let inherited = if id == crate::profiles::ADOPTED_ID {
            legacy::all()?
        } else {
            Keys::new()
        };
        let open = vault::create(&folder, &id, password, inherited)?;
        let key = *open.db_key().expect("a new vault has a database key");
        let path = self.db_path()?;

        let mut slot = self.store_raw()?;
        let plain = std::mem::replace(&mut *slot, sq_core::storage::Store::open_in_memory()?);
        match crate::dbfile::convert(plain, &path, Some(&key)) {
            Ok(sealed) => *slot = sealed,
            Err(e) => {
                // No half state: without the encrypted file there is no password either.
                let _ = vault::delete(&folder, &id);
                *slot = crate::dbfile::open(&path, None)?;
                return Err(e);
            }
        }
        drop(slot);

        *self.vault()? = Some(open);
        self.locked.store(false, Ordering::Relaxed);
        if id == crate::profiles::ADOPTED_ID {
            // Already sealed in the vault; a failure to clear the old entry leaves a copy, not a loss.
            let _ = legacy::clear();
        }
        Ok(())
    }

    /// Removes the password: the database is decrypted and every key behind it is deleted.
    pub fn remove_password(&self, current: &str) -> UiResult<()> {
        let id = self.profile_id()?;
        let folder = self.folder()?;
        let _only = self.db_exclusive()?;
        let checked = vault::unlock(&folder, &id, current)?;
        let key = checked.db_key().copied();
        let path = self.db_path()?;

        let mut slot = self.store_raw()?;
        let sealed = std::mem::replace(&mut *slot, sq_core::storage::Store::open_in_memory()?);
        match crate::dbfile::convert(sealed, &path, None) {
            Ok(plain) => *slot = plain,
            Err(e) => {
                *slot = crate::dbfile::open(&path, key.as_ref())?;
                return Err(e);
            }
        }
        drop(slot);

        vault::delete(&folder, &id)?;
        *self.vault()? = None;
        self.locked.store(false, Ordering::Relaxed);
        self.remembered.store(false, Ordering::Relaxed);
        self.clear_models();
        Ok(())
    }

    /// Deletes the open profile — only ever the open one, so deleting a profile takes being in
    /// it: unlocked, and for a protected one its password typed again. The app moves to another
    /// profile first and removes the folder after; the last profile is never deleted.
    pub fn delete_open_profile(&self, password: Option<&str>) -> UiResult<()> {
        let id = self.profile_id()?;
        if self.is_locked() {
            return Err(locked());
        }
        let folder = self.folder()?;
        if vault::is_protected(&folder) {
            vault::unlock(&folder, &id, password.unwrap_or_default())?;
        }
        let next = self
            .profiles
            .list()?
            .into_iter()
            .find(|p| p.id != id)
            .ok_or_else(|| UiError::invalid("the last profile cannot be deleted"))?;
        // No other connection may still be writing into the folder about to go.
        let _only = self.db_exclusive()?;
        self.open_profile(&next.id)?;
        self.profiles.delete(&id, &next.id)?;
        // Its vault went with the folder; a remembered data key would open nothing.
        let _ = vault::forget(&id);
        Ok(())
    }

    pub fn set_remembered(&self, remember: bool) -> UiResult<()> {
        let id = self.profile_id()?;
        let slot = self.vault()?;
        let open = slot.as_ref().ok_or_else(locked)?;
        if remember {
            open.remember()?;
        } else {
            vault::forget(&id)?;
        }
        self.remembered.store(remember, Ordering::Relaxed);
        Ok(())
    }

    pub fn key_exists(&self, provider: &str) -> UiResult<bool> {
        if let Some(open) = self.vault()?.as_ref() {
            return Ok(open.keys().contains_key(provider));
        }
        if self.is_locked() {
            return Err(locked());
        }
        if self.inherits_device_keys()? {
            return Ok(legacy::exists(provider)?);
        }
        Ok(false)
    }

    /// The key to send with a call. Missing is a failure everywhere but the custom provider, where
    /// it is normal: a model served from this machine (Ollama, LM Studio, llama.cpp) authenticates
    /// nothing, and refusing to call it would be demanding a key be invented.
    pub fn key_for_call(&self, provider: &str) -> UiResult<String> {
        let found = if let Some(open) = self.vault()?.as_ref() {
            open.keys().get(provider).cloned()
        } else if self.is_locked() {
            return Err(locked());
        } else if self.inherits_device_keys()? {
            legacy::get(provider)?
        } else {
            None
        };
        match found {
            Some(key) => Ok(key),
            None if provider == catalog::CUSTOM => Ok(String::new()),
            None => Err(UiError::Auth {
                message: "no key is saved for this provider".into(),
            }),
        }
    }

    pub fn key_save(&self, provider: &str, key: &str) -> UiResult<()> {
        let mut slot = self.vault()?;
        match slot.as_mut() {
            Some(open) => open.update(|keys| {
                keys.insert(provider.to_string(), key.to_string());
            }),
            None if self.is_locked() => Err(locked()),
            None => Err(UiError::PasswordRequired {
                message: "a key is kept only behind a profile password".into(),
            }),
        }
    }

    pub fn key_delete(&self, provider: &str) -> UiResult<()> {
        let mut slot = self.vault()?;
        match slot.as_mut() {
            Some(open) => open.update(|keys| {
                keys.remove(provider);
            }),
            None if self.is_locked() => Err(locked()),
            None if self.inherits_device_keys()? => Ok(legacy::delete(provider)?),
            None => Ok(()),
        }
    }

    /// Only the profile a single-profile install became, and only until it has a password: the
    /// device-wide keys were that install's, and no other profile ever saw them.
    fn inherits_device_keys(&self) -> UiResult<bool> {
        Ok(self.profile_id()? == crate::profiles::ADOPTED_ID && !vault::is_protected(&self.folder()?))
    }

    pub(crate) fn clear_models(&self) {
        if let Ok(mut models) = self.ai_models.lock() {
            models.clear();
        }
    }
}

fn locked() -> UiError {
    UiError::Locked {
        message: "the profile is locked".into(),
    }
}
