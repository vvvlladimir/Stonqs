use crate::error::{Error, Result};
use crate::money::{Currency, normalize_currency};
use serde::{Deserialize, Serialize};

/// FIFO or average cost; per portfolio, since the same trades may be viewed either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CostBasisMethod {
    /// Oldest lot disposed of first.
    #[default]
    Fifo,
    /// All shares treated as one pool; per-unit cost is the weighted
    /// average of purchases. A sale doesn't shift it, a new buy does.
    AverageCost,
}

impl CostBasisMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            CostBasisMethod::Fifo => "FIFO",
            CostBasisMethod::AverageCost => "AVERAGE_COST",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "FIFO" => CostBasisMethod::Fifo,
            "AVERAGE_COST" => CostBasisMethod::AverageCost,
            other => return Err(Error::Invalid(format!("unknown cost basis method {other:?}"))),
        })
    }
}

/// Accounts plus a reporting base currency; calc never assumes a currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Portfolio {
    pub id: String,
    pub name: String,
    pub base_currency: Currency,
    /// An account can belong to several portfolios, hence a list here rather than a field on Account.
    pub account_ids: Vec<String>,
    #[serde(default)]
    pub cost_basis_method: CostBasisMethod,
    /// The owner's consumer-price region, independent of `base_currency`; `None` = no real returns.
    #[serde(default)]
    pub inflation_region: Option<String>,
}

impl Portfolio {
    pub fn new(name: impl Into<String>, base_currency: &str) -> Self {
        Portfolio {
            id: super::new_id(),
            name: name.into(),
            base_currency: normalize_currency(base_currency),
            account_ids: Vec::new(),
            cost_basis_method: CostBasisMethod::default(),
            inflation_region: None,
        }
    }

    pub fn with_inflation_region(mut self, region: &str) -> Self {
        self.inflation_region = Some(super::normalize_region(region));
        self
    }

    pub fn with_cost_basis(mut self, method: CostBasisMethod) -> Self {
        self.cost_basis_method = method;
        self
    }

    pub fn with_accounts(mut self, ids: impl IntoIterator<Item = String>) -> Self {
        self.account_ids = ids.into_iter().collect();
        self
    }
}
