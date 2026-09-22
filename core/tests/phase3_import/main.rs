//! CSV import: detection, overrides, deduplication, and prices.
//! Fixtures mirror English comma-separated and German semicolon-separated broker exports.

mod basics;
mod basis;
mod brokers;
mod canonical;
mod external;
mod ibflex;
mod rules;
mod shapes;
mod signs;

use rust_decimal_macros::dec;
use sq_core::calc::{build_holdings, income_by_kind};
use sq_core::fx::FxRate;
use sq_core::import::{
    AmountBasis, AmountSign, ImportField, ImportMapping, ImportOptions, ImportService, ParseConfig,
    PriceMapping, ProblemCode, RowOverride, RowStatus, SecurityDraft, Severity, canonical_to_file,
};
use sq_core::model::{Account, Security, SecurityKind, Transaction, TransactionKind};
use sq_core::storage::Store;

const PLAIN_STYLE: &str = "\
date,type,symbol,quantity,unit_price,currency,fee,amount,comment
2024-01-15,BUY,AAPL,10,185.50,USD,4.95,,Initial Apple purchase
2024-02-15,SELL,AAPL,5,192.00,USD,4.95,,Partial Apple sale
2024-02-01,DIVIDEND,AAPL,10,0.24,USD,,,Q4 dividend
2024-03-01,DEPOSIT,,,,USD,,5000,Monthly contribution
2024-05-15,SPLIT,NVDA,,,USD,,3,3-for-1 stock split
";

/// A file's header row, the way every fixture here starts.
fn headers_of(csv: &str) -> Vec<String> {
    csv.lines()
        .next()
        .unwrap()
        .split(',')
        .map(str::to_string)
        .collect()
}

fn store_with_account() -> (Store, Account) {
    let store = Store::open_in_memory().unwrap();
    let account = depot(&store, "Broker", "USD");
    (store, account)
}

/// Depot plus its cash account: one broker from the user's perspective.
fn pair(depot: &Account) -> Vec<String> {
    vec![
        depot.id.clone(),
        depot
            .reference_account_id
            .clone()
            .expect("depot without a settlement account"),
    ]
}

/// Depot together with its cash account; cash rows are routed to the linked account.
fn depot(store: &Store, name: &str, currency: &str) -> Account {
    let cash = Account::deposit(format!("{name} · cash"), currency);
    store.save_account(&cash).unwrap();
    let account = Account::securities(name, currency, &cash.id);
    store.save_account(&account).unwrap();
    account
}
