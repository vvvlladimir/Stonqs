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
/// Identity without what the operation is *worth*: account, day, kind, instrument, quantity.
/// A row edited by hand after it was imported — a corrected price, a commission typed in — no
/// longer matches its own file by [`fingerprint`], so re-importing that file would write it a
/// second time. This is what catches it.
///
/// Only a share movement has one: a quantity is what makes two operations of the same day
/// comparable. Two cash rows differing only in amount are two different payments, and treating
/// them as one would hide a real second dividend.
pub fn loose_fingerprint(draft: &TransactionDraft) -> Option<String> {
    // Keyed on the stored instrument alone, like its counterpart: a draft that resolved to no
    // instrument names one the database does not have, so nothing there can be its twin.
    let security = draft.security_id.as_deref()?;
    (!draft.quantity.is_zero()).then(|| {
        format!(
            "{}|{}|{}|{}|{}",
            draft.account_id,
            draft.date,
            draft.kind.as_str(),
            security,
            number(draft.quantity)
        )
    })
}

/// The same identity for a transaction already in storage.
pub fn loose_fingerprint_of(transaction: &Transaction) -> Option<String> {
    let security = transaction.security_id.as_deref()?;
    (!transaction.quantity.is_zero()).then(|| {
        format!(
            "{}|{}|{}|{}|{}",
            transaction.account_id,
            transaction.date,
            transaction.kind.as_str(),
            security,
            number(transaction.quantity)
        )
    })
}

/// Normalizes equivalent decimal spellings before comparison.
fn number(v: Decimal) -> String {
    v.normalize().to_string()
}
