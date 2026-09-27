//! Running a plugin's file reader (ADR-0086), once, in `import_load`; it touches no store.

use super::sandbox;
use crate::error::{UiError, UiResult};
use std::path::Path;

/// The generated side of the contract, kept in a module of its own: the world's `reading` and
/// `warning` become Rust types with the names this file also wants for the host's own.
mod wit {
    wasmtime::component::bindgen!({
        path: "wit/reader.wit",
        world: "reader",
    });
}

use wit::{FileHints, ReadError};

/// What a reader produced, in the host's own words.
#[derive(Debug, Clone)]
pub struct Reading {
    /// A `stonqs.transactions` document (ADR-0066), as the reader wrote it. Unvalidated here —
    /// `parse_canonical` is what reads it, exactly as it reads one the user brought themselves.
    pub canonical: String,
    pub warnings: Vec<ReaderWarning>,
}

/// A reader's own warning. Its `code` is a stranger's vocabulary, so it travels beside the
/// sentence rather than instead of it: the app has no table to translate it from.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ReaderWarning {
    /// Which plugin's reader said it.
    pub plugin: String,
    pub row: Option<u32>,
    pub code: String,
    pub message: String,
}

/// Why a reader did not produce a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Not a file this reader knows. The only one that is not a failure: the host moves on.
    NotMine,
    /// Sealed, and the password was absent or wrong.
    NeedsPassword,
    /// Recognised and unreadable: the reader said so, in its own words.
    Malformed(String),
    /// The module failed before claiming the file, so the host moves past it.
    Broken(String),
}

/// A reader that broke over a file and was passed over, so the wizard can say which and why. The
/// detail is the runtime's English, a developer's line: the sentence around it is the frontend's.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SkippedReader {
    /// `<plugin id>/<reader id>`.
    pub plugin: String,
    pub detail: String,
}

impl Refusal {
    pub fn into_error(self, plugin: &str) -> UiError {
        match self {
            Refusal::NotMine => UiError::invalid(format!("reader {plugin} does not read this file")),
            Refusal::NeedsPassword => UiError::FileProtected {
                reader: plugin.to_string(),
                tried: false,
                message: format!("the file is protected and reader {plugin} needs its password"),
            },
            Refusal::Malformed(why) | Refusal::Broken(why) => UiError::Reader {
                plugin: plugin.to_string(),
                message: why,
            },
        }
    }
}

/// `module` is already `safe_join`ed. Every host-side failure is `Broken`; only the reader's own
/// `malformed` is `Malformed`.
pub fn read(
    module: &Path,
    bytes: &[u8],
    file_name: &str,
    password: Option<&str>,
) -> Result<Reading, Refusal> {
    let outcome = sandbox::run(module, |store, component, linker| {
        let reader = wit::Reader::instantiate(&mut *store, component, linker)
            .map_err(|e| format!("the module did not start: {e}"))?;
        let hints = FileHints {
            file_name: file_name.to_string(),
            password: password.map(str::to_string),
        };
        reader
            .call_read(&mut *store, bytes, &hints)
            .map_err(|e| format!("the module stopped: {e}"))
    })
    .map_err(Refusal::Broken)?;

    match outcome {
        Ok(reading) => Ok(Reading {
            canonical: reading.canonical,
            warnings: reading
                .warnings
                .into_iter()
                .map(|w| ReaderWarning {
                    plugin: String::new(),
                    row: w.row,
                    code: w.code,
                    message: w.message,
                })
                .collect(),
        }),
        Err(ReadError::NotMine) => Err(Refusal::NotMine),
        Err(ReadError::NeedsPassword) => Err(Refusal::NeedsPassword),
        Err(ReadError::Malformed(why)) => Err(Refusal::Malformed(why)),
    }
}

/// A reader must recognise its sample and produce exactly the expected document: a misread column
/// would otherwise look perfectly correct.
pub fn check(id: &str, module: &Path, sample: &[u8], sample_name: &str, expected: &str) -> UiResult<()> {
    let reading = read(module, sample, sample_name, None).map_err(|refusal| match refusal {
        Refusal::NotMine => UiError::invalid(format!("reader {id} does not recognise the sample it ships")),
        other => other.into_error(id),
    })?;

    sq_core::import::parse_canonical(reading.canonical.as_bytes())
        .map_err(|e| UiError::invalid(format!("reader {id} produced no readable transaction file: {e}")))?;

    // Compared as documents rather than as text: how a reader spaces its JSON is not a promise
    // it made, and the operations are.
    let produced: serde_json::Value = serde_json::from_str(&reading.canonical)
        .map_err(|e| UiError::invalid(format!("reader {id}: {e}")))?;
    let wanted: serde_json::Value = serde_json::from_str(expected)
        .map_err(|e| UiError::invalid(format!("reader {id}: its own expectation is not readable: {e}")))?;
    if produced != wanted {
        return Err(UiError::invalid(format!(
            "reader {id} does not read its own sample the way the package says it does"
        )));
    }
    Ok(())
}
