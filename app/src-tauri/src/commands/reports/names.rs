//! The names a person reads a row by.
//!
//! The core answers in ids, and an id means nothing on screen, so every row on this screen
//! carries the ticker, the instrument's name and the account's label beside the figures. An id
//! nothing answers to is shown as itself rather than as a blank — a row that vanishes is worse
//! than a row with an ugly label.

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
