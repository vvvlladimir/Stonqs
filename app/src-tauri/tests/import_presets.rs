//! The two kinds of import layout in one list: the ones the user made and the ones the app
//! ships. Both are removable, but only one of them is removable for good.

use sq_app_lib::import_templates::{
    TemplateSource, delete_template, listing, restore_presets, save_template,
};
use sq_core::import::{ImportField, ImportMapping, ParseConfig, builtin_presets};
use std::path::PathBuf;

/// A directory of its own per test: these functions write files beside the database.
fn db_path(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("vs-presets-{test}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for file in ["import_templates.json", "import_presets_hidden.json"] {
        let _ = std::fs::remove_file(dir.join(file));
    }
    dir.join("portfolio.sqlite")
}

#[test]
fn the_users_own_layouts_come_before_the_shipped_ones() {
    let path = db_path("order");
    assert_eq!(listing(&path).len(), builtin_presets().len());
    assert!(listing(&path).iter().all(|t| t.source == TemplateSource::Builtin));

    // A name sorting after every broker still comes first: the grouping decides, not the name.
    let mapping = ImportMapping::detect(&["date".to_string(), "type".to_string()]);
    save_template(&path, "Zzz Invest", ParseConfig::default(), mapping).unwrap();
    let list = listing(&path);
    assert_eq!(list[0].name, "Zzz Invest");
    assert_eq!(list[0].source, TemplateSource::User);
    assert_eq!(list.len(), builtin_presets().len() + 1);
}

#[test]
fn a_shipped_preset_can_be_removed_and_brought_back() {
    let path = db_path("remove");
    let name = builtin_presets()[0].name.clone();

    let list = delete_template(&path, &name).unwrap();
    assert!(!list.iter().any(|t| t.name == name));
    assert_eq!(list.len(), builtin_presets().len() - 1);
    // Removal outlives the call: it is written down, not performed.
    assert!(!listing(&path).iter().any(|t| t.name == name));

    let list = restore_presets(&path).unwrap();
    assert!(list.iter().any(|t| t.name == name));
}

#[test]
fn saving_over_a_shipped_name_replaces_it_instead_of_doubling_it() {
    let path = db_path("shadow");
    let name = builtin_presets()[0].name.clone();

    let mine = ImportMapping::detect(&["Дата".to_string(), "Операция".to_string()]);
    let list = save_template(&path, &name, ParseConfig::default(), mine).unwrap();
    assert_eq!(list.iter().filter(|t| t.name == name).count(), 1);
    let mine = list.iter().find(|t| t.name == name).unwrap();
    assert_eq!(mine.source, TemplateSource::User);
    assert_eq!(mine.mapping.column(ImportField::Date), Some("Дата"));

    // Deleting the copy is not deleting the shipped layout underneath it.
    let list = delete_template(&path, &name).unwrap();
    let back = list.iter().find(|t| t.name == name).unwrap();
    assert_eq!(back.source, TemplateSource::Builtin);
}

#[test]
fn a_shipped_preset_arrives_with_the_wordings_every_broker_shares() {
    let preset = builtin_presets()
        .iter()
        .find(|p| p.name == "Trade Republic")
        .expect("Trade Republic preset");
    let mapping = preset.mapping();

    assert_eq!(mapping.column(ImportField::Date), Some("date"));
    assert_eq!(mapping.column(ImportField::Kind), Some("type"));
    assert_eq!(preset.config.delimiter, Some(','));
    // The file lists only what is peculiar to this broker; the rest is merged in on use.
    assert_eq!(mapping.kind_of("BUY"), Some(sq_core::model::TransactionKind::Buy));
    // A card payment is money leaving the portfolio, and nothing shared says so.
    assert_eq!(
        mapping.kind_of("CARD_TRANSACTION"),
        Some(sq_core::model::TransactionKind::Withdrawal)
    );
    // The broker's older statement keeps its own entry rather than being overwritten.
    let statement = builtin_presets()
        .iter()
        .find(|p| p.name == "Trade Republic · statement")
        .expect("Trade Republic statement preset");
    assert_eq!(statement.mapping().column(ImportField::Date), Some("Datum"));
    assert_eq!(statement.config.delimiter, Some(';'));
}
