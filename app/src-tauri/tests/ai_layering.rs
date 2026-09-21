//! The seam inside `ai/`: the neutral types, the provider adapter, the wire reader, the history
//! layer and the loop are all written without knowing Tauri exists, so moving them into a crate
//! of their own later is a `git mv` rather than a rewrite (ADR-0037). The same
//! trick `.claude/rules/ui-boundary.md` plays with `grep -r tauri core/`.
//!
//! `tools/` names `state::ScopeSelection`, which is a plain snapshot — a portfolio and the
//! accounts in view — and would travel with these files rather than hold them back.
//!
//! The tree is walked rather than listed: a file added under `ai/` is covered the day it is
//! written, which a hand-kept list only manages until somebody forgets.

use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}

/// Comments are stripped first: these files are allowed to *explain* the boundary, and several
/// of them do — what must not appear is a use of it.
fn code(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(before, _)| before))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_provider_layer_does_not_know_tauri_exists() {
    let mut files = Vec::new();
    rust_files(Path::new("src/ai"), &mut files);
    assert!(files.len() > 10, "the walk found almost nothing: {files:?}");

    for path in files {
        let source = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(
            !code(&source).contains("tauri"),
            "{} names tauri; a command belongs in commands/ai.rs, not here",
            path.display()
        );
    }
}
