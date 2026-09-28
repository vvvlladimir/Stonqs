//! One layer above the reader: detection, mapping, the drafts and every check. This is the call
//! the wizard makes on each change of the layout, and the one that decides what will be written.
//! Nothing here touches the database beyond an in-memory one built per case.
#![no_main]

use libfuzzer_sys::fuzz_target;
use sq_core::import::{ImportService, ParseConfig};
use sq_core::model::Account;
use sq_core::storage::Store;

fuzz_target!(|data: &[u8]| {
    let Ok(store) = Store::open_in_memory() else {
        return;
    };
    let cash = Account::deposit("cash", "USD");
    if store.save_account(&cash).is_err() {
        return;
    }
    let account = Account::securities("Broker", "USD", &cash.id);
    if store.save_account(&account).is_err() {
        return;
    }
    let service = ImportService::new(&store);
    let Ok(detected) = service.preview(data, &ParseConfig::default(), None, &[]) else {
        return;
    };
    let mapping = detected.mapping.with_account(&account.id);
    let _ = service.preview(data, &ParseConfig::default(), Some(&mapping), &[]);
});
