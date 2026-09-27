//! Readable names for rows the core answers in ids; an unknown id is shown as itself.

use crate::error::UiResult;
use sq_core::storage::Store;
use std::collections::HashMap;

pub(super) struct Names {
    securities: HashMap<String, (String, String)>,
    accounts: HashMap<String, String>,
}

impl Names {
    pub fn of(store: &Store) -> UiResult<Self> {
        Ok(Names {
            securities: store
                .list_securities()?
                .into_iter()
                .map(|s| (s.id, (s.symbol, s.name)))
                .collect(),
            accounts: store
                .list_accounts()?
                .into_iter()
                .map(|a| (a.id, a.name))
                .collect(),
        })
    }

    /// Ticker and name. An unknown id stands in for its own ticker and has no name.
    pub fn security(&self, id: &str) -> (String, String) {
        self.securities
            .get(id)
            .cloned()
            .unwrap_or_else(|| (id.to_string(), String::new()))
    }

    pub fn account(&self, id: &str) -> String {
        self.accounts.get(id).cloned().unwrap_or_else(|| id.to_string())
    }
}
