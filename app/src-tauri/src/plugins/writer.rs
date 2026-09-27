//! Running a plugin's file writer (ADR-0080): a canonical document in, bytes out; no store, no path.

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

/// The sample must be written exactly as expected, byte for byte: the output is bytes another program judges.
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
