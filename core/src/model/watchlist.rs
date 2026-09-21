use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A named list of instruments the user follows, held or not. It moves no money and is not
/// scoped: what it shows is read off each instrument's own quotes. See ADR-0035.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Watchlist {
    pub id: String,
    pub name: String,
    /// In the user's order.
    pub security_ids: Vec<String>,
}

impl Watchlist {
    pub fn new(name: impl Into<String>) -> Self {
        Watchlist {
            id: super::new_id(),
            name: name.into(),
            security_ids: Vec::new(),
        }
    }

    pub fn with(mut self, security_id: &str) -> Self {
        self.security_ids.push(security_id.to_string());
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::Invalid("a watchlist needs a name".into()));
        }
        let mut seen = HashSet::new();
        if let Some(twice) = self.security_ids.iter().find(|id| !seen.insert(id.as_str())) {
            return Err(Error::Invalid(format!("instrument {twice} is on the list twice")));
        }
        Ok(())
    }
}
