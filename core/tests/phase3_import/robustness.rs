//! What a file does when it is not the file the layout expected: wrong encoding, a quote nobody
//! closed, a cell a megabyte long, a date nobody can read. The rule for every case here is the
//! same and it is not "it works" — it is **the importer answers**: an error naming what went
//! wrong, or a preview that carries the damage as a problem on a row. What it must never do is
//! panic, hang, or write a number nobody can trace back to a line of the file.
//!
//! These are the files a user brings on day one, and a crash here is the whole product.

use super::*;
use sq_core::import::{ImportRow, ParsedCsv};

/// Reads a file the way the wizard's first step does, catching a panic as a failure of its own.
fn parse(name: &str, bytes: &[u8]) -> Option<ParsedCsv> {
    let parsed = std::panic::catch_unwind(|| parse_file(bytes, &ParseConfig::default()));
    match parsed {
        Err(_) => panic!("{name}: the reader panicked on a file a user can pick"),
        Ok(Err(_)) => None,
        Ok(Ok(p)) => Some(p),
    }
}

/// The whole journey with the detected layout, catching a panic anywhere in it.
fn journey(name: &str, bytes: &[u8]) -> Option<(ImportPreview, usize)> {
    let done = std::panic::catch_unwind(|| {
        let (store, account) = store_with_account();
        let service = ImportService::new(&store);
        let detected = service.preview(bytes, &ParseConfig::default(), None, &[])?;
        let mapping = detected.mapping.clone().with_account(&account.id);
        let preview = service.preview(bytes, &ParseConfig::default(), Some(&mapping), &[])?;
        let options = ImportOptions {
            new_security_source: Some("yahoo".into()),
            ..ImportOptions::default()
        };
        let result = service.commit(&preview, &options)?;
        let ids: Vec<String> = store.list_accounts()?.into_iter().map(|a| a.id).collect();
        let written = store.transactions_for_accounts(&ids, None)?.len();
        assert_eq!(
            result.imported + result.updated,
            written,
            "{name}: the ledger holds a different number of rows than the commit reported"
        );
        Ok::<_, sq_core::error::Error>(Some((preview, written)))
    });
    match done {
        Err(_) => panic!("{name}: the import panicked on a file a user can pick"),
        Ok(Err(_)) | Ok(Ok(None)) => None,
        Ok(Ok(Some(pair))) => Some(pair),
    }
}

/// Every row is exactly one of the seven states, and the counts are that partition.
fn counts_partition_the_rows(name: &str, preview: &ImportPreview) {
    let s = &preview.summary;
    let accounted =
        s.ready + s.duplicates + s.similar + s.updated + s.unknown_securities + s.ignored + s.invalid;
    assert_eq!(
        accounted,
        s.total,
        "{name}: {} rows are in no state at all",
        s.total as i64 - accounted as i64
    );
    assert_eq!(
        s.total,
        preview.rows.len(),
        "{name}: the table and the summary disagree"
    );
}

/// A row that is going to be written must carry the draft that says what to write.
fn a_written_row_has_a_draft(name: &str, rows: &[ImportRow]) {
    for row in rows {
        if row.status == RowStatus::Ready || row.status == RowStatus::Updated {
            assert!(
                row.draft.is_some(),
                "{name}: row {} is ready with nothing to write",
                row.number
            );
        }
    }
}

/// A file the reader cannot make anything of at all: an error, never an empty success that
/// leaves the wizard offering a table of nothing.
#[test]
fn a_file_with_no_data_is_refused_rather_than_previewed_empty() {
    for (name, bytes) in [
        ("empty", b"".as_slice()),
        ("byte order mark alone", "\u{feff}".as_bytes()),
        ("blank lines only", b"\n\n\n".as_slice()),
    ] {
        assert!(parse(name, bytes).is_none(), "{name}: read as a file with data");
    }
}

/// A header and nothing under it is a legitimate export — a month with no activity.
#[test]
fn a_header_with_no_rows_is_a_file_with_nothing_to_import() {
    let parsed = parse("header only", b"date,type,amount\n").expect("header row refused");
    assert!(parsed.rows.is_empty());
    let (preview, written) = journey(
        "header only",
        b"date,type,symbol,quantity,price,currency,amount\n",
    )
    .expect("header row refused");
    assert_eq!(preview.summary.total, 0);
    assert_eq!(written, 0);
}

/// The four ways a file arrives that are not UTF-8 with Unix line endings. Every one of them is
/// somebody's bank export, and none of them is something the user can be asked to fix.
#[test]
fn encodings_and_line_endings_are_read_rather_than_refused() {
    let utf16 = {
        let mut v = vec![0xFF, 0xFE];
        for unit in "date,type,amount\n2024-01-15,BUY,10\n".encode_utf16() {
            v.extend_from_slice(&unit.to_le_bytes());
        }
        v
    };
    let latin1 = {
        let mut v = b"date,name,amount\n2024-01-15,".to_vec();
        v.extend_from_slice(&[0xC4, 0xD6, 0xDC]); // ÄÖÜ in ISO-8859-1
        v.extend_from_slice(b",10\n");
        v
    };
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("UTF-16 LE", utf16),
        ("ISO-8859-1", latin1),
        ("CRLF", b"date,type,amount\r\n2024-01-15,BUY,10\r\n".to_vec()),
        ("bare CR", b"date,type,amount\r2024-01-15,BUY,10\r".to_vec()),
        (
            "no newline at the end",
            b"date,type,amount\n2024-01-15,BUY,10".to_vec(),
        ),
        (
            "UTF-8 BOM",
            "\u{feff}date,type,amount\n2024-01-15,BUY,10\n"
                .as_bytes()
                .to_vec(),
        ),
    ];
    for (name, bytes) in cases {
        let parsed = parse(name, &bytes).unwrap_or_else(|| panic!("{name}: refused"));
        assert_eq!(parsed.rows.len(), 1, "{name}: lost the row");
        assert_eq!(parsed.headers[0], "date", "{name}: the first header is not clean");
    }
}

/// A header row that is not a list of distinct names. Neither shape may lose a column, because
/// the mapping is by name and a column nobody can name is a column nobody can map.
#[test]
fn a_header_row_that_repeats_or_omits_a_name_keeps_every_column() {
    let duplicate = parse("duplicate headers", b"date,amount,amount\n2024-01-15,10,20\n").unwrap();
    assert_eq!(
        duplicate.headers.len(),
        3,
        "a repeated name collapsed two columns"
    );
    let empty = parse("empty header cell", b"date,,amount\n2024-01-15,x,10\n").unwrap();
    assert_eq!(empty.headers.len(), 3);
    assert!(!empty.headers[1].is_empty(), "a nameless column cannot be mapped");
}

/// A row with more or fewer cells than the header. Both are common — a trailing delimiter, a
/// note containing the delimiter — and both must be said out loud rather than shifting values
/// one column to the left for the rest of the file.
#[test]
fn a_ragged_row_is_reported_rather_than_shifting_the_columns() {
    let extra = parse(
        "extra cells",
        b"date,type,amount\n2024-01-15,BUY,10,EXTRA\n2024-01-16,BUY,20\n",
    )
    .expect("refused");
    assert!(
        !extra.problems.is_empty(),
        "a row with an extra cell passed unremarked"
    );
    let short = parse(
        "missing cells",
        b"date,type,amount\n2024-01-15\n2024-01-16,BUY,20\n",
    )
    .expect("refused");
    assert_eq!(short.rows.len(), 2, "a short row took the next row's cells");
    assert_eq!(short.value(1, "amount"), Some("20"));
}

/// A quote nobody closed swallows the rest of the file. Losing rows silently is the worst
/// outcome an import can have, so the file must come back either refused or with a problem.
#[test]
fn an_unterminated_quote_does_not_silently_eat_the_file() {
    let body = b"date,note,amount\n\
2024-01-15,\"never closed,10\n\
2024-01-16,fine,20\n\
2024-01-17,fine,30\n";
    match parse("unterminated quote", body) {
        None => {} // refused: the user is told to look at the file
        Some(parsed) => assert!(
            parsed.rows.len() == 3 || !parsed.problems.is_empty(),
            "rows disappeared into an open quote with nothing said: {} rows, {} problems",
            parsed.rows.len(),
            parsed.problems.len()
        ),
    }
}

/// Numbers no arithmetic can hold. The row is refused; the file is not.
#[test]
fn a_number_outside_the_arithmetic_refuses_its_row_and_no_more() {
    let body = b"date,type,symbol,quantity,price,currency,amount\n\
2024-01-15,DEPOSIT,,,,USD,1e400\n\
2024-01-16,DEPOSIT,,,,USD,123456789012345678901234567890.1234\n\
2024-01-17,DEPOSIT,,,,USD,NaN\n\
2024-01-18,DEPOSIT,,,,USD,500\n";
    let (preview, written) = journey("impossible numbers", body).expect("the file was refused whole");
    counts_partition_the_rows("impossible numbers", &preview);
    a_written_row_has_a_draft("impossible numbers", &preview.rows);
    assert!(
        written >= 1,
        "the one good deposit was lost with the three bad ones"
    );
    assert!(
        written < 4,
        "a number no arithmetic can hold was written as a deposit"
    );
}

/// A date in a shape nobody declared. Every one of these is a real export; none of them may be
/// guessed into a wrong day, and every one of them must be visible in the wizard.
#[test]
fn a_date_that_cannot_be_read_is_a_problem_on_its_row() {
    for (name, cell) in [
        ("impossible day", "2024-13-45"),
        ("excel serial", "45000"),
        ("epoch millis", "1705276800000"),
        ("empty", ""),
        ("a word", "settled"),
    ] {
        let body = format!("date,type,symbol,quantity,price,currency,amount\n{cell},DEPOSIT,,,,USD,500\n");
        let (preview, written) = journey(name, body.as_bytes()).unwrap_or_else(|| panic!("{name}: refused"));
        counts_partition_the_rows(name, &preview);
        assert_eq!(
            written, 0,
            "{name}: an unreadable date was written as a day anyway"
        );
        assert!(
            preview.summary.invalid == 1 || !preview.problems.is_empty(),
            "{name}: read as a date with nothing said"
        );
    }
}

/// A cell megabytes long, and a file with hundreds of columns: neither is an attack, both are
/// what a statement with a free-text note looks like.
#[test]
fn an_enormous_cell_and_an_enormous_header_are_read() {
    let mut long = b"date,note,amount\n2024-01-15,".to_vec();
    long.extend(std::iter::repeat_n(b'x', 1_000_000));
    long.extend_from_slice(b",10\n");
    let parsed = parse("1 MB cell", &long).expect("refused");
    assert_eq!(parsed.rows.len(), 1);

    let head: Vec<String> = (0..500).map(|i| format!("c{i}")).collect();
    let row: Vec<String> = (0..500).map(|i| i.to_string()).collect();
    let wide = format!("{}\n{}\n", head.join(","), row.join(","));
    let parsed = parse("500 columns", wide.as_bytes()).expect("refused");
    assert_eq!(parsed.headers.len(), 500);
}

/// Bytes that are not a broker file at all. A user picks the wrong file — a PDF, an image, a
/// database — and gets an answer rather than a wizard full of nonsense.
#[test]
fn bytes_that_are_not_a_file_of_operations_never_reach_the_ledger() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "PDF",
            b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n1 0 obj\n<</Type/Catalog>>\n".to_vec(),
        ),
        (
            "PNG",
            vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13],
        ),
        (
            "SQLite",
            b"SQLite format 3\0\x10\0\x01\x01\0\x40\x20\x20".to_vec(),
        ),
        ("ZIP", b"PK\x03\x04\x14\0\0\0\x08\0".to_vec()),
        ("random bytes", (0u8..=255).cycle().take(4096).collect()),
        (
            "JSON that is not ours",
            br#"{"hello":[1,2,3],"nested":{"a":{"b":{"c":1}}}}"#.to_vec(),
        ),
        (
            "XML that is not Flex",
            b"<?xml version=\"1.0\"?><rss><channel/></rss>".to_vec(),
        ),
    ];
    for (name, bytes) in cases {
        if let Some((preview, written)) = journey(name, &bytes) {
            counts_partition_the_rows(name, &preview);
            assert_eq!(
                written, 0,
                "{name}: bytes that are not operations were written as operations"
            );
        }
    }
}

/// A Flex statement that is broken in the ways XML is broken.
#[test]
fn a_broken_flex_statement_is_refused_rather_than_half_read() {
    let cases: Vec<(&str, Vec<u8>)> = vec![
        ("truncated", b"<FlexQueryResponse><FlexStatements><FlexStatement><Trades><Trade sym".to_vec()),
        (
            "no statements",
            b"<FlexQueryResponse queryName=\"q\" type=\"AF\"></FlexQueryResponse>".to_vec(),
        ),
        (
            "deeply nested",
            {
                let mut v = b"<FlexQueryResponse>".to_vec();
                for _ in 0..5_000 {
                    v.extend_from_slice(b"<a>");
                }
                for _ in 0..5_000 {
                    v.extend_from_slice(b"</a>");
                }
                v.extend_from_slice(b"</FlexQueryResponse>");
                v
            },
        ),
        (
            "entity declaration",
            b"<!DOCTYPE r [<!ENTITY a \"aaaa\">]><FlexQueryResponse><FlexStatements>&a;</FlexStatements></FlexQueryResponse>".to_vec(),
        ),
    ];
    for (name, bytes) in cases {
        if let Some((preview, written)) = journey(name, &bytes) {
            counts_partition_the_rows(name, &preview);
            assert_eq!(written, 0, "{name}: a broken statement wrote operations");
        }
    }
}

/// The same file twice is the commonest thing a user does, and the second run must change
/// nothing at all — every row already in the ledger, the ledger the same size.
#[test]
fn importing_one_file_twice_writes_nothing_the_second_time() {
    let body = b"date,type,symbol,quantity,price,currency,fee,amount\n\
2024-01-15,BUY,AAPL,10,185.50,USD,4.95,1855\n\
2024-02-15,SELL,AAPL,5,192.00,USD,4.95,960\n\
2024-03-01,DEPOSIT,,,,USD,,5000\n\
2024-04-01,DIVIDEND,AAPL,5,0.24,USD,,1.20\n";
    let (store, account) = store_with_account();
    let service = ImportService::new(&store);
    let options = ImportOptions {
        new_security_source: Some("yahoo".into()),
        ..ImportOptions::default()
    };
    let detected = service.preview(body, &ParseConfig::default(), None, &[]).unwrap();
    let mapping = detected.mapping.clone().with_account(&account.id);

    let first = service
        .preview(body, &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    let wrote = service.commit(&first, &options).unwrap();
    let ids: Vec<String> = store.list_accounts().unwrap().into_iter().map(|a| a.id).collect();
    let after_first = store.transactions_for_accounts(&ids, None).unwrap().len();

    let second = service
        .preview(body, &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert_eq!(
        second.summary.duplicates, second.summary.total,
        "the second reading of the same file does not recognise every row"
    );
    let again = service.commit(&second, &options).unwrap();
    assert_eq!(
        again.imported, 0,
        "the same file imported a second time wrote rows"
    );
    let after_second = store.transactions_for_accounts(&ids, None).unwrap().len();
    assert_eq!(after_first, after_second, "the ledger grew on a re-import");
    assert_eq!(wrote.imported, after_first);
}
