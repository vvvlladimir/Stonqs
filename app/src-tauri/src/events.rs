//! Frontend notifications for changed data.

use crate::error::UiResult;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// Changed data scope used to invalidate frontend queries.
#[derive(Debug, Clone, Serialize)]
pub struct DataChanged {
    pub scope: &'static str,
}

pub fn emit_changed(app: &AppHandle, scope: &'static str) -> UiResult<()> {
    app.emit("data:changed", DataChanged { scope })?;
    Ok(())
}
