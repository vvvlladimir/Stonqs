//! The shipped example plugin, installed the way the app installs one.
//!
//! It is the format's documentation, so it has to be a package this build actually accepts —
//! a README that does not install is worse than none.

use sq_app_lib::plugins::{Base, Plugins, Status};
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
/// express (ADR-0073). `UPDATE_FIXTURES=1` rewrites the expectation the package ships and then
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
        .read_file("statement.sta", &sample)
        .unwrap()
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
    assert!(matches!(refusal, sq_app_lib::plugins::reader::Refusal::Failed(_)));

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
