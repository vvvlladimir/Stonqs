//! Running a plugin's file writer (ADR-0080): a `stonqs.transactions` document in, the bytes of
//! another format out. What the module may reach is `sandbox.rs`, shared with the reader.
//!
//! A writer sees what the host hands it and nothing else — no store, no portfolio, no path. The
//! host writes the bytes it returns where the user chose.

use super::sandbox;
use crate::error::{UiError, UiResult};
use std::path::Path;

mod wit {
    wasmtime::component::bindgen!({
        path: "wit/writer.wit",
        world: "writer",
    });
}

/// Runs one writer over one document. Every failure — a module that does not start, a trap, the
/// deadline, the memory ceiling, or the writer's own refusal — is one `Err`, carrying why.
pub fn write(module: &Path, canonical: &str) -> Result<Vec<u8>, String> {
    sandbox::run(module, |store, component, linker| {
        let writer = wit::Writer::instantiate(&mut *store, component, linker)
            .map_err(|e| format!("the module did not start: {e}"))?;
        writer
            .call_write(&mut *store, canonical)
            .map_err(|e| format!("the module stopped: {e}"))
    })?
}

/// What a writer must prove before the package carrying it is installed: its sample is the app's
/// own transaction file, and writing it produces exactly the bytes the package says it does. The
/// comparison is byte for byte — unlike a reader's, the output *is* bytes, and a file another
/// program reads is judged by that program on every byte of it.
pub fn check(id: &str, module: &Path, sample: &[u8], expected: &[u8]) -> UiResult<()> {
    sq_core::import::parse_canonical(sample)
        .map_err(|e| UiError::invalid(format!("writer {id}: its sample is not a transaction file: {e}")))?;
    let sample = std::str::from_utf8(sample)
        .map_err(|e| UiError::invalid(format!("writer {id}: its sample is not text: {e}")))?;
    let written = write(module, sample).map_err(|message| UiError::Writer {
        plugin: id.to_string(),
        message,
    })?;
    if written != expected {
        return Err(UiError::invalid(format!(
            "writer {id} does not write its own sample the way the package says it does"
        )));
    }
    Ok(())
}
