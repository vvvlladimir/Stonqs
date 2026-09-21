//! The device-wide provider keys from before profile passwords (ADR-0046): one OS keychain entry
//! holding every provider's key. Since ADR-0048 a key lives in its profile's vault; this entry is
//! only read by the profile a single-profile install became, until that profile gets a password
//! and the keys move into its vault. Nothing new is ever written here. No command reads a key
//! back: `secrets.rs` is the only caller.

use keyring::Entry;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Fixed across every provider, and the same string as `vault::SERVICE`.
const SERVICE: &str = "app.stonqs.ai";
/// The account of the one entry that holds all keys, as JSON `{provider: key}`.
const VAULT: &str = "keys";

pub type Keys = BTreeMap<String, String>;

/// `None` until the first read succeeds. A refused or failed read is not cached, so the next
/// caller asks again rather than the app believing no key was ever saved.
static CACHE: Mutex<Option<Keys>> = Mutex::new(None);

/// Where the entry lives. The keychain in the app; a map in the tests, which must not touch it.
trait Backend {
    fn read(&self, account: &str) -> keyring::Result<Option<String>>;
    fn write(&self, account: &str, secret: &str) -> keyring::Result<()>;
    fn remove(&self, account: &str) -> keyring::Result<()>;
}

struct Keychain;

impl Backend for Keychain {
    fn read(&self, account: &str) -> keyring::Result<Option<String>> {
        match Entry::new(SERVICE, account)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn write(&self, account: &str, secret: &str) -> keyring::Result<()> {
        Entry::new(SERVICE, account)?.set_password(secret)
    }

    fn remove(&self, account: &str) -> keyring::Result<()> {
        match Entry::new(SERVICE, account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// Runs `f` over the keys, loading them first if this run has not yet. The lock is held across
/// the keychain read on purpose: two threads asking at once must wait for one prompt, not raise
/// two.
fn with_keys<T>(
    backend: &dyn Backend,
    f: impl FnOnce(&mut Keys) -> keyring::Result<T>,
) -> keyring::Result<T> {
    let mut cache = CACHE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if cache.is_none() {
        *cache = Some(load(backend)?);
    }
    f(cache.as_mut().expect("loaded above"))
}

fn load(backend: &dyn Backend) -> keyring::Result<Keys> {
    match backend.read(VAULT)? {
        Some(json) => {
            serde_json::from_str(&json).map_err(|e| keyring::Error::Invalid(VAULT.into(), e.to_string()))
        }
        None => migrate(backend),
    }
}

/// Before ADR-0046 each provider had an entry of its own, its id as the account. Those are moved
/// into the one entry once — written there first, removed after, so a failure in between leaves
/// a key in two places rather than in none.
fn migrate(backend: &dyn Backend) -> keyring::Result<Keys> {
    let mut keys = Keys::new();
    let ids = super::catalog::PROVIDERS
        .iter()
        .map(|p| p.id)
        .chain([super::catalog::CUSTOM]);
    for id in ids {
        if let Some(key) = backend.read(id)? {
            keys.insert(id.to_string(), key);
        }
    }
    if !keys.is_empty() {
        store(backend, &keys)?;
        for id in keys.keys() {
            backend.remove(id)?;
        }
    }
    Ok(keys)
}

fn store(backend: &dyn Backend, keys: &Keys) -> keyring::Result<()> {
    if keys.is_empty() {
        return backend.remove(VAULT);
    }
    let json =
        serde_json::to_string(keys).map_err(|e| keyring::Error::Invalid(VAULT.into(), e.to_string()))?;
    backend.write(VAULT, &json)
}

/// The cache changes only once the keychain has taken the change: a refused write must not leave
/// the app believing in a key that is not saved.
fn update(backend: &dyn Backend, change: impl FnOnce(&mut Keys)) -> keyring::Result<()> {
    with_keys(backend, |keys| {
        let mut next = keys.clone();
        change(&mut next);
        store(backend, &next)?;
        *keys = next;
        Ok(())
    })
}

/// Deleting an already-absent key is not an error — the end state is what the caller wants.
pub fn delete(provider: &str) -> keyring::Result<()> {
    update(&Keychain, |keys| {
        keys.remove(provider);
    })
}

pub fn exists(provider: &str) -> keyring::Result<bool> {
    with_keys(&Keychain, |keys| Ok(keys.contains_key(provider)))
}

pub fn get(provider: &str) -> keyring::Result<Option<String>> {
    with_keys(&Keychain, |keys| Ok(keys.get(provider).cloned()))
}

/// Every key, for the move into a vault.
pub fn all() -> keyring::Result<Keys> {
    with_keys(&Keychain, |keys| Ok(keys.clone()))
}

/// Removes the entry once its keys are sealed in a vault.
pub fn clear() -> keyring::Result<()> {
    update(&Keychain, |keys| keys.clear())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A keychain in a map, counting reads — a read is what costs the user a prompt.
    #[derive(Default)]
    struct Fake {
        entries: RefCell<BTreeMap<String, String>>,
        reads: RefCell<usize>,
    }

    impl Backend for Fake {
        fn read(&self, account: &str) -> keyring::Result<Option<String>> {
            *self.reads.borrow_mut() += 1;
            Ok(self.entries.borrow().get(account).cloned())
        }
        fn write(&self, account: &str, secret: &str) -> keyring::Result<()> {
            self.entries.borrow_mut().insert(account.into(), secret.into());
            Ok(())
        }
        fn remove(&self, account: &str) -> keyring::Result<()> {
            self.entries.borrow_mut().remove(account);
            Ok(())
        }
    }

    /// Old per-provider entries end up in the one entry and are gone from where they were.
    #[test]
    fn legacy_entries_move_into_one() {
        let fake = Fake::default();
        fake.write("openai", "sk-a").unwrap();
        fake.write("custom", "sk-c").unwrap();

        let keys = load(&fake).unwrap();
        assert_eq!(keys.get("openai").map(String::as_str), Some("sk-a"));
        assert_eq!(keys.get("custom").map(String::as_str), Some("sk-c"));

        let entries = fake.entries.borrow();
        assert_eq!(entries.len(), 1, "only the vault remains");
        let vault: Keys = serde_json::from_str(&entries[VAULT]).unwrap();
        assert_eq!(vault, keys);
    }

    /// Once the vault exists, loading reads it and nothing else: one read, whatever the number
    /// of providers.
    #[test]
    fn a_vault_is_one_read() {
        let fake = Fake::default();
        fake.write(VAULT, r#"{"anthropic":"sk-b"}"#).unwrap();
        let keys = load(&fake).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(*fake.reads.borrow(), 1);
    }

    /// No keys anywhere writes nothing: an empty vault would be an entry for no reason.
    #[test]
    fn nothing_to_migrate_writes_nothing() {
        let fake = Fake::default();
        assert!(load(&fake).unwrap().is_empty());
        assert!(fake.entries.borrow().is_empty());
    }

    /// Touches the real OS keychain, so it stays out of the default run — like a network test.
    /// Uses its own provider id, never `"openai"`, so it cannot clobber a real saved key.
    #[test]
    #[ignore = "requires OS keychain"]
    fn exists_delete_round_trip() {
        let provider = "stonqs-test";
        update(&Keychain, |keys| {
            keys.insert(provider.into(), "sk-test".into());
        })
        .unwrap();
        assert!(exists(provider).unwrap());

        delete(provider).unwrap();
        assert!(!exists(provider).unwrap());
    }
}
