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
