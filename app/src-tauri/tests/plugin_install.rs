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

    let css = plugins
        .theme_css("app.stonqs.midnight", "midnight")
        .unwrap();
    assert!(css.contains(":root"), "a theme redefines the app's own properties");

    std::fs::remove_dir_all(&dir).ok();
}
