//! Domain model: accounts, securities, transactions, positions, portfolio.
//!
//! Module-wide rules: money is always [`rust_decimal::Decimal`], never
//! `f64` (`0.1 + 0.2 != 0.3` is unacceptable for accounting); every struct
//! derives `Serialize`/`Deserialize` up front, since retrofitting it later
//! means touching every type at once; ids are `String` (UUID v4) so an
//! object can exist before it's written to the database.

mod account;
mod account_group;
mod ai_chat;
mod alert;
mod attribute;
mod corporate_action;
mod event;
mod goal;
mod plan;
mod portfolio;
mod position;
mod security;
mod target;
mod taxonomy;
mod transaction;
mod watchlist;

pub use account::{Account, AccountKind};
pub use account_group::AccountGroup;
pub use ai_chat::{AiChat, AiEffort, AiMessage, AiToolMode, AiUsage, AiUsageTotal};
pub use alert::{AlertCrossing, AlertDirection, AlertKind, AlertSide, CrossingDirection, SecurityAlert};
pub use attribute::{AttributeKind, SecurityAttributeDef};
pub use corporate_action::{CorporateAction, CorporateActionKind};
pub use event::{SecurityEvent, SecurityEventKind};
pub use goal::{ContributionLimit, Goal};
pub use plan::{Interval, InvestmentPlan, PlanLeg, Schedule};
pub use portfolio::{CostBasisMethod, Portfolio};
pub use position::{Lot, Position};
pub use security::{Security, SecurityKind, floor_to_step, is_isin, observed_quantity_step};
pub use target::{AllocationTarget, TargetWeight};
pub use taxonomy::{
    CashClassification, SecurityClassification, Taxonomy, TaxonomyKind, TaxonomyNode, cash_subject_key,
    parse_cash_subject,
};
pub use transaction::{Transaction, TransactionKind};
pub use watchlist::Watchlist;

pub fn new_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Case fold for a consumer-price region. Codes are ISO 3166 alpha-2 plus the two aggregates
/// Eurostat publishes (`EA`, `EU`); which of them a source can answer is the source's business.
pub fn normalize_region(region: &str) -> String {
    region.trim().to_ascii_uppercase()
}
