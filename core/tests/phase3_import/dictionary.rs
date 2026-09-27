//! Operation wordings added from outside the binary — a plugin's dictionary for a language the
//! shipped keywords do not speak. Added words fill gaps; they never re-answer a wording the app
//! already reads.

use super::*;
use sq_core::import::KindWords;

const FINNISH: &str = "\
date,type,symbol,quantity,unit_price,currency,amount
2024-01-15,Osto,AAPL,10,185.50,USD,
2024-02-15,Myynti,AAPL,5,192.00,USD,
2024-03-01,Talletus,,,,USD,5000
2024-03-02,Nosto,,,,USD,1000
2024-04-01,BUY,AAPL,1,190.00,USD,
";

fn finnish() -> KindWords {
    // "Nosto" (withdrawal) contains "Osto" (buy), so the longer word is listed first — the same
    // rule the shipped table follows for "Verkoop" and "Koop".
    KindWords::new([
        ("Nosto", TransactionKind::Withdrawal),
        ("Osto", TransactionKind::Buy),
        ("Myynti", TransactionKind::Sell),
        ("Talletus", TransactionKind::Deposit),
        // A stranger's package claiming the app's own word: never reached.
        ("BUY", TransactionKind::Sell),
    ])
}

/// A word short enough to be inside almost any wording is not read: "E" as a deposit would turn
/// every wording nobody answered into one. Two ideographs are a whole word, and are read.
#[test]
fn a_word_too_short_to_mean_anything_is_not_read() {
    let words = KindWords::new([
        ("e", TransactionKind::Deposit),
        ("to", TransactionKind::Buy),
        ("买入", TransactionKind::Buy),
        ("Talletus", TransactionKind::Deposit),
    ]);
    assert_eq!(words.too_short(), ["E", "TO"]);

    let mapping = ImportMapping::default()
        .with_detected_kinds_using(["Monthly statement", "买入股票", "Talletus"].into_iter(), &words);
    let aliases = &mapping.kind_aliases;
    assert_eq!(
        aliases.get("MONTHLYSTATEMENT"),
        None,
        "not a deposit because it holds an E"
    );
    assert_eq!(aliases.get("买入股票"), Some(&TransactionKind::Buy));
    assert_eq!(aliases.get("TALLETUS"), Some(&TransactionKind::Deposit));
}

fn kind_of(preview: &ImportPreview, value: &str) -> Option<TransactionKind> {
    preview
        .kinds
        .iter()
        .find(|k| k.value == value)
        .and_then(|k| k.kind)
}

#[test]
fn a_dictionary_answers_wordings_the_shipped_keywords_do_not() {
    let (store, account) = store_with_account();
    let mapping = ImportMapping::detect(&headers_of(FINNISH)).with_account(&account.id);

    let without = ImportService::new(&store)
        .preview(FINNISH.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert_eq!(
        without.unknown_kinds().len(),
        4,
        "no Finnish wording is known to the app: {:?}",
        without.unknown_kinds()
    );

    let with = ImportService::new(&store)
        .with_kind_dictionary(finnish())
        .preview(FINNISH.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert!(with.unknown_kinds().is_empty(), "{:?}", with.unknown_kinds());
    assert_eq!(kind_of(&with, "Osto"), Some(TransactionKind::Buy));
    assert_eq!(kind_of(&with, "Myynti"), Some(TransactionKind::Sell));
    assert_eq!(kind_of(&with, "Talletus"), Some(TransactionKind::Deposit));
    assert_eq!(
        kind_of(&with, "Nosto"),
        Some(TransactionKind::Withdrawal),
        "order decides between nested words"
    );
    assert_eq!(
        kind_of(&with, "BUY"),
        Some(TransactionKind::Buy),
        "an added word never re-answers what the app already reads"
    );
    assert_eq!(with.summary.invalid, 0);
}

#[test]
fn a_word_the_shipped_keywords_answer_otherwise_is_reported_as_shadowed() {
    let words = finnish();
    assert_eq!(words.shadowed(), vec![("BUY", TransactionKind::Buy)]);
}

#[test]
fn a_dictionary_reads_from_its_file_in_order() {
    let words: KindWords = serde_json::from_str(
        r#"{"words":[{"word":"Nosto","kind":"WITHDRAWAL"},{"word":"Osto","kind":"BUY"}]}"#,
    )
    .unwrap();
    assert_eq!(
        words,
        KindWords::new([
            ("NOSTO", TransactionKind::Withdrawal),
            ("OSTO", TransactionKind::Buy)
        ])
    );
}
