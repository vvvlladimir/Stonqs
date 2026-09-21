//! A profile's secrets — the AI provider keys — sealed under its password. See ADR-0048.
//!
//! `vault.json` sits in the profile's folder. Its presence is what "the profile has a password"
//! means. The password never opens the keys directly: Argon2id turns it into a key-encryption key
//! that seals a random data key, and the data key seals the keys. Changing the password therefore
//! re-seals 32 bytes, and "remember on this device" stores the data key, never the password.
//!
//! Plain files over an explicitly passed folder, so it is tested on a temporary one; which vault
//! is open is `AppState`'s business.

use crate::error::{UiError, UiResult};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng, Payload, rand_core::RngCore};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use zeroize::{Zeroize, Zeroizing};

const FILE: &str = "vault.json";
const VERSION: u32 = 1;

/// Keychain service of the remembered data keys; the account is `profile:<id>`.
const SERVICE: &str = "app.stonqs.desktop.ai";

/// Shortest password accepted. NIST SP 800-63B asks for at least eight characters and nothing
/// else — no composition rules, which only make passwords harder to remember.
pub const MIN_PASSWORD: usize = 8;

/// Argon2id cost for new vaults: 64 MiB, three passes, one lane — above OWASP's floor
/// (19 MiB, two passes) and still well under a second on a phone. Stored per vault, so a later
/// build can raise it without breaking the old files.
const COST: Kdf = Kdf {
    // The tests derive dozens of keys in a debug build; the arithmetic is the same at any cost.
    m_kib: if cfg!(test) { 1024 } else { 64 * 1024 },
    t: if cfg!(test) { 1 } else { 3 },
    p: 1,
    salt: String::new(),
};

pub type Keys = BTreeMap<String, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Kdf {
    m_kib: u32,
    t: u32,
    p: u32,
    salt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Sealed {
    nonce: String,
    data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct File {
    version: u32,
    kdf: Kdf,
    /// The data key, sealed with the key derived from the password.
    data_key: Sealed,
    /// The provider keys as JSON `{provider: key}`, sealed with the data key.
    keys: Sealed,
    /// The database's SQLCipher key, sealed with the data key (ADR-0049). Absent in a vault made
    /// before the database was encrypted; `Unlocked::ensure_db_key` adds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    db_key: Option<Sealed>,
}

/// An open vault: the data key and the keys it sealed, in memory until the profile is locked or
/// left. Both are wiped when this is dropped.
pub struct Unlocked {
    folder: PathBuf,
    profile: String,
    data_key: Zeroizing<[u8; 32]>,
    keys: Keys,
    db_key: Option<Zeroizing<[u8; 32]>>,
}

impl Drop for Unlocked {
    fn drop(&mut self) {
        for key in self.keys.values_mut() {
            key.zeroize();
        }
    }
}

pub fn is_protected(folder: &Path) -> bool {
    folder.join(FILE).exists()
}

/// Gives a profile its password. `keys` are what it starts with — the device-wide keys of a
/// profile that had them before passwords existed, or none.
pub fn create(folder: &Path, profile: &str, password: &str, keys: Keys) -> UiResult<Unlocked> {
    if is_protected(folder) {
        return Err(UiError::invalid("the profile already has a password"));
    }
    check_password(password)?;
    let mut data_key = Zeroizing::new([0u8; 32]);
    OsRng.fill_bytes(&mut data_key[..]);
    let unlocked = Unlocked {
        folder: folder.to_path_buf(),
        profile: profile.to_string(),
        data_key,
        keys,
        db_key: Some(random_key()),
    };
    let kdf = fresh_kdf();
    let file = File {
        version: VERSION,
        data_key: seal_data_key(&kdf, password, profile, &unlocked.data_key)?,
        kdf,
        keys: unlocked.seal_keys()?,
        db_key: unlocked.seal_db_key()?,
    };
    write(folder, &file)?;
    Ok(unlocked)
}

/// Opens the vault with the password. A wrong one is `wrong_password`, never a generic failure:
/// the user's next step is to type it again.
pub fn unlock(folder: &Path, profile: &str, password: &str) -> UiResult<Unlocked> {
    let file = read(folder)?;
    let kek = derive(&file.kdf, password)?;
    let bytes = open(&kek, &file.data_key, &aad("data_key", profile)).ok_or_else(wrong_password)?;
    let data_key = to_key(bytes)?;
    open_keys(folder, profile, data_key, &file)
}

/// Opens the vault with a data key remembered on this device.
pub fn unlock_with(folder: &Path, profile: &str, data_key: Zeroizing<[u8; 32]>) -> UiResult<Unlocked> {
    let file = read(folder)?;
    open_keys(folder, profile, data_key, &file)
}

/// Removes the vault and, with it, every key it held: a key is kept only behind a password.
pub fn delete(folder: &Path, profile: &str) -> UiResult<()> {
    std::fs::remove_file(folder.join(FILE)).map_err(io)?;
    forget(profile)
}

impl Unlocked {
    pub fn keys(&self) -> &Keys {
        &self.keys
    }

    pub fn db_key(&self) -> Option<&[u8; 32]> {
        self.db_key.as_deref()
    }

    /// The database key, created and sealed into the file if this vault predates it.
    pub fn ensure_db_key(&mut self) -> UiResult<[u8; 32]> {
        if self.db_key.is_none() {
            self.db_key = Some(random_key());
            let mut file = read(&self.folder)?;
            file.db_key = self.seal_db_key()?;
            if let Err(e) = write(&self.folder, &file) {
                self.db_key = None;
                return Err(e);
            }
        }
        Ok(*self.db_key.as_deref().expect("set above"))
    }

    fn seal_db_key(&self) -> UiResult<Option<Sealed>> {
        self.db_key
            .as_deref()
            .map(|key| seal(&self.data_key, key, &aad("db_key", &self.profile)))
            .transpose()
    }

    /// Changes the stored keys; memory follows only once the file has taken the change.
    pub fn update(&mut self, change: impl FnOnce(&mut Keys)) -> UiResult<()> {
        let mut next = self.keys.clone();
        change(&mut next);
        let previous = std::mem::replace(&mut self.keys, next);
        let written = self.seal_keys().and_then(|keys| {
            let mut file = read(&self.folder)?;
            file.keys = keys;
            write(&self.folder, &file)
        });
        if written.is_err() {
            self.keys = previous;
        }
        written
    }

    /// Re-seals the data key under a new password. The current one is asked again even though
    /// the vault is open: an unlocked, unattended app must not be enough to take it over.
    pub fn change_password(&self, current: &str, new: &str) -> UiResult<()> {
        check_password(new)?;
        unlock(&self.folder, &self.profile, current)?;
        let mut file = read(&self.folder)?;
        file.kdf = fresh_kdf();
        file.data_key = seal_data_key(&file.kdf, new, &self.profile, &self.data_key)?;
        write(&self.folder, &file)
    }

    /// Keeps the data key in the OS keychain, so this device opens the profile without asking.
    pub fn remember(&self) -> UiResult<()> {
        let encoded = Zeroizing::new(B64.encode(&self.data_key[..]));
        entry(&self.profile)?.set_password(&encoded).map_err(keychain)
    }

    fn seal_keys(&self) -> UiResult<Sealed> {
        let json =
            Zeroizing::new(serde_json::to_vec(&self.keys).map_err(|e| UiError::internal(e.to_string()))?);
        seal(&self.data_key, &json, &aad("keys", &self.profile))
    }
}

/// The data key this device remembers for a profile, if it does.
pub fn recall(profile: &str) -> UiResult<Option<Zeroizing<[u8; 32]>>> {
    match entry(profile)?.get_password() {
        Ok(encoded) => {
            let encoded = Zeroizing::new(encoded);
            let bytes = Zeroizing::new(
                B64.decode(encoded.as_bytes())
                    .map_err(|e| UiError::internal(e.to_string()))?,
            );
            Ok(Some(to_key(bytes)?))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(keychain(e)),
    }
}

pub fn forget(profile: &str) -> UiResult<()> {
    match entry(profile)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(keychain(e)),
    }
}

fn open_keys(folder: &Path, profile: &str, data_key: Zeroizing<[u8; 32]>, file: &File) -> UiResult<Unlocked> {
    // A remembered key that no longer opens the vault (the vault was recreated) is a wrong key,
    // and the answer to it is the password.
    let json = open(&data_key, &file.keys, &aad("keys", profile)).ok_or_else(wrong_password)?;
    let keys: Keys = serde_json::from_slice(&json).map_err(|e| UiError::internal(e.to_string()))?;
    let db_key = match &file.db_key {
        Some(sealed) => Some(to_key(
            open(&data_key, sealed, &aad("db_key", profile)).ok_or_else(wrong_password)?,
        )?),
        None => None,
    };
    Ok(Unlocked {
        folder: folder.to_path_buf(),
        profile: profile.to_string(),
        data_key,
        keys,
        db_key,
    })
}

fn check_password(password: &str) -> UiResult<()> {
    if password.chars().count() < MIN_PASSWORD {
        return Err(UiError::invalid("the password is shorter than eight characters"));
    }
    Ok(())
}

fn random_key() -> Zeroizing<[u8; 32]> {
    let mut key = Zeroizing::new([0u8; 32]);
    OsRng.fill_bytes(&mut key[..]);
    key
}

fn fresh_kdf() -> Kdf {
    let mut salt = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    Kdf {
        salt: B64.encode(salt),
        ..COST
    }
}

fn derive(kdf: &Kdf, password: &str) -> UiResult<Zeroizing<[u8; 32]>> {
    let params =
        Params::new(kdf.m_kib, kdf.t, kdf.p, Some(32)).map_err(|e| UiError::internal(e.to_string()))?;
    let salt = B64
        .decode(&kdf.salt)
        .map_err(|e| UiError::internal(e.to_string()))?;
    let mut out = Zeroizing::new([0u8; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), &salt, &mut out[..])
        .map_err(|e| UiError::internal(e.to_string()))?;
    Ok(out)
}

fn seal_data_key(kdf: &Kdf, password: &str, profile: &str, data_key: &[u8; 32]) -> UiResult<Sealed> {
    let kek = derive(kdf, password)?;
    seal(&kek, data_key, &aad("data_key", profile))
}

/// Associated data binds a ciphertext to its purpose and its profile: a sealed blob copied into
/// another field or another profile's vault does not open there.
fn aad(purpose: &str, profile: &str) -> Vec<u8> {
    format!("stonqs/vault/v{VERSION}/{purpose}/{profile}").into_bytes()
}

fn seal(key: &[u8; 32], plain: &[u8], aad: &[u8]) -> UiResult<Sealed> {
    let cipher = XChaCha20Poly1305::new(key.into());
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    let data = cipher
        .encrypt(&nonce, Payload { msg: plain, aad })
        .map_err(|_| UiError::internal("sealing failed"))?;
    Ok(Sealed {
        nonce: B64.encode(nonce),
        data: B64.encode(data),
    })
}

/// `None` is a failed authentication — a wrong key or a tampered file; both mean "not this key".
fn open(key: &[u8; 32], sealed: &Sealed, aad: &[u8]) -> Option<Zeroizing<Vec<u8>>> {
    let nonce = B64.decode(&sealed.nonce).ok()?;
    let data = B64.decode(&sealed.data).ok()?;
    if nonce.len() != 24 {
        return None;
    }
    let cipher = XChaCha20Poly1305::new(key.into());
    cipher
        .decrypt(XNonce::from_slice(&nonce), Payload { msg: &data, aad })
        .ok()
        .map(Zeroizing::new)
}

fn to_key(bytes: Zeroizing<Vec<u8>>) -> UiResult<Zeroizing<[u8; 32]>> {
    let array: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| UiError::internal("a data key is 32 bytes"))?;
    Ok(Zeroizing::new(array))
}

fn read(folder: &Path) -> UiResult<File> {
    match std::fs::read(folder.join(FILE)) {
        Ok(bytes) => {
            let file: File = serde_json::from_slice(&bytes).map_err(|e| UiError::internal(e.to_string()))?;
            if file.version != VERSION {
                return Err(UiError::internal(format!(
                    "vault version {} is unknown",
                    file.version
                )));
            }
            Ok(file)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(UiError::PasswordRequired {
            message: "the profile has no password".into(),
        }),
        Err(e) => Err(io(e)),
    }
}

/// Written aside and renamed over: a torn vault would lose every key at once.
fn write(folder: &Path, file: &File) -> UiResult<()> {
    let json = serde_json::to_vec_pretty(file).map_err(|e| UiError::internal(e.to_string()))?;
    let temp = folder.join(format!("{FILE}.tmp"));
    std::fs::write(&temp, json).map_err(io)?;
    std::fs::rename(&temp, folder.join(FILE)).map_err(io)
}

fn entry(profile: &str) -> UiResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, &format!("profile:{profile}")).map_err(keychain)
}

fn wrong_password() -> UiError {
    UiError::WrongPassword {
        message: "the password does not open this profile".into(),
    }
}

fn keychain(e: keyring::Error) -> UiError {
    UiError::internal(e.to_string())
}

fn io(e: std::io::Error) -> UiError {
    UiError::internal(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("vs-vault-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn keys(pairs: &[(&str, &str)]) -> Keys {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// What goes in comes out with the right password, and nothing of it is readable on disk.
    #[test]
    fn keys_round_trip_and_are_not_on_disk_in_the_clear() {
        let dir = folder("round");
        let created = create(&dir, "p1", "correct horse", keys(&[("openai", "sk-secret-123")])).unwrap();
        drop(created);

        let on_disk = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(!on_disk.contains("sk-secret-123"));
        assert!(!on_disk.contains("correct horse"));

        let opened = unlock(&dir, "p1", "correct horse").unwrap();
        assert_eq!(opened.keys()["openai"], "sk-secret-123");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_wrong_password_is_its_own_error() {
        let dir = folder("wrong");
        create(&dir, "p1", "correct horse", Keys::new()).unwrap();
        let error = unlock(&dir, "p1", "wrong horse!").err().unwrap();
        assert!(matches!(error, UiError::WrongPassword { .. }));
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// A vault copied into another profile's folder does not open there: the profile id is part
    /// of what was sealed.
    #[test]
    fn a_vault_is_bound_to_its_profile() {
        let dir = folder("bound");
        create(&dir, "p1", "correct horse", Keys::new()).unwrap();
        assert!(unlock(&dir, "p2", "correct horse").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// A new password opens the same keys; the old one no longer does. The data key is unchanged,
    /// which is why a device that remembered it keeps working.
    #[test]
    fn changing_the_password_keeps_the_keys() {
        let dir = folder("change");
        let mut vault = create(&dir, "p1", "first password", Keys::new()).unwrap();
        vault
            .update(|k| {
                k.insert("anthropic".into(), "sk-b".into());
            })
            .unwrap();
        assert!(
            vault
                .change_password("not the password", "second password")
                .is_err()
        );
        vault
            .change_password("first password", "second password")
            .unwrap();
        let data_key = vault.data_key.clone();
        drop(vault);

        assert!(unlock(&dir, "p1", "first password").is_err());
        assert_eq!(
            unlock(&dir, "p1", "second password").unwrap().keys()["anthropic"],
            "sk-b"
        );
        assert_eq!(
            unlock_with(&dir, "p1", data_key).unwrap().keys()["anthropic"],
            "sk-b"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// The database key comes back the same after every unlock and survives a password change —
    /// a different one would leave the encrypted database unreadable.
    #[test]
    fn the_database_key_is_stable() {
        let dir = folder("dbkey");
        let vault = create(&dir, "p1", "first password", Keys::new()).unwrap();
        let key = *vault.db_key().unwrap();
        vault
            .change_password("first password", "second password")
            .unwrap();
        drop(vault);
        assert_eq!(
            *unlock(&dir, "p1", "second password").unwrap().db_key().unwrap(),
            key
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// A vault written before databases were encrypted gets its key on first use, once.
    #[test]
    fn an_older_vault_gains_a_database_key() {
        let dir = folder("older");
        drop(create(&dir, "p1", "long enough", Keys::new()).unwrap());
        let mut file = read(&dir).unwrap();
        file.db_key = None;
        write(&dir, &file).unwrap();

        let mut open = unlock(&dir, "p1", "long enough").unwrap();
        assert!(open.db_key().is_none());
        let key = open.ensure_db_key().unwrap();
        assert_eq!(open.ensure_db_key().unwrap(), key, "created once");
        drop(open);
        assert_eq!(*unlock(&dir, "p1", "long enough").unwrap().db_key().unwrap(), key);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn short_passwords_and_a_second_vault_are_refused() {
        let dir = folder("rules");
        assert!(create(&dir, "p1", "short", Keys::new()).is_err());
        create(&dir, "p1", "long enough", Keys::new()).unwrap();
        assert!(create(&dir, "p1", "long enough", Keys::new()).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn no_vault_reads_as_password_required() {
        let dir = folder("none");
        assert!(!is_protected(&dir));
        assert!(matches!(
            unlock(&dir, "p1", "whatever1").err().unwrap(),
            UiError::PasswordRequired { .. }
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
