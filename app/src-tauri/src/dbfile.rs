//! Opening a profile's database with or without its key, and turning the file from plain into
//! encrypted and back (ADR-0049). SQLCipher cannot convert a file in place, so a conversion
//! exports a copy and swaps it in; this module is the only place that renames a database file.

use crate::error::{UiError, UiResult};
use sq_core::storage::Store;
use std::path::{Path, PathBuf};

/// Opens the database — encrypted when a key is given. A file that the key does not open but
/// that opens plain is one whose encryption was interrupted before the swap (the vault already
/// held the key): it is encrypted now instead of being reported as unreadable.
pub fn open(path: &Path, key: Option<&[u8; 32]>) -> UiResult<Store> {
    let Some(key) = key else {
        return Ok(Store::open(path)?);
    };
    match Store::open_encrypted(path, key) {
        Ok(store) => Ok(store),
        Err(encrypted) => match Store::open(path) {
            Ok(plain) => convert(plain, path, Some(key)),
            Err(_) => Err(encrypted.into()),
        },
    }
}

/// Rewrites the database under `key` (or plain for `None`) and returns it reopened. `store` must
/// be the only connection to the file — the caller guarantees it (`AppState::db_gate`).
///
/// The swap keeps a way back at every step: the copy is complete and opened before the original
/// moves, and the original is removed only once the copy is in its place and opens there.
pub fn convert(store: Store, path: &Path, key: Option<&[u8; 32]>) -> UiResult<Store> {
    let copy = sibling(path, "converting");
    let old = sibling(path, "previous");
    remove_quietly(&copy);
    remove_quietly(&old);

    store.checkpoint()?;
    store.export_to(&copy, key)?;
    reopen(&copy, key)?;
    drop(store);

    // An emptied log, but its files are the original's: next to the new file they would be read
    // as its own.
    for suffix in ["-wal", "-shm"] {
        remove_quietly(&with_suffix(path, suffix));
    }
    std::fs::rename(path, &old).map_err(io)?;
    if let Err(e) = std::fs::rename(&copy, path) {
        let _ = std::fs::rename(&old, path);
        return Err(io(e));
    }
    match reopen(path, key) {
        Ok(store) => {
            // An unlink, not an erase: on an SSD the old blocks are the drive's to reuse.
            remove_quietly(&old);
            Ok(store)
        }
        Err(e) => {
            let _ = std::fs::rename(path, &copy);
            let _ = std::fs::rename(&old, path);
            Err(e)
        }
    }
}

fn reopen(path: &Path, key: Option<&[u8; 32]>) -> UiResult<Store> {
    Ok(match key {
        Some(key) => Store::open_encrypted(path, key)?,
        None => Store::open(path)?,
    })
}

fn sibling(path: &Path, tag: &str) -> PathBuf {
    with_suffix(path, &format!(".{tag}"))
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(suffix);
    PathBuf::from(name)
}

fn remove_quietly(path: &Path) {
    let _ = std::fs::remove_file(path);
}

fn io(e: std::io::Error) -> UiError {
    UiError::internal(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sq_core::model::Portfolio;

    fn path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vs-dbfile-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("portfolio.db")
    }

    /// Plain to encrypted and back in the same place, the data intact and no stray files left.
    #[test]
    fn a_file_is_converted_in_its_place() {
        let db = path("convert");
        let key = [3u8; 32];
        let store = Store::open(&db).unwrap();
        store.save_portfolio(&Portfolio::new("Kept", "EUR")).unwrap();

        let sealed = convert(store, &db, Some(&key)).unwrap();
        assert_eq!(sealed.list_portfolios().unwrap()[0].name, "Kept");
        drop(sealed);
        assert!(Store::open(&db).is_err(), "the file itself is encrypted now");

        let plain = convert(open(&db, Some(&key)).unwrap(), &db, None).unwrap();
        assert_eq!(plain.list_portfolios().unwrap()[0].name, "Kept");
        drop(plain);

        let left: Vec<_> = std::fs::read_dir(db.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.contains("converting") || name.contains("previous"))
            .collect();
        assert!(left.is_empty(), "{left:?}");
        std::fs::remove_dir_all(db.parent().unwrap()).unwrap();
    }

    /// The vault holds a key but the swap never happened: opening finishes the job.
    #[test]
    fn an_interrupted_encryption_is_finished_on_open() {
        let db = path("interrupted");
        let key = [4u8; 32];
        let store = Store::open(&db).unwrap();
        store.save_portfolio(&Portfolio::new("Kept", "EUR")).unwrap();
        drop(store);

        let opened = open(&db, Some(&key)).unwrap();
        assert_eq!(opened.list_portfolios().unwrap()[0].name, "Kept");
        drop(opened);
        assert!(Store::open(&db).is_err());
        std::fs::remove_dir_all(db.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_wrong_key_is_an_error_not_a_conversion() {
        let db = path("wrong");
        convert(Store::open(&db).unwrap(), &db, Some(&[5u8; 32])).unwrap();
        assert!(open(&db, Some(&[6u8; 32])).is_err());
        assert!(
            Store::open_encrypted(&db, &[5u8; 32]).is_ok(),
            "the file is untouched"
        );
        std::fs::remove_dir_all(db.parent().unwrap()).unwrap();
    }
}
