use serde::{Deserialize, Serialize};

/// A saved slice of accounts ("only IBKR", "only the pension"), not a second
/// portfolio: groups overlap freely, and deleting one touches no accounts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountGroup {
    pub id: String,
    pub name: String,
    pub account_ids: Vec<String>,
}

impl AccountGroup {
    pub fn new(name: impl Into<String>) -> Self {
        AccountGroup {
            id: super::new_id(),
            name: name.into(),
            account_ids: Vec::new(),
        }
    }

    pub fn with_accounts(mut self, ids: impl IntoIterator<Item = String>) -> Self {
        self.account_ids = ids.into_iter().collect();
        self
    }
}
