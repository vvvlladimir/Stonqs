use super::TransactionDraft;
use crate::model::Transaction;
use rust_decimal::Decimal;

/// One operation already in the database, as identity alone: what it is called by the broker,
/// which row it is, and what it currently says. The preview needs no more of it than that.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownRow {
    pub external_id: String,
    pub transaction_id: String,
    pub fingerprint: String,
}

impl KnownRow {
    /// `None` for a transaction the broker never named — it is recognised by its content.
    pub fn of(t: &Transaction) -> Option<Self> {
        Some(KnownRow {
            external_id: t.external_id.clone()?,
            transaction_id: t.id.clone(),
            fingerprint: fingerprint_of(t),
        })
    }
}

/// Stable, human-readable identity for an imported transaction; see ADR 0005.
pub fn fingerprint(draft: &TransactionDraft) -> String {
    // Keep the identity readable so duplicate decisions can be explained.
    let security = draft
        .security_id
        .as_deref()
        .or(draft.symbol.as_deref())
        .unwrap_or("-");
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        draft.account_id,
        draft.date,
        draft.kind.as_str(),
        security,
        number(draft.quantity),
        number(draft.amount),
        draft.currency
    )
}
/// Computes the same identity for a transaction already in storage.
pub fn fingerprint_of(transaction: &Transaction) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}",
        transaction.account_id,
        transaction.date,
        transaction.kind.as_str(),
        transaction.security_id.as_deref().unwrap_or("-"),
        number(transaction.quantity),
        number(transaction.amount),
        transaction.currency
    )
}
/// Normalizes equivalent decimal spellings before comparison.
fn number(v: Decimal) -> String {
    v.normalize().to_string()
}
