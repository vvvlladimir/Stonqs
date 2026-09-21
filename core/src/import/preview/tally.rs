//! What the file says, counted: every distinct operation wording, account label and ticker, with
//! how often it occurs and what it resolves to. This is what the wizard's mapping panels list.

use super::{AccountMapping, KindMapping, SymbolMapping};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Tallies {
    pub kinds: BTreeMap<String, KindMapping>,
    pub accounts: BTreeMap<String, AccountMapping>,
    pub symbols: BTreeMap<String, SymbolMapping>,
}

impl Tallies {
    /// Most frequent first, ties broken by the value itself so the same file always previews the
    /// same way.
    pub fn into_sorted(self) -> (Vec<KindMapping>, Vec<SymbolMapping>, Vec<AccountMapping>) {
        let mut kinds: Vec<KindMapping> = self.kinds.into_values().collect();
        kinds.sort_by(|a, b| b.count.cmp(&a.count).then(a.value.cmp(&b.value)));
        let mut symbols: Vec<SymbolMapping> = self.symbols.into_values().collect();
        symbols.sort_by(|a, b| b.count.cmp(&a.count).then(a.value.cmp(&b.value)));
        let mut accounts: Vec<AccountMapping> = self.accounts.into_values().collect();
        accounts.sort_by(|a, b| b.count.cmp(&a.count).then(a.value.cmp(&b.value)));
        (kinds, symbols, accounts)
    }
}
