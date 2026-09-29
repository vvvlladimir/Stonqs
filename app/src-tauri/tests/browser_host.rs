//! The browser host (`examples/browser_host`) answers the frontend in a plain browser with the real
//! command functions. Two ways it could quietly stop doing that are checked here, without a browser.

#[path = "../examples/browser_host/routes.rs"]
mod routes;
#[path = "../examples/browser_host/setup.rs"]
mod setup;

use sq_app_lib::state::AppState;
use std::collections::BTreeSet;
use tauri::Manager;

/// The names in `generate_handler!`, read off `lib.rs` the way the handler itself reads them.
fn registered() -> BTreeSet<String> {
    let lib = include_str!("../src/lib.rs");
    let start = lib
        .find("tauri::generate_handler![")
        .expect("the command handler");
    let end = start + lib[start..].find(']').expect("the end of the handler");
    lib[start..end]
        .lines()
        .skip(1)
        .filter_map(|line| line.trim().strip_suffix(','))
        .filter_map(|path| path.rsplit("::").next())
        .map(str::to_string)
        .collect()
}

/// A new command has to be sorted into one list or the other: routed, so the browser (and e2e)
/// sees it, or named desktop-only with the reason. Left out, a screen asking for it fails by name.
#[test]
fn every_command_is_either_routed_or_desktop_only() {
    let registered = registered();
    assert!(registered.len() > 100, "lib.rs no longer reads as a command list");
    let routed: BTreeSet<String> = routes::ROUTED.iter().map(|s| s.to_string()).collect();
    let desktop: BTreeSet<String> = routes::DESKTOP_ONLY.iter().map(|(s, _)| s.to_string()).collect();

    let unsorted: Vec<_> = registered
        .difference(&routed)
        .filter(|c| !desktop.contains(*c))
        .collect();
    assert!(unsorted.is_empty(), "sort into routes.rs: {unsorted:?}");
    let both: Vec<_> = routed.intersection(&desktop).collect();
    assert!(both.is_empty(), "routed and desktop-only at once: {both:?}");
    let stale: Vec<_> = routed
        .union(&desktop)
        .filter(|c| !registered.contains(*c))
        .collect();
    assert!(stale.is_empty(), "no longer registered: {stale:?}");
}

/// The profile the browser sees: a demo with accounts, and a command answering through the routes.
#[test]
fn the_host_serves_a_seeded_demo() {
    let dir = setup::data_dir("test");
    let app = setup::demo_app(&dir);
    let state = app.state::<AppState>();
    let status = routes::answer(&state, "app_status", &serde_json::json!({}))
        .expect("app_status is routed")
        .expect("app_status answers");
    assert!(
        status["account_count"].as_u64().unwrap() > 0,
        "the demo seeded no account"
    );
    let _ = std::fs::remove_dir_all(dir);
}

/// The host answers a repeated read from memory, which is only right if asking twice gives the same
/// answer. Checked here with the arguments each read accepts empty; a read that fails that way
/// (it needs an id) is not asked.
#[test]
fn a_read_asked_twice_answers_the_same() {
    let dir = setup::data_dir("test-reads");
    let app = setup::demo_app(&dir);
    let state = app.state::<AppState>();
    let today = chrono::Local::now().date_naive().to_string();
    let args =
        serde_json::json!({ "date": today, "asOf": today, "from": "2024-01-01", "to": today, "filter": {} });
    let mut asked = 0;
    for read in routes::READS {
        let first = routes::answer(&state, read, &args).expect("a read is routed");
        let Ok(first) = first else { continue };
        let second = routes::answer(&state, read, &args).expect("a read is routed");
        assert_eq!(Ok(first), second, "{read} answered differently the second time");
        asked += 1;
    }
    assert!(asked > 30, "only {asked} reads answered with generic arguments");
    let _ = std::fs::remove_dir_all(dir);
}
