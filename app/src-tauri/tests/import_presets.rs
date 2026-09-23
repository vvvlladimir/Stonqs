//! The two kinds of import layout in one list: the ones the user made and the ones the app
//! ships. Both are removable, but only one of them is removable for good.

use sq_app_lib::import_templates::{
    TemplateSource, delete_template, listing, restore_presets, save_template,
};
use sq_app_lib::plugins::Plugins;
use sq_core::import::{ImportField, ImportMapping, ParseConfig, builtin_presets};
use std::path::PathBuf;

/// A directory of its own per test: these functions write files beside the database.
/// No plugin is installed in these tests: the two kinds here are the user's and the app's.
fn no_plugins() -> Plugins {
    Plugins::new(std::env::temp_dir().join("vs-presets-no-plugins"))
}

/// The id the list currently shows this name under — the identity a command is given, so that a
/// name printed by two sources cannot decide which one is removed.
fn id_of(path: &std::path::Path, name: &str) -> String {
    listing(path, &no_plugins())
        .into_iter()
        .find(|t| t.name == name)
        .expect("a layout by that name")
        .id
}

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
    assert_eq!(listing(&path, &no_plugins()).len(), builtin_presets().len());
    assert!(
        listing(&path, &no_plugins())
            .iter()
            .all(|t| t.source == TemplateSource::Builtin)
    );

    // A name sorting after every broker still comes first: the grouping decides, not the name.
    let mapping = ImportMapping::detect(&["date".to_string(), "type".to_string()]);
    save_template(
        &path,
        &no_plugins(),
        "Zzz Invest",
        ParseConfig::default(),
        mapping,
    )
    .unwrap();
    let list = listing(&path, &no_plugins());
    assert_eq!(list[0].name, "Zzz Invest");
    assert_eq!(list[0].source, TemplateSource::User);
    assert_eq!(list.len(), builtin_presets().len() + 1);
}

#[test]
fn a_shipped_preset_can_be_removed_and_brought_back() {
    let path = db_path("remove");
    let name = builtin_presets()[0].name.clone();

    let shipped = id_of(&path, &name);
    let list = delete_template(&path, &no_plugins(), &shipped).unwrap();
    assert!(!list.iter().any(|t| t.name == name));
    assert_eq!(list.len(), builtin_presets().len() - 1);
    // Removal outlives the call: it is written down, not performed.
    assert!(!listing(&path, &no_plugins()).iter().any(|t| t.name == name));

    let list = restore_presets(&path, &no_plugins()).unwrap();
    assert!(list.iter().any(|t| t.name == name));
}

#[test]
fn saving_over_a_shipped_name_replaces_it_instead_of_doubling_it() {
    let path = db_path("shadow");
    let name = builtin_presets()[0].name.clone();

    let mine = ImportMapping::detect(&["Дата".to_string(), "Операция".to_string()]);
    let list = save_template(&path, &no_plugins(), &name, ParseConfig::default(), mine).unwrap();
    assert_eq!(list.iter().filter(|t| t.name == name).count(), 1);
    let mine = list.iter().find(|t| t.name == name).unwrap();
    assert_eq!(mine.source, TemplateSource::User);
    assert_eq!(mine.mapping.column(ImportField::Date), Some("Дата"));

    // Deleting the copy is not deleting the shipped layout underneath it: they are two ids.
    let list = delete_template(&path, &no_plugins(), &id_of(&path, &name)).unwrap();
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

/// A layout that arrives as a plugin: it proves itself against the sample it ships, joins the
/// one list the wizard reads, and is not removable from there — the plugin is what installed it.
mod from_a_plugin {
    use super::*;
    use sq_app_lib::plugins::Plugins;

    /// Real Trade Republic headers, the same sample shape the shipped layouts are checked with.
    const SAMPLE: &str = "\
date,type,symbol,name,shares,price,amount,fee,currency
2025-06-13,BUY,IE00B5BMR087,Core S&P 500,0.36,552.56,-199.99,-1.00,EUR
2025-06-13,CUSTOMER_INPAYMENT,,,,,1750.00,,EUR
";

    fn layout(kind_aliases: &str) -> String {
        format!(
            r#"{{"name":"Example Broker",
                 "config":{{"delimiter":",","skip_top_rows":0,"skip_bottom_rows":0,
                            "has_header":true,"date_format":"%Y-%m-%d",
                            "decimal_separator":"."}},
                 "mapping":{{"account_id":null,
                             "columns":{{"DATE":"date","KIND":"type","SYMBOL":"symbol",
                              "NAME":"name","QUANTITY":"shares","PRICE":"price",
                              "AMOUNT":"amount","FEE":"fee","CURRENCY":"currency"}},
                             "kind_aliases":{kind_aliases},"ignored_kinds":[],
                             "symbol_aliases":{{}},"account_aliases":{{}},
                             "default_currency":null,"amount_sign":"SIGNED",
                             "new_securities":{{}}}}}}"#
        )
    }

    fn package(dir: &std::path::Path, aliases: &str) -> PathBuf {
        let source = dir.join("package");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            source.join("plugin.json"),
            r#"{"id":"com.example.broker","api":1,"name":"Example Broker","version":"1.0.0",
                "provides":{"layouts":[{"id":"example","file":"layout.json","sample":"sample.csv"}]}}"#,
        )
        .unwrap();
        std::fs::write(source.join("layout.json"), layout(aliases)).unwrap();
        std::fs::write(source.join("sample.csv"), SAMPLE).unwrap();
        source
    }

    fn plugin_root(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vs-plugin-layout-{test}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_layout_that_reads_its_own_sample_installs_and_is_listed() {
        let dir = plugin_root("ok");
        let plugins = Plugins::new(&dir);
        plugins
            .install(&package(&dir, r#"{"CUSTOMERINPAYMENT":"DEPOSIT"}"#))
            .unwrap();

        let path = db_path("plugin-listed");
        let listed = listing(&path, &plugins);
        let mine = listed
            .iter()
            .find(|t| t.name == "Example Broker")
            .expect("the plugin's layout is in the one list the wizard reads");
        assert_eq!(mine.source, TemplateSource::Plugin);
        assert_eq!(mine.id, "com.example.broker/example");
        assert_eq!(mine.plugin.as_deref(), Some("com.example.broker"));

        // It is not the wizard's to delete: removing half a package is not a state to leave.
        assert!(delete_template(&path, &plugins, &mine.id).is_err());
    }

    #[test]
    fn a_layout_that_leaves_a_wording_of_its_own_sample_unmapped_does_not_install() {
        let dir = plugin_root("unmapped");
        let plugins = Plugins::new(&dir);
        // `CUSTOMER_INPAYMENT` is this broker's word for a deposit and nothing shared says so.
        let failure = plugins.install(&package(&dir, "{}")).unwrap_err();

        assert!(
            format!("{failure:?}").contains("unmapped"),
            "the package says which wording it left behind: {failure:?}"
        );
        assert!(
            listing(&db_path("plugin-refused"), &plugins)
                .iter()
                .all(|t| t.name != "Example Broker"),
            "a refused package installs nothing at all"
        );
    }
}
