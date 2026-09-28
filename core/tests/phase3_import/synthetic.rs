//! Every shipped layout, over a file written from its own declaration. A conformance fixture is
//! a real export and worth more; there are 29 layouts and one fixture, so this is what stands
//! between the other 28 and a user whose broker is one of them.
//!
//! The file is generated from the layout itself — its columns, its delimiter, its date format,
//! its decimal separator, its wordings — so what it proves is narrow and worth stating: a layout
//! that cannot read a file laid out exactly as it says cannot read its broker's either. It does
//! not prove the layout matches what the broker actually prints. Only `conformance.rs` does that.

use super::*;
use sq_core::import::{BrokerPreset, ImportField, builtin_presets};
use sq_core::model::TransactionKind;

/// A cell for one field, in the shape the layout says its file is written in. A row whose
/// operation carries no instrument leaves the instrument columns empty, the way the broker does:
/// a deposit naming a ticker is a different file than the one this layout describes.
fn cell(field: ImportField, kind: &str, preset: &BrokerPreset, row: usize, holds: bool) -> String {
    if !holds
        && matches!(
            field,
            ImportField::Symbol
                | ImportField::Isin
                | ImportField::Name
                | ImportField::Quantity
                | ImportField::Price
        )
    {
        return String::new();
    }
    let decimal = preset.config.decimal_separator.unwrap_or('.');
    let money = |v: &str| v.replace('.', &decimal.to_string());
    let date = preset.config.date_format.as_deref().unwrap_or("%Y-%m-%d");
    // Noon rather than a bare date: a layout whose format carries a time (`%H:%M:%S`) cannot
    // render one from a date alone, and chrono says so by failing to write rather than by
    // returning an error.
    let day = chrono::NaiveDate::from_ymd_opt(2024, 1, (row % 28 + 1) as u32)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    match field {
        ImportField::Date => {
            use std::fmt::Write;
            let mut text = String::new();
            match write!(text, "{}", day.format(date)) {
                Ok(()) => text,
                Err(_) => day.date().to_string(),
            }
        }
        ImportField::Kind => kind.to_string(),
        ImportField::Symbol => "AAPL".into(),
        ImportField::Isin => "US0378331005".into(),
        ImportField::Name => "Apple Inc".into(),
        ImportField::Quantity => "10".into(),
        ImportField::Price => money("100.00"),
        ImportField::Amount => money("1000.00"),
        ImportField::Fee => money("1.00"),
        ImportField::Tax => money("0.50"),
        ImportField::Currency | ImportField::FeeCurrency | ImportField::TaxCurrency => preset
            .mapping
            .default_currency
            .clone()
            .unwrap_or_else(|| "USD".into()),
        ImportField::FxRate => money("1.00"),
        ImportField::Account => "Main".into(),
        // One id per row: a repeated one is a link, and every row claiming one link would read
        // the whole file as internal transfers.
        ImportField::LinkId | ImportField::ExternalId => format!("REF{row:04}"),
        ImportField::Note => "note".into(),
    }
}

/// A cell as a file carries it: quoted when it holds the delimiter, which several layouts
/// cannot avoid — a decimal comma in a comma-separated file, a date whose format has one in it.
fn quoted(value: &str, delimiter: char) -> String {
    if value.contains(delimiter) || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// A file laid out exactly as the layout declares, one row per wording it knows.
fn file_for(preset: &BrokerPreset) -> Vec<u8> {
    let delimiter = preset.config.delimiter.unwrap_or(',');
    let fields: Vec<ImportField> = ImportField::ALL
        .iter()
        .copied()
        .filter(|f| preset.mapping.columns.contains_key(f))
        .collect();
    let header: Vec<String> = fields
        .iter()
        .map(|f| quoted(&preset.mapping.columns[f], delimiter))
        .collect();
    // The service rows the layout says to cut, so the header lands where it expects it.
    let mut out = String::new();
    for i in 0..preset.config.skip_top_rows {
        out.push_str(&format!("statement line {}\n", i + 1));
    }
    out.push_str(&header.join(&delimiter.to_string()));
    // The layout's own wordings, not the shared ones: a shipped list is what this checks.
    let wordings: Vec<TransactionKind> = preset.mapping.kind_aliases.values().copied().collect();
    let wordings: Vec<(String, TransactionKind)> = preset
        .mapping
        .kind_aliases
        .keys()
        .cloned()
        .zip(wordings)
        .collect();
    for (row, (kind, means)) in wordings.iter().enumerate() {
        let holds = means.requires_security();
        let cells: Vec<String> = fields
            .iter()
            .map(|f| quoted(&cell(*f, kind, preset, row, holds), delimiter))
            .collect();
        out.push('\n');
        out.push_str(&cells.join(&delimiter.to_string()));
    }
    out.push('\n');
    out.into_bytes()
}

/// A layout that names no date column, or no wording at all, has nothing to generate from.
fn testable(preset: &BrokerPreset) -> bool {
    preset.mapping.columns.contains_key(&ImportField::Date) && !preset.mapping.kind_aliases.is_empty()
}

/// The whole journey per layout: its own file, its own mapping, nothing asked of a human.
#[test]
fn every_shipped_layout_imports_a_file_written_the_way_it_says() {
    let mut broken = Vec::new();
    for preset in builtin_presets() {
        if !testable(preset) {
            continue;
        }
        let bytes = file_for(preset);
        let currency = preset
            .mapping
            .default_currency
            .clone()
            .unwrap_or_else(|| "USD".into());
        let store = Store::open_in_memory().unwrap();
        let account = depot(&store, "Broker", &currency);
        let mapping = preset.mapping().with_account(&account.id);
        let service = ImportService::new(&store);

        let preview = match service.preview(&bytes, &preset.config, Some(&mapping), &[]) {
            Ok(p) => p,
            Err(e) => {
                broken.push(format!(
                    "{}: the file it declares cannot be read: {e}",
                    preset.name
                ));
                continue;
            }
        };
        let unknown = preview.unknown_kinds();
        if !unknown.is_empty() {
            broken.push(format!(
                "{}: its own wordings come back unread: {unknown:?}",
                preset.name
            ));
        }
        if preview.summary.invalid > 0 {
            let why: Vec<String> = preview
                .rows
                .iter()
                .filter(|r| r.status == RowStatus::Invalid)
                .flat_map(|r| r.problems.iter().map(|p| format!("{:?} {}", p.code, p.message)))
                .take(3)
                .collect();
            broken.push(format!(
                "{}: {} of {} rows cannot be written — {}",
                preset.name,
                preview.summary.invalid,
                preview.summary.total,
                why.join("; ")
            ));
        }
        if let Err(e) = service.commit(
            &preview,
            &ImportOptions {
                new_security_source: Some("yahoo".into()),
                ..ImportOptions::default()
            },
        ) {
            broken.push(format!("{}: the commit failed: {e}", preset.name));
        }
    }
    assert!(broken.is_empty(), "\n{}", broken.join("\n"));
}

/// Every wording a layout ships must mean an operation this app has. A typo in the data file is
/// otherwise a question put to the user for every row carrying that wording.
#[test]
fn a_layout_never_ships_a_wording_it_cannot_resolve() {
    let mut unresolved = Vec::new();
    for preset in builtin_presets() {
        let merged = preset.mapping();
        for wording in preset.mapping.kind_aliases.keys() {
            if !merged.kind_aliases.contains_key(wording) {
                unresolved.push(format!("{}: {wording}", preset.name));
            }
        }
        // A wording cannot be both mapped and skipped: the wizard would show it twice.
        for skipped in &preset.mapping.ignored_kinds {
            if preset.mapping.kind_aliases.contains_key(skipped) {
                unresolved.push(format!("{}: {skipped} is both mapped and ignored", preset.name));
            }
        }
    }
    assert!(unresolved.is_empty(), "\n{}", unresolved.join("\n"));
}
