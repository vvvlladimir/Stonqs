use crate::error::{Error, Result};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// What the account can hold. Deposit and securities accounts are separate: one
/// deposit account can back several depots, never the reverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountKind {
    /// Cash balance in one currency; never holds securities.
    Deposit,
    /// Holds instruments; settles through [`Account::reference_account_id`].
    Securities,
}

/// A brokerage or bank account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    /// Cash currency; for a depot, the settlement currency of its reference account.
    pub currency: Currency,
    pub kind: AccountKind,
    /// Required for [`AccountKind::Securities`], forbidden for [`AccountKind::Deposit`].
    #[serde(default)]
    pub reference_account_id: Option<String>,
    pub is_active: bool,
    pub opened_at: Option<NaiveDate>,
}

impl Account {
    pub fn deposit(name: impl Into<String>, currency: &str) -> Self {
        Account {
            id: super::new_id(),
            name: name.into(),
            currency: normalize_currency(currency),
            kind: AccountKind::Deposit,
            reference_account_id: None,
            is_active: true,
            opened_at: None,
        }
    }

    /// Reference is required here, not settable later — a depot without one can't record a purchase.
    pub fn securities(name: impl Into<String>, currency: &str, reference_account_id: &str) -> Self {
        Account {
            id: super::new_id(),
            name: name.into(),
            currency: normalize_currency(currency),
            kind: AccountKind::Securities,
            reference_account_id: Some(reference_account_id.to_string()),
            is_active: true,
            opened_at: None,
        }
    }

    /// Where this account's cash lands: itself for a deposit, its reference account for a depot.
    pub fn settlement_account_id(&self) -> &str {
        match self.kind {
            AccountKind::Deposit => &self.id,
            AccountKind::Securities => self.reference_account_id.as_deref().unwrap_or(&self.id),
        }
    }

    /// Lives on the model, not the host, so import and the CLI share the same rule.
    pub fn validate(&self) -> Result<()> {
        match (self.kind, self.reference_account_id.as_deref()) {
            (AccountKind::Securities, None) => Err(Error::Invalid(format!(
                "depot {:?} must reference a deposit account",
                self.name
            ))),
            (AccountKind::Deposit, Some(_)) => Err(Error::Invalid(format!(
                "deposit account {:?} cannot reference another account",
                self.name
            ))),
            (AccountKind::Securities, Some(reference)) if reference == self.id => Err(Error::Invalid(
                format!("account {:?} references itself", self.name),
            )),
            _ => Ok(()),
        }
    }
}

/// String, not int: a DB dump reading `"SECURITIES"` beats reading `0`.
impl AccountKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AccountKind::Deposit => "DEPOSIT",
            AccountKind::Securities => "SECURITIES",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "DEPOSIT" => Ok(AccountKind::Deposit),
            "SECURITIES" => Ok(AccountKind::Securities),
            other => Err(Error::Invalid(format!("unknown account kind {other:?}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn securities_account_requires_reference() {
        let cash = Account::deposit("Bank", "EUR");
        let depot = Account::securities("Broker", "EUR", &cash.id);
        assert!(cash.validate().is_ok());
        assert!(depot.validate().is_ok());
        assert_eq!(depot.settlement_account_id(), cash.id);

        let orphan = Account {
            reference_account_id: None,
            ..depot.clone()
        };
        assert!(orphan.validate().is_err());

        let self_ref = Account {
            reference_account_id: Some(depot.id.clone()),
            ..depot
        };
        assert!(self_ref.validate().is_err());
    }

    #[test]
    fn deposit_account_cannot_reference() {
        let a = Account::deposit("Bank", "EUR");
        let b = Account {
            reference_account_id: Some("other".into()),
            ..a
        };
        assert!(b.validate().is_err());
    }
}
