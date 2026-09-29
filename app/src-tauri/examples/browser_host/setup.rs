//! The profile the browser host serves: a throwaway folder, a demo portfolio dated backwards from
//! today, and the first-run questions already answered so no dialog covers the screen.

use sq_app_lib::commands::{demo, profiles};
use sq_app_lib::state::AppState;
use sq_app_lib::{commands::sources, settings};
use std::path::{Path, PathBuf};
use tauri::test::MockRuntime;
use tauri::{App, Manager};

/// One folder per port, wiped on start: two hosts never share one, and a rerun leaves no litter.
pub fn data_dir(port: &str) -> PathBuf {
    std::env::temp_dir().join(format!("stonqs-browser-host-{}", port))
}

/// No window is opened: the mock runtime exists only so the commands can be handed a `State`.
pub fn demo_app(dir: &Path) -> App<MockRuntime> {
    let _ = std::fs::remove_dir_all(dir);
    let app = tauri::test::mock_app();
    app.manage(AppState::at(dir).expect("the browser host's data folder"));
    let state = app.state::<AppState>();
    // Not the adopted first profile: that id is the one whose AI keys live in the keychain.
    let profile = profiles::profile_create(state.clone(), "Browser demo".into()).expect("a profile");
    state.open_profile(&profile.id).expect("the new profile opens");
    demo::demo_seed(state.clone()).expect("the demo seeds");
    sources::market_sources_confirm(state.clone()).expect("the sources question is answered");
    settings::ui_state_save(state.clone(), serde_json::json!({ "tour": { "done": true } }))
        .expect("the tour offer is answered");
    app
}
