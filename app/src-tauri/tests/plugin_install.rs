//! The shipped example plugin, installed the way the app installs one.
//!
//! It is the format's documentation, so it has to be a package this build actually accepts —
//! a README that does not install is worse than none.

use sq_app_lib::plugins::{Base, Plugins, Status};
use std::path::Path;

#[test]
fn the_example_theme_installs_and_is_offered() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/midnight");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    let plugins = Plugins::new(&dir);
    let installed = plugins.install(&example).unwrap();
    assert_eq!(installed.status, Status::Ok, "the example is built for this API");

    let themes = plugins.themes().unwrap();
    assert_eq!(themes.len(), 1);
    assert_eq!(themes[0].key, "app.stonqs.midnight/midnight");
    assert_eq!(themes[0].base, Base::Dark);

    let css = plugins.theme_css("app.stonqs.midnight", "midnight").unwrap();
    assert!(
        css.contains(":root"),
        "a theme redefines the app's own properties"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_example_layout_installs_and_reads_its_own_sample() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/northbay");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    let plugins = Plugins::new(&dir);
    // Installing is the check: a layout that does not read its own sample is refused here.
    let installed = plugins.install(&example).unwrap();
    assert_eq!(installed.status, Status::Ok);

    let layouts = plugins.layouts().unwrap();
    assert_eq!(layouts.len(), 1);
    assert_eq!(layouts[0].0, "app.stonqs.northbay/northbay");
    assert_eq!(layouts[0].1.name, "Northbay Securities");

    // And the wizard recognises the sample as *this* layout: fitting no better than a shipped one
    // would be a tie, which is no answer at all, and the example would teach the wrong lesson.
    let sample = std::fs::read(example.join("sample.csv")).unwrap();
    let headers: Vec<String> = String::from_utf8_lossy(&sample)
        .lines()
        .next()
        .unwrap()
        .split(',')
        .map(str::to_string)
        .collect();
    let recognised = sq_app_lib::import_templates::match_for(
        &dir.join("portfolio.db"),
        &plugins,
        &headers,
        Some("sample.csv"),
        &String::from_utf8_lossy(&sample),
    )
    .expect("the file is recognised as one layout");
    assert_eq!(recognised.id, "app.stonqs.northbay/northbay");

    std::fs::remove_dir_all(&dir).ok();
}
