//! What the app is, legally: the licence notices of everything it was built from. The file is
//! generated into the repository root (`scripts/notices.mjs`) and compiled in, because MIT, BSD
//! and ISC ask that their text travel with a compiled copy and not only with the source.

use crate::error::{UiError, UiResult};

/// Every dependency's licence text, as markdown. Compiled in rather than read at runtime — there
/// is no directory to find beside the binary on any of the platforms this ships to.
const NOTICES: &str = include_str!("../../../../THIRD-PARTY-NOTICES.md");

/// Write the notices where the user picked. They are a megabyte of text, so they are saved rather
/// than sent over IPC and rendered; the app lists the packages from its own bundle.
#[tauri::command]
pub fn notices_save(path: String) -> UiResult<()> {
    std::fs::write(&path, NOTICES).map_err(|e| UiError::invalid(format!("cannot write {path}: {e}")))
}
