use crate::error::{Error, Result};
use crate::money::{Currency, normalize_currency};
use serde::{Deserialize, Serialize};

/// Both methods are "correct" but answer different questions: FIFO is what
/// most tax jurisdictions require and gives a holding period per lot;
/// average cost is a common default elsewhere and is needed to
/// reconcile against history already accumulated there. Lives on the
/// portfolio, not globally — the same trades can legitimately be viewed
/// either way.
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

/// A set of accounts plus a reporting base currency.
///
/// Base currency lives here, not in global settings — the same accounts can
/// legitimately be reported in EUR and in USD as two different portfolios.
/// The calc engine never assumes a currency; it's always passed in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Portfolio {
    pub id: String,
    pub name: String,
    pub base_currency: Currency,
    /// An account can belong to several portfolios, hence a list here rather than a field on Account.
    pub account_ids: Vec<String>,
    #[serde(default)]
    pub cost_basis_method: CostBasisMethod,
    /// Where the owner spends, as a consumer-price region (`DE`, `EA`, `US`). It is not derived
    /// from `base_currency`: reporting in USD says nothing about which prices the owner pays.
    /// `None` means real returns are not reported.
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
