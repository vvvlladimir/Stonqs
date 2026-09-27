//! The shipped example plugin, installed the way the app installs one.
//!
//! It is the format's documentation, so it has to be a package this build actually accepts —
//! a README that does not install is worse than none.

use sq_app_lib::plugins::{Base, PageKind, Plugins, Status};
use std::path::Path;

#[test]
fn the_example_theme_installs_and_is_offered() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/midnight");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    let plugins = Plugins::new(&dir);
    let installed = plugins.install(&example).unwrap();
    assert_eq!(installed.status, Status::Ok, "the example is built for this API");

    let themes = plugins.themes().unwrap();
    assert_eq!(themes.len(), 1);
    assert_eq!(themes[0].key, "app.stonqs.midnight/midnight");
    assert_eq!(themes[0].base, Base::Dark);

    let css = plugins.theme_css("app.stonqs.midnight", "midnight").unwrap();
    assert!(
        css.contains(":root"),
        "a theme redefines the app's own properties"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_example_layout_installs_and_reads_its_own_sample() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/northbay");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    let plugins = Plugins::new(&dir);
    // Installing is the check: a layout that does not read its own sample is refused here.
    let installed = plugins.install(&example).unwrap();
    assert_eq!(installed.status, Status::Ok);

    let layouts = plugins.layouts().unwrap();
    assert_eq!(layouts.len(), 1);
    assert_eq!(layouts[0].0, "app.stonqs.northbay/northbay");
    assert_eq!(layouts[0].1.name, "Northbay Securities");

    // And the wizard recognises the sample as *this* layout: fitting no better than a shipped one
    // would be a tie, which is no answer at all, and the example would teach the wrong lesson.
    let sample = std::fs::read(example.join("sample.csv")).unwrap();
    let headers: Vec<String> = String::from_utf8_lossy(&sample)
        .lines()
        .next()
        .unwrap()
        .split(',')
        .map(str::to_string)
        .collect();
    let recognised = sq_app_lib::import_templates::match_for(
        &dir.join("portfolio.db"),
        &plugins,
        &headers,
        Some("sample.csv"),
        &String::from_utf8_lossy(&sample),
    )
    .expect("the file is recognised as one layout");
    assert_eq!(recognised.id, "app.stonqs.northbay/northbay");

    std::fs::remove_dir_all(&dir).ok();
}

/// The compute half of the example: a WASM component that reads a format no column mapping can
/// express (ADR-0086). `UPDATE_FIXTURES=1` rewrites the expectation the package ships and then
/// fails on purpose, so the diff is read rather than waved through.
#[test]
fn the_example_reader_installs_and_reads_its_own_sample() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/mt940");
    let sample = std::fs::read(example.join("sample.sta")).unwrap();

    let reading =
        sq_app_lib::plugins::reader::read(&example.join("reader.wasm"), &sample, "sample.sta", None)
            .expect("the reader reads the sample it ships");

    if std::env::var("UPDATE_FIXTURES").is_ok() {
        let pretty: serde_json::Value = serde_json::from_str(&reading.canonical).unwrap();
        std::fs::write(
            example.join("expected.json"),
            format!("{}\n", serde_json::to_string_pretty(&pretty).unwrap()),
        )
        .unwrap();
        panic!("expected.json was rewritten — read the diff and run again without UPDATE_FIXTURES");
    }

    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let plugins = Plugins::new(&dir);
    // Installing runs the fixture check: what the reader produces must be the file the package
    // says it produces, and must parse as the app's own transaction format.
    let installed = plugins.install(&example).unwrap();
    assert_eq!(installed.status, Status::Ok);
    assert_eq!(installed.readers.len(), 1);

    // And the installed copy is what answers for a file of that kind.
    let claimed = plugins
        .read_file("statement.sta", &sample, None)
        .unwrap()
        .read
        .expect("a reader claims the file");
    assert_eq!(claimed.0, "app.stonqs.mt940/mt940");
    assert_eq!(claimed.1.canonical, reading.canonical);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_reader_answers_the_same_way_twice() {
    // The clock it is given does not move and its randomness is seeded, so there is nothing for
    // a second run to differ by — which is what lets the host run a reader once, at load, and
    // hand the commit what the preview was built from.
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/mt940");
    let sample = std::fs::read(example.join("sample.sta")).unwrap();
    let module = example.join("reader.wasm");

    let once = sq_app_lib::plugins::reader::read(&module, &sample, "sample.sta", None).unwrap();
    let twice = sq_app_lib::plugins::reader::read(&module, &sample, "sample.sta", None).unwrap();
    assert_eq!(once.canonical, twice.canonical);
}

#[test]
fn a_file_the_reader_does_not_know_is_not_a_failure() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/mt940");
    let csv = b"date,type,amount\n2024-01-01,BUY,100\n";

    let refusal = sq_app_lib::plugins::reader::read(&example.join("reader.wasm"), csv, "book.csv", None)
        .expect_err("a CSV is not an MT940 statement");
    assert_eq!(
        refusal,
        sq_app_lib::plugins::reader::Refusal::NotMine,
        "the host moves on to the next reader rather than failing the import"
    );
}

#[test]
fn a_module_that_is_not_a_component_fails_rather_than_installs() {
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let module = dir.join("reader.wasm");
    std::fs::write(&module, b"not a wasm module at all").unwrap();

    let refusal = sq_app_lib::plugins::reader::read(&module, b":20:X\n:61:x\n", "s.sta", None)
        .expect_err("a file that is not a component cannot read anything");
    assert!(matches!(refusal, sq_app_lib::plugins::reader::Refusal::Broken(_)));

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_reader_that_does_not_match_its_own_expectation_installs_nothing() {
    // The whole reason a reader ships an expectation and a layout does not: a layout that
    // misreads a column leaves a question in the wizard, while a reader that misreads one hands
    // over a document that looks perfectly correct.
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/mt940");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    let source = dir.join("package");
    std::fs::create_dir_all(&source).unwrap();
    for file in ["plugin.json", "reader.wasm", "sample.sta", "expected.json"] {
        std::fs::copy(example.join(file), source.join(file)).unwrap();
    }
    let expected = std::fs::read_to_string(source.join("expected.json")).unwrap();
    std::fs::write(
        source.join("expected.json"),
        expected.replace("\"DEPOSIT\"", "\"WITHDRAWAL\""),
    )
    .unwrap();

    let plugins = Plugins::new(&dir);
    let failure = plugins.install(&source).unwrap_err();
    assert!(
        format!("{failure:?}").contains("does not read its own sample"),
        "{failure:?}"
    );
    assert!(plugins.list().unwrap().is_empty(), "nothing was written");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_example_classification_set_installs_and_reads_as_a_tree() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/regions");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    let plugins = Plugins::new(&dir);
    // Installing is the check: a set that reads as no tree, or leaves a row invalid, is refused.
    let installed = plugins.install(&example).unwrap();
    assert_eq!(installed.status, Status::Ok);

    let sets = plugins.taxonomy_sets().unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].key, "app.stonqs.regions/regions");
    assert_eq!(sets[0].name, "Regions");

    // And the CSV goes back out to be previewed by the same code every taxonomy file goes
    // through — the set has no path into the portfolio of its own.
    let csv = plugins.taxonomy_csv("app.stonqs.regions", "regions").unwrap();
    let parsed = sq_core::import::parse_csv(&csv, &sq_core::import::ParseConfig::default()).unwrap();
    let config = sq_core::import::detect_taxonomy_config(&parsed);
    let preview = sq_core::import::build_taxonomy_preview(&parsed, &config, &[], None);
    assert_eq!(
        preview.name, "Regions",
        "the first level repeats and is the tree's name"
    );
    assert!(
        preview
            .nodes
            .iter()
            .any(|n| n.path.last().is_some_and(|last| last == "Emerging markets")),
        "the tree carries its categories"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_classification_set_that_is_not_a_tree_installs_nothing() {
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    let source = dir.join("package");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("plugin.json"),
        r#"{"id":"com.example.flat","api":1,"name":"Flat",
            "provides":{"taxonomies":[{"id":"flat","name":"Flat","file":"flat.csv"}]}}"#,
    )
    .unwrap();
    // No level column anywhere: this is a price list, not a classification.
    std::fs::write(source.join("flat.csv"), "Date,Close\n2024-01-01,100\n").unwrap();

    let plugins = Plugins::new(&dir);
    let failure = plugins.install(&source).unwrap_err();
    assert!(
        format!("{failure:?}").contains("classification set"),
        "{failure:?}"
    );
    assert!(plugins.list().unwrap().is_empty(), "nothing was written");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_example_dictionary_installs_and_answers_the_import() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/finnish-words");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    let plugins = Plugins::new(&dir);
    // Installing is the check: the sample must be one only this dictionary can read.
    let installed = plugins.install(&example).unwrap();
    assert_eq!(installed.status, Status::Ok);
    assert_eq!(installed.dictionaries.len(), 1);

    // And the installed words are what the import reads after its own.
    let sample = std::fs::read(example.join("sample.csv")).unwrap();
    let store = sq_core::storage::Store::open_in_memory().unwrap();
    let preview = sq_core::import::ImportService::new(&store)
        .with_kind_dictionary(plugins.kind_words().unwrap())
        .preview(&sample, &sq_core::import::ParseConfig::default(), None, &[])
        .unwrap();
    let kind = |value: &str| {
        preview
            .kinds
            .iter()
            .find(|k| k.value == value)
            .and_then(|k| k.kind)
    };
    assert!(
        preview.unknown_kinds().is_empty(),
        "{:?}",
        preview.unknown_kinds()
    );
    assert_eq!(kind("Osto"), Some(sq_core::model::TransactionKind::Buy));
    assert_eq!(
        kind("Nosto"),
        Some(sq_core::model::TransactionKind::Withdrawal),
        "listed before the word it contains"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_dictionary_whose_sample_the_app_already_reads_installs_nothing() {
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    let source = dir.join("package");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("plugin.json"),
        r#"{"id":"com.example.words","api":1,"name":"Words",
            "provides":{"dictionaries":[{"id":"w","file":"words.json","sample":"sample.csv"}]}}"#,
    )
    .unwrap();
    std::fs::write(
        source.join("words.json"),
        r#"{"words":[{"word":"Osto","kind":"BUY"}]}"#,
    )
    .unwrap();
    // Every wording here is the app's own, so the sample proves nothing about the words.
    std::fs::write(
        source.join("sample.csv"),
        "Date,Type,Amount,Currency\n2024-01-02,DEPOSIT,100,EUR\n",
    )
    .unwrap();

    let plugins = Plugins::new(&dir);
    let failure = plugins.install(&source).unwrap_err();
    assert!(format!("{failure:?}").contains("reads without it"), "{failure:?}");
    assert!(plugins.list().unwrap().is_empty(), "nothing was written");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_dictionary_claiming_the_apps_own_word_installs_nothing() {
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    let source = dir.join("package");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("plugin.json"),
        r#"{"id":"com.example.words","api":1,"name":"Words",
            "provides":{"dictionaries":[{"id":"w","file":"words.json","sample":"sample.csv"}]}}"#,
    )
    .unwrap();
    // The shipped keywords read "Sell" first, so this word would never be reached.
    std::fs::write(
        source.join("words.json"),
        r#"{"words":[{"word":"Osto","kind":"BUY"},{"word":"Sell","kind":"BUY"}]}"#,
    )
    .unwrap();
    std::fs::write(
        source.join("sample.csv"),
        "Date,Type,Amount,Currency\n2024-01-02,Osto,100,EUR\n",
    )
    .unwrap();

    let plugins = Plugins::new(&dir);
    let failure = plugins.install(&source).unwrap_err();
    assert!(format!("{failure:?}").contains("already reads"), "{failure:?}");
    assert!(plugins.list().unwrap().is_empty(), "nothing was written");

    std::fs::remove_dir_all(&dir).ok();
}

/// The writer half of the compute contract (ADR-0080): the app's own transaction file in, another
/// program's format out. `UPDATE_FIXTURES=1` rewrites the expectation the package ships and then
/// fails on purpose, so the diff is read rather than waved through.
#[test]
fn the_example_writer_installs_and_writes_its_own_sample() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/ledger");
    let sample = std::fs::read_to_string(example.join("sample.json")).unwrap();

    let written = sq_app_lib::plugins::writer::write(&example.join("writer.wasm"), &sample)
        .expect("the writer writes the sample it ships");

    if std::env::var("UPDATE_FIXTURES").is_ok() {
        std::fs::write(example.join("expected.journal"), &written).unwrap();
        panic!("expected.journal was rewritten — read the diff and run again without UPDATE_FIXTURES");
    }

    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let plugins = Plugins::new(&dir);
    // Installing runs the fixture check: the sample must be written to exactly these bytes.
    let installed = plugins.install(&example).unwrap();
    assert_eq!(installed.status, Status::Ok);

    let writers = plugins.writers().unwrap();
    assert_eq!(writers.len(), 1);
    assert_eq!(writers[0].key, "app.stonqs.ledger/ledger");
    assert_eq!(writers[0].extension, "journal");

    // And the installed copy is what the export asks.
    let again = plugins.write("app.stonqs.ledger/ledger", &sample).unwrap();
    assert_eq!(again, written);
    assert_eq!(again, std::fs::read(example.join("expected.journal")).unwrap());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_writer_that_does_not_match_its_own_expectation_installs_nothing() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/ledger");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    let source = dir.join("package");
    std::fs::create_dir_all(&source).unwrap();
    for file in ["plugin.json", "writer.wasm", "sample.json", "expected.journal"] {
        std::fs::copy(example.join(file), source.join(file)).unwrap();
    }
    let expected = std::fs::read_to_string(source.join("expected.journal")).unwrap();
    std::fs::write(
        source.join("expected.journal"),
        expected.replace("Buy AAPL", "Sell AAPL"),
    )
    .unwrap();

    let plugins = Plugins::new(&dir);
    let failure = plugins.install(&source).unwrap_err();
    assert!(
        format!("{failure:?}").contains("does not write its own sample"),
        "{failure:?}"
    );
    assert!(plugins.list().unwrap().is_empty(), "nothing was written");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_writer_refusing_a_document_names_the_plugin() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/ledger");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let plugins = Plugins::new(&dir);
    plugins.install(&example).unwrap();

    let failure = plugins
        .write(
            "app.stonqs.ledger/ledger",
            r#"{"format":"stonqs.transactions","version":9,"rows":[]}"#,
        )
        .unwrap_err();
    assert!(
        matches!(&failure, sq_app_lib::error::UiError::Writer { plugin, .. } if plugin == "app.stonqs.ledger/ledger"),
        "{failure:?}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_example_widget_installs_and_is_served_under_its_own_policy() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/heat");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    let plugins = Plugins::new(&dir);
    assert_eq!(plugins.install(&example).unwrap().status, Status::Ok);

    let widgets = plugins.widgets().unwrap();
    assert_eq!(widgets.len(), 1);
    assert_eq!(widgets[0].key, "app.stonqs.heat/heat");
    assert_eq!(
        widgets[0].plugin_name, "Heat map",
        "the tile carries whose it is (ADR-0082)"
    );
    assert_eq!(widgets[0].reads, [sq_app_lib::plugins::Read::Positions]);

    let (html, csp) = plugins.page(PageKind::Widget, "app.stonqs.heat", "heat").unwrap();
    assert!(
        html.contains("stonqs.render("),
        "the module is inlined into the page"
    );
    let nonce = csp
        .split("'nonce-")
        .nth(1)
        .and_then(|rest| rest.split('\'').next())
        .unwrap();
    assert_eq!(
        html.matches(&format!("nonce=\"{nonce}\"")).count(),
        2,
        "the shim and the module, and nothing else, may run"
    );
    let (_, again) = plugins.page(PageKind::Widget, "app.stonqs.heat", "heat").unwrap();
    assert_ne!(csp, again, "a nonce is never reused");
    assert!(
        plugins
            .page(PageKind::Widget, "app.stonqs.heat", "missing")
            .is_err()
    );
    assert!(plugins.page(PageKind::Screen, "app.stonqs.heat", "heat").is_err());
    assert!(plugins.page(PageKind::Widget, "../heat", "heat").is_err());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_widget_reading_something_this_api_does_not_offer_installs_nothing() {
    let dir = std::env::temp_dir().join(format!("stonqs-widget-{}", uuid::Uuid::new_v4()));
    let source = dir.join("src");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(
        source.join("plugin.json"),
        r#"{"id":"com.example.greedy","api":1,"name":"Greedy","provides":{"widgets":[
            {"id":"w","name":"W","file":"w.js","reads":["transactions"],
             "size":{"w":6,"h":6},"min":{"w":3,"h":3}}]}}"#,
    )
    .unwrap();
    std::fs::write(source.join("w.js"), "stonqs.render(() => {});").unwrap();

    let plugins = Plugins::new(&dir);
    assert!(plugins.install(&source).is_err(), "a read is from a closed list");
    assert!(plugins.list().unwrap().is_empty(), "nothing was written");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_example_screen_installs_keeps_a_document_and_is_served() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/spending");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    let plugins = Plugins::new(&dir);
    assert_eq!(plugins.install(&example).unwrap().status, Status::Ok);

    let screens = plugins.screens().unwrap();
    assert_eq!(screens.len(), 1);
    assert_eq!(screens[0].key, "app.stonqs.spending/spending");
    assert!(screens[0].periodic && screens[0].storage);
    assert_eq!(screens[0].reads, [sq_app_lib::plugins::Read::Transactions]);

    assert!(plugins.keeps_state("app.stonqs.spending").unwrap());
    let (html, _) = plugins
        .page(PageKind::Screen, "app.stonqs.spending", "spending")
        .unwrap();
    assert!(html.contains("stonqs.save("), "the screen keeps its rules");
    assert!(
        plugins
            .page(PageKind::Widget, "app.stonqs.spending", "spending")
            .is_err(),
        "a screen is not served as a widget"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_plugin_without_storage_keeps_nothing() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/heat");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    let plugins = Plugins::new(&dir);
    plugins.install(&example).unwrap();
    assert!(!plugins.keeps_state("app.stonqs.heat").unwrap());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn the_example_tool_installs_and_answers_its_own_sample() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/concentration");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();

    let plugins = Plugins::new(&dir);
    // Installing is the check: the sample must come out exactly as `expected.json` says.
    assert_eq!(plugins.install(&example).unwrap().status, Status::Ok);

    let tools = plugins.tools().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(
        tools[0].model_name,
        "plugin_app_stonqs_concentration_concentration"
    );
    assert!(
        !tools[0].periodic,
        "positions are read on a date, not over a period"
    );

    let sample: serde_json::Value =
        serde_json::from_slice(&std::fs::read(example.join("sample.json")).unwrap()).unwrap();
    let answer = sq_app_lib::plugins::tool::call(&tools[0].module, &sample["args"], &sample["data"]).unwrap();
    assert_eq!(answer["effective_positions"], "2.6");
    assert_eq!(answer["top_share_percent"], "80.0");

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_tool_that_does_not_answer_its_own_sample_installs_nothing() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/concentration");
    let dir = std::env::temp_dir().join(format!("stonqs-tool-{}", uuid::Uuid::new_v4()));
    let source = dir.join("src");
    std::fs::create_dir_all(&source).unwrap();
    for file in ["plugin.json", "tool.wasm", "schema.json", "sample.json"] {
        std::fs::copy(example.join(file), source.join(file)).unwrap();
    }
    std::fs::write(source.join("expected.json"), r#"{"positions": 99}"#).unwrap();

    let plugins = Plugins::new(&dir);
    assert!(plugins.install(&source).is_err());
    assert!(plugins.list().unwrap().is_empty(), "nothing was written");

    std::fs::remove_dir_all(&dir).ok();
}

/// `.` and `-` both become `_` in the name the model calls a tool by, so `a.b` + `c` and `a` +
/// `b_c` meet in `plugin_a_b_c`. The second package is refused rather than hiding one tool.
#[test]
fn a_tool_whose_name_another_plugin_already_answers_to_installs_nothing() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/concentration");
    let dir = std::env::temp_dir().join(format!("stonqs-tool-{}", uuid::Uuid::new_v4()));
    let plugins = Plugins::new(dir.join("root"));
    let package = |plugin: &str, tool: &str| {
        let source = dir.join(plugin);
        std::fs::create_dir_all(&source).unwrap();
        for file in ["tool.wasm", "schema.json", "sample.json", "expected.json"] {
            std::fs::copy(example.join(file), source.join(file)).unwrap();
        }
        let manifest = std::fs::read_to_string(example.join("plugin.json"))
            .unwrap()
            .replace("app.stonqs.concentration", plugin)
            .replace(r#""id": "concentration""#, &format!(r#""id": "{tool}""#));
        std::fs::write(source.join("plugin.json"), manifest).unwrap();
        source
    };

    plugins.install(&package("a.b", "c")).unwrap();
    let failure = plugins.install(&package("a", "b_c")).unwrap_err();
    assert!(format!("{failure:?}").contains("plugin_a_b_c"), "{failure:?}");
    assert_eq!(
        plugins.list().unwrap().len(),
        1,
        "the second package was not written"
    );
    // Reinstalling the owner is not a clash with itself.
    plugins.install(&package("a.b", "c")).unwrap();

    std::fs::remove_dir_all(&dir).ok();
}

/// A module that breaks over a file never said the file was its own, so the import moves past it
/// and says so — one broken package must not refuse every file the user opens.
#[test]
fn a_reader_that_breaks_is_passed_over_and_named() {
    let example = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/mt940");
    let dir = std::env::temp_dir().join(format!("stonqs-example-{}", uuid::Uuid::new_v4()));
    let plugins = Plugins::new(&dir);
    plugins.install(&example).unwrap();
    // What a trap, a deadline or a corrupted file looks like from the host's side.
    std::fs::write(
        dir.join("plugins/app.stonqs.mt940/reader.wasm"),
        b"no longer a module",
    )
    .unwrap();

    let reading = plugins
        .read_file("statement.sta", b":20:X\n", None)
        .expect("a broken reader is not the import's failure");
    assert!(
        reading.read.is_none(),
        "the file goes on to the app's own readers"
    );
    assert_eq!(reading.skipped.len(), 1);
    assert_eq!(reading.skipped[0].plugin, "app.stonqs.mt940/mt940");

    std::fs::remove_dir_all(&dir).ok();
}

/// A sealed file stops the import naming the reader that asked, and the password given back
/// reaches that reader alone: a wrong one says so, the right one reads the file.
#[test]
fn a_sealed_file_is_read_once_its_reader_is_given_the_password() {
    use sq_app_lib::error::UiError;
    use sq_app_lib::plugins::Unlock;

    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sealed");
    let dir = std::env::temp_dir().join(format!("stonqs-sealed-{}", uuid::Uuid::new_v4()));
    let plugins = Plugins::new(&dir);
    // The sample is an open file: a package proves itself without anybody's password.
    plugins.install(&fixture).unwrap();

    let expected = std::fs::read_to_string(fixture.join("expected.json")).unwrap();
    let sealed = format!("SEALED:hunter2\n{}", expected.trim_end());
    let unlock = |reader: &str, password: &str| Unlock {
        reader: reader.into(),
        password: password.into(),
    };
    let asked = |result: Result<_, UiError>| match result {
        Err(UiError::FileProtected { reader, tried, .. }) => (reader, tried),
        other => panic!(
            "expected a password prompt, got {:?}",
            other.map(|_: sq_app_lib::plugins::FileReading| ())
        ),
    };

    let (reader, tried) = asked(plugins.read_file("statement.sealed", sealed.as_bytes(), None));
    assert_eq!(
        reader, "test.sealed/sealed",
        "the prompt names the reader that asked"
    );
    assert!(!tried, "nothing was tried yet");

    let wrong = unlock("test.sealed/sealed", "letmein");
    let (_, tried) = asked(plugins.read_file("statement.sealed", sealed.as_bytes(), Some(&wrong)));
    assert!(tried, "a refused password is told apart from none");

    let elsewhere = unlock("someone.else/reader", "hunter2");
    let (_, tried) = asked(plugins.read_file("statement.sealed", sealed.as_bytes(), Some(&elsewhere)));
    assert!(
        !tried,
        "a password meant for another reader is not handed to this one"
    );

    let right = unlock("test.sealed/sealed", "hunter2");
    let read = plugins
        .read_file("statement.sealed", sealed.as_bytes(), Some(&right))
        .unwrap()
        .read
        .expect("the reader claims the file");
    assert_eq!(read.1.canonical.trim_end(), expected.trim_end());

    std::fs::remove_dir_all(&dir).ok();
}
