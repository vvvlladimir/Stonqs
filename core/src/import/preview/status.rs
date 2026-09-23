//! What a row *is* once it has been read: ready, a duplicate, missing its instrument, ignored or
//! invalid. Only an `Error` makes a row invalid — a direction corrected from the sign is a
//! warning, and a warned row still imports.

use super::{ImportRow, RowStatus, TransactionDraft};
use crate::import::dedupe::{KnownRow, fingerprint, loose_fingerprint};
use crate::import::parse::{ImportProblem, ProblemCode, Severity};
use crate::model::TransactionKind;
use std::collections::HashSet;

/// Re-importing the same file must be a no-op, so a row is compared both against the database
/// and against the rows already read out of this file. A file that names its rows is compared
/// by that name first: the broker restating an operation is not a second operation.
pub(super) struct Dedupe<'a> {
    known: &'a HashSet<String>,
    loose: &'a HashSet<String>,
    external: &'a [KnownRow],
    seen: HashSet<String>,
}

impl<'a> Dedupe<'a> {
    pub fn against(known: &'a HashSet<String>, loose: &'a HashSet<String>, external: &'a [KnownRow]) -> Self {
        Dedupe {
            known,
            loose,
            external,
            seen: HashSet::new(),
        }
    }

    fn stored(&self, external_id: &str) -> Option<&KnownRow> {
        self.external.iter().find(|row| row.external_id == external_id)
    }
}

/// A transfer that names an instrument loses the instrument on commit, so say so while the user
/// can still map the wording to a delivery.
pub(super) fn check_transfer_keeps_its_instrument(
    draft: &Option<TransactionDraft>,
    number: usize,
    problems: &mut Vec<ImportProblem>,
) {
    if let Some(d) = draft
        && matches!(d.kind, TransactionKind::TransferIn | TransactionKind::TransferOut)
        && d.symbol.is_some()
        && !d.quantity.is_zero()
    {
        problems.push(ImportProblem::row(
            ProblemCode::TransferWithSecurity,
            number,
            "a transfer that names an instrument: map this kind to DELIVERY_INBOUND \
             or SECURITY_TRANSFER_IN, or the instrument is lost",
        ));
    }
}

pub(super) fn decide(
    number: usize,
    ignored: bool,
    draft: &mut Option<TransactionDraft>,
    dedupe: &mut Dedupe<'_>,
    problems: &mut Vec<ImportProblem>,
) -> RowStatus {
    let mut status = if ignored {
        RowStatus::Ignored
    } else if problems.iter().any(ImportProblem::is_error) || draft.is_none() {
        RowStatus::Invalid
    } else {
        RowStatus::Ready
    };

    if let Some(d) = draft
        && status == RowStatus::Ready
    {
        let d: &mut TransactionDraft = d;
        if d.kind.requires_security() && d.security_id.is_none() {
            status = RowStatus::UnknownSecurity;
        }
        if let Err(e) = d.to_transaction()
            && status == RowStatus::Ready
        {
            problems.push(ImportProblem::row(
                ProblemCode::InvalidTransaction,
                number,
                e.to_string(),
            ));
            status = RowStatus::Invalid;
        }
        let print = fingerprint(d);
        // The broker's own name for the row outranks its content: same id and same numbers is
        // the row we already have, same id and different numbers is that row restated.
        if let Some(external_id) = d.external_id.clone()
            && let Some(stored) = dedupe.stored(&external_id)
        {
            if stored.fingerprint == print {
                problems.push(ImportProblem::row(
                    ProblemCode::DuplicateInStore,
                    number,
                    "the same transaction is already in the database",
                ));
                return RowStatus::Duplicate;
            }
            d.replaces = Some(stored.transaction_id.clone());
            problems.push(
                ImportProblem::row(
                    ProblemCode::RestatedInStore,
                    number,
                    format!("the broker restated operation {external_id}: importing replaces the stored row"),
                )
                .warn(),
            );
            return RowStatus::Updated;
        }
        if dedupe.known.contains(&print) {
            problems.push(ImportProblem::row(
                ProblemCode::DuplicateInStore,
                number,
                "the same transaction is already in the database",
            ));
            status = RowStatus::Duplicate;
        } else if !dedupe.seen.insert(print) {
            problems.push(ImportProblem::row(
                ProblemCode::DuplicateInFile,
                number,
                "the same row already appeared in this file",
            ));
            status = RowStatus::Duplicate;
        } else if let Some(loose) = loose_fingerprint(d)
            && dedupe.loose.contains(&loose)
        {
            // Same day, same account, same instrument, same quantity — and a different value.
            // Either the stored row was corrected by hand, or the same size really traded twice
            // that day at two prices. Only the user knows which, so it is offered, not decided.
            problems.push(
                ImportProblem::row(
                    ProblemCode::SimilarInStore,
                    number,
                    "an operation of the same day, account, instrument and quantity is already in \
                     the database with different values — probably this row, corrected by hand \
                     after it was imported",
                )
                .warn(),
            );
            status = RowStatus::Similar;
        }
    }

    status
}

/// Derived from the rows themselves, so the counts and the table can never disagree.
pub(super) fn summarize(rows: &[ImportRow]) -> super::ImportSummary {
    let mut summary = super::ImportSummary::default();
    for row in rows {
        summary.total += 1;
        match row.status {
            RowStatus::Ready => summary.ready += 1,
            RowStatus::Duplicate => summary.duplicates += 1,
            RowStatus::Similar => summary.similar += 1,
            RowStatus::Updated => summary.updated += 1,
            RowStatus::UnknownSecurity => summary.unknown_securities += 1,
            RowStatus::Ignored => summary.ignored += 1,
            RowStatus::Invalid => summary.invalid += 1,
        }
        if row.problems.iter().any(|p| p.severity == Severity::Warning) {
            summary.warnings += 1;
        }
    }
    summary
}
