//! Properties, not examples. A fixture says what one broker's file does; these say what *every*
//! file must do, over files nobody wrote by hand. Four of them are the ones a user feels:
//! reading a file twice gives the same answer, the order of the rows in it does not change the
//! ledger, importing it twice writes it once, and what the app exports it can read back.
//!
//! Generated cases are deliberately small and plausible — the hostile ones live in
//! `robustness.rs`. What is generated here is the *shape* of a broker file: which columns it
//! carries, how it spells a number and a date, what it calls an operation.

use super::*;
use proptest::prelude::*;
use sq_core::model::Transaction;

/// One operation as a broker might print it, before it is spelled in any particular way.
#[derive(Debug, Clone)]
struct Op {
    day: u32,
    month: u32,
    kind: &'static str,
    symbol: &'static str,
    quantity: u32,
    price: u32,
    cents: u32,
}

impl Op {
    /// The file's own spelling of this operation: comma or semicolon, dot or comma decimals,
    /// ISO or day-first dates.
    fn row(&self, style: &Style) -> String {
        let amount = self.amount();
        let cell = |v: String| v;
        let money = |v: String| {
            if style.decimal_comma {
                v.replace('.', ",")
            } else {
                v
            }
        };
        let date = if style.day_first {
            format!("{:02}/{:02}/2024", self.day, self.month)
        } else {
            format!("2024-{:02}-{:02}", self.month, self.day)
        };
        let (symbol, quantity, price) = if self.kind == "DEPOSIT" || self.kind == "WITHDRAWAL" {
            (String::new(), String::new(), String::new())
        } else {
            (
                self.symbol.to_string(),
                self.quantity.to_string(),
                money(format!("{}.{:02}", self.price, self.cents)),
            )
        };
        [
            cell(date),
            cell(self.kind.to_string()),
            symbol,
            quantity,
            price,
            "USD".to_string(),
            money(amount),
        ]
        .join(&style.delimiter.to_string())
    }

    fn amount(&self) -> String {
        if self.kind == "DEPOSIT" || self.kind == "WITHDRAWAL" {
            return format!("{}.00", self.price);
        }
        let cents = u64::from(self.quantity) * (u64::from(self.price) * 100 + u64::from(self.cents));
        format!("{}.{:02}", cents / 100, cents % 100)
    }
}

#[derive(Debug, Clone)]
struct Style {
    delimiter: char,
    decimal_comma: bool,
    day_first: bool,
}

fn op() -> impl Strategy<Value = Op> {
    (
        1u32..=28,
        1u32..=12,
        prop::sample::select(vec!["BUY", "SELL", "DIVIDEND", "DEPOSIT", "WITHDRAWAL"]),
        prop::sample::select(vec!["AAPL", "MSFT", "VWCE", "SPY"]),
        1u32..=50,
        1u32..=900,
        0u32..=99,
    )
        .prop_map(|(day, month, kind, symbol, quantity, price, cents)| Op {
            day,
            month,
            kind,
            symbol,
            quantity,
            price,
            cents,
        })
}

fn style() -> impl Strategy<Value = Style> {
    (prop::sample::select(vec![',', ';']), any::<bool>(), any::<bool>()).prop_map(
        |(delimiter, decimal_comma, day_first)| Style {
            delimiter,
            // A comma decimal separator in a comma-separated file needs quoting nobody prints.
            decimal_comma: decimal_comma && delimiter == ';',
            day_first,
        },
    )
}

fn file_of(ops: &[Op], style: &Style) -> Vec<u8> {
    let header = [
        "date", "type", "symbol", "quantity", "price", "currency", "amount",
    ]
    .join(&style.delimiter.to_string());
    let mut out = header;
    for op in ops {
        out.push('\n');
        out.push_str(&op.row(style));
    }
    out.push('\n');
    out.into_bytes()
}

fn options() -> ImportOptions {
    ImportOptions {
        new_security_source: Some("yahoo".into()),
        ..ImportOptions::default()
    }
}

/// The ledger as a value that does not depend on generated ids or on the order rows were
/// written in: what a user would compare two imports by.
fn ledger(store: &Store) -> Vec<String> {
    let ids: Vec<String> = store.list_accounts().unwrap().into_iter().map(|a| a.id).collect();
    // The instrument by its ticker: an id is minted per store, so two imports of one file would
    // differ in nothing else.
    let symbols: std::collections::HashMap<String, String> = store
        .list_securities()
        .unwrap()
        .into_iter()
        .map(|s| (s.id, s.symbol))
        .collect();
    let mut rows: Vec<String> = store
        .transactions_for_accounts(&ids, None)
        .unwrap()
        .iter()
        .map(|t: &Transaction| {
            let security = t
                .security_id
                .as_ref()
                .and_then(|id| symbols.get(id))
                .map_or("-", String::as_str);
            format!(
                "{}|{}|{}|{}|{}|{}",
                t.date,
                t.kind.as_str(),
                security,
                t.quantity.normalize(),
                t.amount.normalize(),
                t.currency
            )
        })
        .collect();
    rows.sort();
    rows
}

/// Imports a whole file into a fresh store and answers with what the ledger then holds.
fn import(bytes: &[u8]) -> Result<Vec<String>, sq_core::error::Error> {
    let (store, account) = store_with_account();
    let service = ImportService::new(&store);
    let detected = service.preview(bytes, &ParseConfig::default(), None, &[])?;
    let mapping = detected.mapping.clone().with_account(&account.id);
    let preview = service.preview(bytes, &ParseConfig::default(), Some(&mapping), &[])?;
    service.commit(&preview, &options())?;
    Ok(ledger(&store))
}

proptest! {
    // Generated files are cheap; a store is not. Enough cases to shake a shape loose, not a
    // benchmark.
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    /// The wizard re-previews on every change. Two readings of one file must be the same
    /// reading, or the table under the user's hands changes when nothing did.
    #[test]
    fn reading_one_file_twice_gives_the_same_preview(ops in prop::collection::vec(op(), 1..12), style in style()) {
        let bytes = file_of(&ops, &style);
        let (store, account) = store_with_account();
        let service = ImportService::new(&store);
        let detected = service.preview(&bytes, &ParseConfig::default(), None, &[]).unwrap();
        let mapping = detected.mapping.clone().with_account(&account.id);
        let first = service.preview(&bytes, &ParseConfig::default(), Some(&mapping), &[]).unwrap();
        let second = service.preview(&bytes, &ParseConfig::default(), Some(&mapping), &[]).unwrap();
        prop_assert_eq!(first.summary, second.summary);
        prop_assert_eq!(first.rows, second.rows);
        prop_assert_eq!(first.mapping, second.mapping);
    }

    /// A broker prints its statement newest first or oldest first, and some print neither. The
    /// ledger must not be able to tell.
    #[test]
    fn the_order_of_the_rows_does_not_change_the_ledger(ops in prop::collection::vec(op(), 1..12), style in style()) {
        let forward = file_of(&ops, &style);
        let reversed: Vec<Op> = ops.iter().rev().cloned().collect();
        let reversed = file_of(&reversed, &style);
        match (import(&forward), import(&reversed)) {
            (Ok(a), Ok(b)) => prop_assert_eq!(a, b),
            (Err(_), Err(_)) => {}
            (a, b) => prop_assert!(false, "one order imported and the other did not: {:?} / {:?}", a.is_ok(), b.is_ok()),
        }
    }

    /// The commonest thing a user does after an import is the same import again.
    #[test]
    fn importing_a_file_twice_writes_it_once(ops in prop::collection::vec(op(), 1..12), style in style()) {
        let bytes = file_of(&ops, &style);
        let (store, account) = store_with_account();
        let service = ImportService::new(&store);
        let detected = service.preview(&bytes, &ParseConfig::default(), None, &[]).unwrap();
        let mapping = detected.mapping.clone().with_account(&account.id);
        let first = service.preview(&bytes, &ParseConfig::default(), Some(&mapping), &[]).unwrap();
        service.commit(&first, &options()).unwrap();
        let after_one = ledger(&store);

        let second = service.preview(&bytes, &ParseConfig::default(), Some(&mapping), &[]).unwrap();
        let again = service.commit(&second, &options()).unwrap();
        prop_assert_eq!(again.imported, 0);
        prop_assert_eq!(after_one, ledger(&store));
    }

    /// What the app writes, the app reads: an export re-imported into an empty portfolio must
    /// produce the same ledger, or a backup is not one.
    #[test]
    fn a_canonical_export_imports_back_to_the_same_ledger(ops in prop::collection::vec(op(), 1..12), style in style()) {
        let bytes = file_of(&ops, &style);
        let (store, account) = store_with_account();
        let service = ImportService::new(&store);
        let detected = service.preview(&bytes, &ParseConfig::default(), None, &[]).unwrap();
        let mapping = detected.mapping.clone().with_account(&account.id);
        let preview = service.preview(&bytes, &ParseConfig::default(), Some(&mapping), &[]).unwrap();
        service.commit(&preview, &options()).unwrap();

        let accounts = store.list_accounts().unwrap();
        let ids: Vec<String> = accounts.iter().map(|a| a.id.clone()).collect();
        let written = store.transactions_for_accounts(&ids, None).unwrap();
        let exported = canonical_to_file(&written, &accounts, &store.list_securities().unwrap()).unwrap();
        let before = ledger(&store);

        let (fresh, _) = store_with_account();
        let service = ImportService::new(&fresh);
        let detected = service.preview(exported.as_bytes(), &ParseConfig::default(), None, &[]).unwrap();
        let read_back = service
            .preview(exported.as_bytes(), &ParseConfig::default(), Some(&detected.mapping), &[])
            .unwrap();
        prop_assert_eq!(read_back.summary.invalid, 0, "the app cannot read its own export");
        service.commit(&read_back, &options()).unwrap();
        prop_assert_eq!(before, ledger(&fresh));
    }

    /// Every row the preview calls ready reaches the ledger, and nothing else does: the count
    /// on the last screen of the wizard is what the user checks the import by.
    #[test]
    fn the_summary_is_what_the_commit_writes(ops in prop::collection::vec(op(), 1..12), style in style()) {
        let bytes = file_of(&ops, &style);
        let (store, account) = store_with_account();
        let service = ImportService::new(&store);
        let detected = service.preview(&bytes, &ParseConfig::default(), None, &[]).unwrap();
        let mapping = detected.mapping.clone().with_account(&account.id);
        let preview = service.preview(&bytes, &ParseConfig::default(), Some(&mapping), &[]).unwrap();
        let expected = preview.summary.ready + preview.summary.unknown_securities;
        let result = service.commit(&preview, &options()).unwrap();
        prop_assert_eq!(result.imported, expected);
        prop_assert_eq!(ledger(&store).len(), expected);
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// Arbitrary bytes are a file the user can pick — the wrong one, a truncated download, a
    /// file the disk half wrote. The reader answers or refuses; it never comes apart.
    #[test]
    fn arbitrary_bytes_never_bring_the_reader_down(bytes in prop::collection::vec(any::<u8>(), 0..2048)) {
        let read = std::panic::catch_unwind(|| parse_file(&bytes, &ParseConfig::default()));
        prop_assert!(read.is_ok(), "the reader panicked on {} bytes", bytes.len());
    }

    /// The same, one layer up: whatever the reader made of those bytes, the preview must survive
    /// it — that is the call the wizard makes on every keystroke.
    #[test]
    fn arbitrary_text_never_brings_the_preview_down(text in "\\PC{0,400}") {
        let read = std::panic::catch_unwind(|| {
            let (store, account) = store_with_account();
            let service = ImportService::new(&store);
            let detected = service.preview(text.as_bytes(), &ParseConfig::default(), None, &[])?;
            let mapping = detected.mapping.clone().with_account(&account.id);
            service.preview(text.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        });
        prop_assert!(read.is_ok(), "the preview panicked on {text:?}");
    }
}
