//! What each kind of content must prove before its package is installed. Anything that computes
//! is run against the sample it ships and compared with the answer it ships.

use super::manifest::{MANIFEST, Provides, safe_join};
use super::{reader, tool, widget, writer};
use crate::error::{UiError, UiResult};
use crate::import_templates::{check_dictionary, check_layout};
use std::path::Path;

/// Runs every check in `provides`; the first failure refuses the whole package.
pub(super) fn content(source: &Path, provides: &Provides) -> UiResult<()> {
    let read = |file: &str| {
        std::fs::read(safe_join(source, file)?).map_err(|e| UiError::invalid(format!("{file}: {e}")))
    };
    let text = |file: &str| {
        std::fs::read_to_string(safe_join(source, file)?)
            .map_err(|e| UiError::invalid(format!("{file}: {e}")))
    };

    // Same promise as the shipped layouts' fixtures, for a layout nobody here has seen.
    for def in &provides.layouts {
        check_layout(&def.id, &text(&def.file)?, &read(&def.sample)?, &def.sample)?;
    }
    for def in &provides.readers {
        let (sample, expected) = (read(&def.sample)?, text(&def.expected)?);
        reader::check(
            &def.id,
            &safe_join(source, &def.file)?,
            &sample,
            &def.sample,
            &expected,
        )?;
    }
    for def in &provides.writers {
        check_extension(&def.id, &def.extension)?;
        let (sample, expected) = (read(&def.sample)?, read(&def.expected)?);
        writer::check(&def.id, &safe_join(source, &def.file)?, &sample, &expected)?;
    }
    // A page draws rather than answers, so only its shape is checked.
    for def in &provides.widgets {
        widget::check(def, &read(&def.file)?)?;
    }
    for def in &provides.screens {
        widget::check_screen(def, &read(&def.file)?)?;
    }
    // The schema too: one a provider refuses breaks every chat, not just this tool.
    for def in &provides.tools {
        let module = safe_join(source, &def.file)?;
        tool::check(
            def,
            &module,
            &read(&def.schema)?,
            &read(&def.sample)?,
            &read(&def.expected)?,
        )?;
    }
    // The file is the data, so there is no expectation to compare with.
    for def in &provides.taxonomies {
        taxonomy(&def.id, &read(&def.file)?)?;
    }
    // Its sample must be one the app cannot read alone.
    for def in &provides.dictionaries {
        check_dictionary(&def.id, &text(&def.file)?, &read(&def.sample)?)?;
    }
    Ok(())
}

/// A writer's ending becomes a save dialog's filter, so it is letters and digits only.
fn check_extension(id: &str, extension: &str) -> UiResult<()> {
    if extension.is_empty() || extension.len() > 12 || !extension.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(UiError::invalid(format!(
            "writer {id}: extension {extension:?} is not letters and digits without the dot"
        )));
    }
    Ok(())
}

/// Reads as a tree and leaves nothing invalid — checked against no securities on purpose.
fn taxonomy(id: &str, csv: &[u8]) -> UiResult<()> {
    use sq_core::import::{ParseConfig, Severity, build_taxonomy_preview, detect_taxonomy_config, parse_csv};

    let parsed = parse_csv(csv, &ParseConfig::default())
        .map_err(|e| UiError::invalid(format!("classification set {id}: {e}")))?;
    let config = detect_taxonomy_config(&parsed);
    let preview = build_taxonomy_preview(&parsed, &config, &[], None);

    if preview.nodes.is_empty() {
        return Err(UiError::invalid(format!(
            "classification set {id} reads as no tree at all"
        )));
    }
    if let Some(problem) = preview.problems.iter().find(|p| p.severity == Severity::Error) {
        return Err(UiError::invalid(format!(
            "classification set {id} does not read: {}",
            problem.message
        )));
    }
    Ok(())
}

/// Every named file must be a plain file: a symlink would have the checks and the copy read
/// whatever it points at. A missing one is reported by the check that needs it.
pub(super) fn regular_files(source: &Path, provides: &Provides) -> UiResult<()> {
    for file in std::iter::once(MANIFEST).chain(provides.files()) {
        let path = safe_join(source, file)?;
        if let Ok(meta) = std::fs::symlink_metadata(&path)
            && !meta.file_type().is_file()
        {
            return Err(UiError::invalid(format!(
                "{file} is not a plain file inside the plugin"
            )));
        }
    }
    Ok(())
}
