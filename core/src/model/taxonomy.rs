use crate::error::{Error, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaxonomyKind {
    AssetClass,
    Region,
    Sector,
    /// User-defined tree.
    Custom,
}

/// A classification tree; a user has several, independent of each other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Taxonomy {
    pub id: String,
    pub name: String,
    pub kind: TaxonomyKind,
}

impl Taxonomy {
    pub fn new(name: impl Into<String>, kind: TaxonomyKind) -> Self {
        Taxonomy {
            id: super::new_id(),
            name: name.into(),
            kind,
        }
    }
}

/// A tree node; `parent_id == None` is a root. Self-reference instead of fixed levels — depth is unbounded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonomyNode {
    pub id: String,
    pub taxonomy_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    /// Sibling order, user-chosen, not alphabetical.
    pub rank: i64,
    /// Palette slot (1..8), not a HEX — colors belong to the theme. `None` = derive from node order.
    pub color: Option<i64>,
}

impl TaxonomyNode {
    pub fn root(taxonomy_id: &str, name: impl Into<String>) -> Self {
        TaxonomyNode {
            id: super::new_id(),
            taxonomy_id: taxonomy_id.to_string(),
            parent_id: None,
            name: name.into(),
            rank: 0,
            color: None,
        }
    }

    pub fn child(parent: &TaxonomyNode, name: impl Into<String>) -> Self {
        TaxonomyNode {
            id: super::new_id(),
            taxonomy_id: parent.taxonomy_id.clone(),
            parent_id: Some(parent.id.clone()),
            name: name.into(),
            rank: 0,
            color: None,
        }
    }

    pub fn with_rank(mut self, rank: i64) -> Self {
        self.rank = rank;
        self
    }
}

/// Assigns a security to a node with a weight (a global ETF is ~60% US, 40% elsewhere — one category would misreport region allocation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityClassification {
    pub security_id: String,
    pub node_id: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub weight: Decimal,
}

impl SecurityClassification {
    pub fn new(security_id: &str, node_id: &str, weight: Decimal) -> Self {
        SecurityClassification {
            security_id: security_id.to_string(),
            node_id: node_id.to_string(),
            weight,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.weight <= Decimal::ZERO || self.weight > Decimal::ONE {
            return Err(Error::Invalid(format!(
                "classification weight must be in (0, 1], got {}",
                self.weight
            )));
        }
        Ok(())
    }
}

const CASH_SUBJECT_PREFIX: &str = "cash:";

/// Cash subject key: `cash:<account>:<currency>` — see CLAUDE.md `TaxonomySubject` invariant.
pub fn cash_subject_key(account_id: &str, currency: &str) -> String {
    format!(
        "{CASH_SUBJECT_PREFIX}{account_id}:{}",
        crate::money::normalize_currency(currency)
    )
}

/// `None` means the key is a security, not cash.
pub fn parse_cash_subject(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix(CASH_SUBJECT_PREFIX)?;
    let (account_id, currency) = rest.rsplit_once(':')?;
    Some((account_id, currency))
}

/// Assigns an account's cash balance (per currency) to a node with a weight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashClassification {
    pub account_id: String,
    pub currency: String,
    pub node_id: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub weight: Decimal,
}

impl CashClassification {
    pub fn new(account_id: &str, currency: &str, node_id: &str, weight: Decimal) -> Self {
        CashClassification {
            account_id: account_id.to_string(),
            currency: crate::money::normalize_currency(currency),
            node_id: node_id.to_string(),
            weight,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.weight <= Decimal::ZERO || self.weight > Decimal::ONE {
            return Err(Error::Invalid(format!(
                "classification weight must be in (0, 1], got {}",
                self.weight
            )));
        }
        Ok(())
    }
}

impl TaxonomyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TaxonomyKind::AssetClass => "ASSET_CLASS",
            TaxonomyKind::Region => "REGION",
            TaxonomyKind::Sector => "SECTOR",
            TaxonomyKind::Custom => "CUSTOM",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "ASSET_CLASS" => TaxonomyKind::AssetClass,
            "REGION" => TaxonomyKind::Region,
            "SECTOR" => TaxonomyKind::Sector,
            "CUSTOM" => TaxonomyKind::Custom,
            other => return Err(Error::Invalid(format!("unknown taxonomy kind {other:?}"))),
        })
    }
}
