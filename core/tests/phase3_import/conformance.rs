//! Conformance: one broker layout, one file, one expected result.
//!
//! A layout is data, and the day it can be installed rather than shipped, an untested one is
//! somebody else's untested one. So a fixture states the whole journey — the file is recognised
//! as that layout, and importing it produces exactly these operations, written in the app's own
//! transaction format (ADR-0066) so the expectation needs no second vocabulary.
//!
//! Regenerate an expectation with `UPDATE_FIXTURES=1 cargo test -p sq-core --test phase3_import`
//! and read the diff before keeping it: a fixture is only worth what was checked by eye once.

use super::*;
use serde::Deserialize;
use sq_core::import::{CanonicalFile, CanonicalRow, best_match, builtin_presets};
use std::path::{Path, PathBuf};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/presets")
}

/// A fixture's own file, for a test that wants the bytes rather than the whole journey. One
/// sample lives in one place: a second copy of a broker's export drifts from the expectation
/// that was checked against it.
pub fn sample(name: &str, file: &str) -> String {
    let path = fixtures_dir().join(name).join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[derive(Debug, Deserialize)]
struct Fixture {
    /// The layout the file must be recognised as.
    preset: String,
    file: String,
    /// `redacted` is a real export with the numbers and names replaced; `declared` is written
    /// from the broker's published columns and proves the layout self-consistent, nothing more.
    #[allow(dead_code)]
    sample: String,
    #[allow(dead_code)]
    #[serde(default)]
    note: String,
    account: FixtureAccount,
    questions: Questions,
}

#[derive(Debug, Deserialize)]
struct FixtureAccount {
    name: String,
    /// `SECURITIES` for a broker statement, `DEPOSIT` for a bank or wallet one.
    kind: String,
    currency: String,
}

/// What the wizard had to ask a human. The point of a layout is that this is all zeros.
#[derive(Debug, Default, PartialEq, Eq, Deserialize)]
struct Questions {
    /// 1 when the file was not recognised as a layout at all.
    layout: usize,
    /// Wordings the layout has no meaning for, each of which is one alias to pick.
    kinds: usize,
    /// Rows that cannot be written as they stand.
    invalid: usize,
    /// Rows written, but with something to look at.
    warnings: usize,
}

struct Loaded {
    name: String,
    dir: PathBuf,
    fixture: Fixture,
    bytes: Vec<u8>,
}

fn load_all() -> Vec<Loaded> {
    let mut out = Vec::new();
    let dir = fixtures_dir();
    let entries = std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let meta = std::fs::read_to_string(path.join("fixture.json")).unwrap();
        let fixture: Fixture = serde_json::from_str(&meta).unwrap();
        let bytes = std::fs::read(path.join(&fixture.file)).unwrap();
        out.push(Loaded {
            name: path.file_name().unwrap().to_string_lossy().into_owned(),
            dir: path,
            fixture,
            bytes,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    assert!(!out.is_empty(), "no fixtures under {}", dir.display());
    out
}

fn preset_named(name: &str) -> &'static BrokerPreset {
    builtin_presets()
        .iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("no shipped preset named {name:?}"))
}

/// The account a file of this shape lands on, with its cash side — the pair the wizard derives
/// from "what the file is" rather than from a picker.
fn account_for(store: &Store, fixture: &Fixture) -> Account {
    match fixture.account.kind.as_str() {
        "SECURITIES" => depot(store, &fixture.account.name, &fixture.account.currency),
        "DEPOSIT" => {
            let account = Account::deposit(&fixture.account.name, &fixture.account.currency);
            store.save_account(&account).unwrap();
            account
        }
        other => panic!("fixture account kind {other:?}"),
    }
}

/// What the file is recognised as, asked exactly the way the host asks it.
fn recognised(loaded: &Loaded) -> Option<&'static str> {
    let preset = preset_named(&loaded.fixture.preset);
    let parsed = parse_file(&loaded.bytes, &preset.config).unwrap();
    let head = String::from_utf8_lossy(&loaded.bytes[..loaded.bytes.len().min(2048)]).into_owned();
    best_match(
        builtin_presets()
            .iter()
            .map(|p| (p.name.as_str(), p.match_rule())),
        &parsed.headers,
        Some(&loaded.fixture.file),
        &head,
    )
}

#[test]
fn every_fixture_is_recognised_as_the_layout_it_names() {
    for loaded in load_all() {
        assert_eq!(
            recognised(&loaded),
            Some(loaded.fixture.preset.as_str()),
            "{}: recognised as something else, or tied with another layout",
            loaded.name
        );
    }
}

/// Rows in an order that does not depend on generated ids: a commit writes them under fresh
/// uuids, and `(date, id)` is the ledger's order, not a file's.
fn sorted(mut rows: Vec<CanonicalRow>) -> Vec<CanonicalRow> {
    rows.sort_by(|a, b| {
        (&a.date, &a.kind, &a.symbol, &a.amount, &a.quantity).cmp(&(
            &b.date,
            &b.kind,
            &b.symbol,
            &b.amount,
            &b.quantity,
        ))
    });
    rows
}

/// The whole journey: recognised layout -> preview -> commit -> what the ledger now holds.
fn imported(loaded: &Loaded) -> (CanonicalFile, ImportPreview) {
    let store = Store::open_in_memory().unwrap();
    let account = account_for(&store, &loaded.fixture);
    let preset = preset_named(&loaded.fixture.preset);
    let mapping = preset.mapping().with_account(&account.id);

    let service = ImportService::new(&store);
    let preview = service
        .preview(&loaded.bytes, &preset.config, Some(&mapping), &[])
        .unwrap();
    service
        .commit(&preview, &ImportOptions::default())
        .unwrap_or_else(|e| panic!("{}: {e}", loaded.name));

    let accounts = store.list_accounts().unwrap();
    let ids: Vec<String> = accounts.iter().map(|a| a.id.clone()).collect();
    let written = store.transactions_for_accounts(&ids, None).unwrap();
    let file = canonical_to_file(&written, &accounts, &store.list_securities().unwrap()).unwrap();
    let mut parsed: CanonicalFile = serde_json::from_str(&file).unwrap();
    parsed.rows = sorted(parsed.rows);
    (parsed, preview)
}

#[test]
fn every_fixture_imports_to_the_operations_it_expects() {
    let updating = std::env::var_os("UPDATE_FIXTURES").is_some();
    for loaded in load_all() {
        let (produced, _) = imported(&loaded);
        let path = loaded.dir.join("expected.json");
        if updating {
            let json = serde_json::to_string_pretty(&produced).unwrap();
            std::fs::write(&path, json + "\n").unwrap();
            continue;
        }
        let expected: CanonicalFile =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|e| {
                panic!(
                    "{}: {e} — run with UPDATE_FIXTURES=1 and read the diff",
                    loaded.name
                )
            }))
            .unwrap();
        assert_eq!(
            produced.rows,
            sorted(expected.rows),
            "{} imported differently than its expectation",
            loaded.name
        );
    }
    assert!(!updating, "fixtures rewritten; re-run without UPDATE_FIXTURES");
}

#[test]
fn a_known_layout_asks_the_user_for_nothing() {
    let mut report = String::from("\nfixture                        layout kinds invalid warnings\n");
    let mut wrong = Vec::new();
    for loaded in load_all() {
        let (_, preview) = imported(&loaded);
        let asked = Questions {
            layout: usize::from(recognised(&loaded).is_none()),
            kinds: preview.unknown_kinds().len(),
            invalid: preview.summary.invalid,
            warnings: preview.summary.warnings,
        };
        report.push_str(&format!(
            "{:<30} {:>6} {:>5} {:>7} {:>8}\n",
            loaded.name, asked.layout, asked.kinds, asked.invalid, asked.warnings
        ));
        if asked != loaded.fixture.questions {
            wrong.push(format!(
                "{}: {asked:?}, fixture says {:?}",
                loaded.name, loaded.fixture.questions
            ));
        }
    }
    println!("{report}");
    assert!(wrong.is_empty(), "{}\n{}", report, wrong.join("\n"));
}

/// Every shipped layout, with no fixture needed: the columns a layout maps are what it is
/// recognised by, so a layout that cannot recognise its own columns recognises nothing — and two
/// that fit one file equally well recognise nothing either, which is the expensive case.
#[test]
fn every_shipped_layout_recognises_its_own_columns() {
    let mut undetected = Vec::new();
    for preset in builtin_presets() {
        let headers: Vec<String> = preset.mapping.columns.values().cloned().collect();
        let picked = best_match(
            builtin_presets()
                .iter()
                .map(|p| (p.name.as_str(), p.match_rule())),
            &headers,
            None,
            "",
        );
        if picked != Some(preset.name.as_str()) {
            undetected.push(format!("{} -> {picked:?}", preset.name));
        }
    }
    assert!(
        undetected.is_empty(),
        "layouts that do not answer to their own header row:\n{}",
        undetected.join("\n")
    );
}

/// Layouts shipped without a fixture, listed rather than counted: the list is the debt, and it
/// is meant to shrink. A new layout joins it or brings a fixture — never silently neither.
const WITHOUT_A_FIXTURE: &[&str] = &[
    "Avanza",
    "BUX",
    "Bitvavo",
    "Charles Schwab",
    "CoinTracking",
    "Coinbase",
    "Crypto.com",
    "DEGIRO",
    "Delta",
    "Directa",
    "Disnat",
    "Finpension",
    "Freetrade",
    "Interactive Brokers · dividends",
    "Interactive Brokers · trades",
    "InvestEngine",
    "Investimental",
    "Parqet",
    "Rabobank",
    "Relai",
    "Revolut Crypto",
    "Revolut Invest",
    "Saxo Bank",
    "Swissquote",
    "Trade Republic · statement",
    "Trading 212",
    "XTB",
    "eToro",
];

#[test]
fn the_layouts_without_a_fixture_are_the_ones_listed() {
    let covered: Vec<String> = load_all().into_iter().map(|l| l.fixture.preset.clone()).collect();
    let mut missing: Vec<&str> = builtin_presets()
        .iter()
        .map(|p| p.name.as_str())
        .filter(|name| !covered.iter().any(|c| c == name))
        .collect();
    missing.sort_unstable();
    let mut expected: Vec<&str> = WITHOUT_A_FIXTURE.to_vec();
    expected.sort_unstable();
    assert_eq!(
        missing, expected,
        "the shipped layouts and the fixtures no longer agree — add the fixture, or add the name"
    );
}
