//! How long a big file takes. Not a benchmark — a ceiling, so a change that turns a pass over
//! the rows into a pass per row is caught here rather than by the user whose broker prints ten
//! years of a monthly plan. `#[ignore]` because the figures only mean anything in a release
//! build: run it as `cargo test --release -p sq-core --test phase3_import -- --ignored`.

use super::*;
use std::time::{Duration, Instant};

/// A file of `rows` operations over 40 instruments, each row distinct.
fn big_file(rows: usize) -> Vec<u8> {
    let mut out = String::from("date,type,symbol,quantity,price,currency,amount\n");
    for i in 0..rows {
        out.push_str(&format!(
            "2024-{:02}-{:02},BUY,SYM{},{},{}.{:02},USD,{}.{:02}\n",
            (i % 12) + 1,
            (i % 28) + 1,
            i % 40,
            (i % 7) + 1,
            100 + i % 900,
            i % 100,
            100 + i % 900,
            i % 100
        ));
    }
    out.into_bytes()
}

fn took(what: &str, limit: Duration, run: impl FnOnce()) -> Duration {
    let start = Instant::now();
    run();
    let spent = start.elapsed();
    println!("{what:<26} {spent:>10.2?}  (ceiling {limit:.2?})");
    assert!(
        spent < limit,
        "{what} took {spent:.2?}, over the {limit:.2?} ceiling"
    );
    spent
}

/// Every step of a 50 000-row import, each under a ceiling generous enough that only a change
/// of complexity trips it. The user's own wait is the sum of these, and the wizard re-previews
/// on every change of the layout — so the preview's ceiling is the one that matters.
#[test]
#[ignore = "timing only means something in a release build"]
fn a_fifty_thousand_row_file_stays_under_its_ceilings() {
    let bytes = big_file(50_000);
    took("parse", Duration::from_secs(5), || {
        let parsed = parse_file(&bytes, &ParseConfig::default()).unwrap();
        assert_eq!(parsed.rows.len(), 50_000);
    });

    let (store, account) = store_with_account();
    let service = ImportService::new(&store);
    let mut detected = None;
    took("detect the layout", Duration::from_secs(8), || {
        detected = Some(
            service
                .preview(&bytes, &ParseConfig::default(), None, &[])
                .unwrap(),
        );
    });
    let mapping = detected.unwrap().mapping.with_account(&account.id);

    let mut preview = None;
    took("preview", Duration::from_secs(8), || {
        preview = Some(
            service
                .preview(&bytes, &ParseConfig::default(), Some(&mapping), &[])
                .unwrap(),
        );
    });
    let preview = preview.unwrap();

    took("commit", Duration::from_secs(10), || {
        let result = service
            .commit(
                &preview,
                &ImportOptions {
                    new_security_source: Some("yahoo".into()),
                    ..ImportOptions::default()
                },
            )
            .unwrap();
        assert!(result.imported > 0);
    });

    // The second preview compares every row against a ledger that now holds them: the pass that
    // would be quadratic if identity were a scan rather than a lookup.
    took("preview against the ledger", Duration::from_secs(12), || {
        let again = service
            .preview(&bytes, &ParseConfig::default(), Some(&mapping), &[])
            .unwrap();
        assert_eq!(again.summary.duplicates, again.summary.total);
    });
}

/// Ten times the rows must not cost a hundred times the work. Complexity, not speed: the ratio
/// is what is asserted, so a slow machine fails no differently than a fast one.
#[test]
#[ignore = "timing only means something in a release build"]
fn the_cost_of_a_preview_grows_with_the_rows_and_no_faster() {
    let small = big_file(2_000);
    let large = big_file(20_000);
    let run = |bytes: &[u8]| {
        let (store, account) = store_with_account();
        let service = ImportService::new(&store);
        let detected = service
            .preview(bytes, &ParseConfig::default(), None, &[])
            .unwrap();
        let mapping = detected.mapping.with_account(&account.id);
        let start = Instant::now();
        service
            .preview(bytes, &ParseConfig::default(), Some(&mapping), &[])
            .unwrap();
        start.elapsed()
    };
    // Warm the allocator and the page cache so the first measurement is not the slow one.
    run(&small);
    let a = run(&small).as_secs_f64();
    let b = run(&large).as_secs_f64();
    let ratio = b / a.max(1e-6);
    println!("2 000 rows {a:.3}s, 20 000 rows {b:.3}s — ×{ratio:.1} for ten times the rows");
    assert!(
        ratio < 30.0,
        "ten times the rows cost ×{ratio:.1}: the pass is no longer linear"
    );
}
