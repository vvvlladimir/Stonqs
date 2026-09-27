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
fn a_content_id_that_is_not_dull_or_is_taken_twice_is_refused() {
    let dir = temp();
    let plugins = Plugins::new(&dir);
    let with_themes = |themes: &str| {
        let source = package(&dir, "com.example.ids", API);
        std::fs::write(
            source.join(MANIFEST),
            format!(r#"{{"id":"com.example.ids","api":1,"name":"Ids","provides":{{"themes":[{themes}]}}}}"#),
        )
        .unwrap();
        source
    };
    let theme = |id: &str| format!(r#"{{"id":"{id}","name":"T","file":"midnight.css"}}"#);

    let spaced = plugins.install(&with_themes(&theme("My Theme"))).unwrap_err();
    assert!(format!("{spaced:?}").contains("theme id"), "{spaced:?}");
    let slashed = plugins.install(&with_themes(&theme("a/b"))).unwrap_err();
    assert!(
        format!("{slashed:?}").contains("theme id"),
        "a slash would split the key"
    );
    let twice = plugins
        .install(&with_themes(&format!("{},{}", theme("t"), theme("t"))))
        .unwrap_err();
    assert!(format!("{twice:?}").contains("two of its themes"), "{twice:?}");
    assert!(plugins.list().unwrap().is_empty(), "nothing was written");
}

#[test]
fn a_reinstall_that_cannot_be_copied_leaves_the_installed_version() {
    let dir = temp();
    let plugins = Plugins::new(&dir);
    let source = package(&dir, "com.example.midnight", API);
    plugins.install(&source).unwrap();

    // A theme is not read by any check, so only the copy notices it is gone.
    std::fs::remove_file(source.join("midnight.css")).unwrap();
    assert!(plugins.install(&source).is_err());

    assert_eq!(plugins.list().unwrap().len(), 1, "no half-written copy is listed");
    assert_eq!(
        plugins.theme_css("com.example.midnight", "midnight").unwrap(),
        ":root { --bg: #000; }",
        "the version installed before is still whole"
    );
    let stray = std::fs::read_dir(dir.join(FOLDER))
        .unwrap()
        .filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().starts_with('.'))
        .count();
    assert_eq!(stray, 0, "the failed copy was cleaned up");
}

#[test]
fn a_folder_renamed_by_hand_is_listed_as_misplaced_and_offers_nothing() {
    let dir = temp();
    let plugins = Plugins::new(&dir);
    plugins
        .install(&package(&dir, "com.example.midnight", API))
        .unwrap();
    std::fs::rename(
        dir.join(FOLDER).join("com.example.midnight"),
        dir.join(FOLDER).join("renamed"),
    )
    .unwrap();

    let listed = plugins.list().unwrap();
    assert_eq!(listed[0].id, "renamed", "removing it removes that folder");
    assert_eq!(
        listed[0].status,
        Status::Misplaced {
            manifest_id: "com.example.midnight".into()
        }
    );
    assert!(plugins.themes().unwrap().is_empty());
    assert!(
        plugins.theme_css("renamed", "midnight").is_err(),
        "not under the folder's name"
    );
    assert!(plugins.theme_css("com.example.midnight", "midnight").is_err());

    plugins.remove("renamed").unwrap();
    assert!(plugins.list().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn a_file_that_is_a_link_out_of_the_package_is_refused() {
    let dir = temp();
    let source = package(&dir, "com.example.midnight", API);
    let secret = dir.join("secret.txt");
    std::fs::write(&secret, "not the plugin's").unwrap();
    std::fs::remove_file(source.join("midnight.css")).unwrap();
    std::os::unix::fs::symlink(&secret, source.join("midnight.css")).unwrap();

    let plugins = Plugins::new(&dir);
    let failure = plugins.install(&source).unwrap_err();
    assert!(format!("{failure:?}").contains("not a plain file"), "{failure:?}");
    assert!(plugins.list().unwrap().is_empty());
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
fn a_package_edited_by_hand_is_seen_once_the_list_is_refreshed() {
    let dir = temp();
    let plugins = Plugins::new(&dir);
    assert!(plugins.list().unwrap().is_empty());

    // Copied into the folder without going through install.
    let by_hand = dir.join(FOLDER).join("com.example.midnight");
    std::fs::create_dir_all(&by_hand).unwrap();
    let source = package(&dir, "com.example.midnight", API);
    std::fs::copy(source.join(MANIFEST), by_hand.join(MANIFEST)).unwrap();
    std::fs::copy(source.join("midnight.css"), by_hand.join("midnight.css")).unwrap();

    assert!(
        plugins.list().unwrap().is_empty(),
        "one reading of the folder serves every step"
    );
    plugins.refresh();
    assert_eq!(plugins.list().unwrap().len(), 1);
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
