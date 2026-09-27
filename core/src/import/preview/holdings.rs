//! A sale the ledger cannot cover once the file is in.
//!
//! The file alone does not answer it — the purchase may have been imported last year — so the
//! stored operations are replayed together with the rows about to be written, in the order the
//! holdings builder applies them (`calc::quantity_gaps`). A warning, like every heuristic here:
//! the missing purchase may simply be in the next file (ADR-0089).

use super::cells::cell_of;
use super::{ImportContext, ImportRow, RowStatus};
use crate::calc::quantity_gaps;
use crate::import::mapping::{ImportField, ImportMapping, normalize_alias};
use crate::import::parse::{ImportProblem, ProblemCode};
use crate::model::Transaction;
use std::collections::{BTreeSet, HashMap, HashSet};

const ROW_ID: &str = "import-row:";

pub(super) fn check(rows: &mut [ImportRow], mapping: &ImportMapping, context: &ImportContext<'_>) {
    let replaced: HashSet<&str> = rows
        .iter()
        .filter(|r| r.status == RowStatus::Updated)
        .filter_map(|r| r.draft.as_ref()?.replaces.as_deref())
        .collect();
    let mut ledger: Vec<Transaction> = context
        .ledger
        .iter()
        .filter(|t| !replaced.contains(t.id.as_str()))
        .cloned()
        .collect();

    for (index, row) in rows.iter().enumerate() {
        if !matches!(
            row.status,
            RowStatus::Ready | RowStatus::Updated | RowStatus::UnknownSecurity
        ) {
            continue;
        }
        let Some(draft) = &row.draft else { continue };
        if draft.kind.quantity_sign() == 0 {
            continue;
        }
        // An instrument the database does not have yet is keyed by what the file calls it.
        let Some(key) = draft.security_id.clone().or_else(|| {
            draft
                .isin
                .as_deref()
                .or(draft.symbol.as_deref())
                .map(|s| format!("new:{}", normalize_alias(s)))
        }) else {
            continue;
        };
        let mut t = Transaction::cash(
            &draft.account_id,
            draft.kind,
            draft.date,
            draft.amount,
            &draft.currency,
        );
        t.id = format!("{ROW_ID}{index}");
        t.security_id = Some(key);
        t.quantity = draft.quantity;
        ledger.push(t);
    }

    let Ok(gaps) = quantity_gaps(&ledger, context.corporate_actions) else {
        return;
    };
    let unread = unread_rows(rows, mapping);
    for gap in gaps {
        let Some(index) = gap
            .transaction_id
            .strip_prefix(ROW_ID)
            .and_then(|i| i.parse::<usize>().ok())
        else {
            continue;
        };
        let row = &rows[index];
        let draft = row.draft.as_ref().expect("only rows with a draft are replayed");
        let symbol = draft
            .symbol
            .as_deref()
            .or(draft.isin.as_deref())
            .unwrap_or_default()
            .to_string();
        let mut problem = ImportProblem::row(
            ProblemCode::SaleExceedsHoldings,
            row.number,
            format!(
                "{} of {symbol} leave on {}, but only {} is held then — the purchase or incoming \
                 transfer is missing; the figures for {symbol} will be estimated until it is added",
                gap.quantity.normalize(),
                gap.date,
                gap.held.normalize()
            ),
        )
        .with("symbol", &symbol)
        .with("date", gap.date)
        .with("quantity", gap.quantity.normalize())
        .with("held", gap.held.normalize())
        .with("missing", gap.missing.normalize())
        .warn();
        if let Some((count, kinds)) = identities(row, mapping).iter().find_map(|id| unread.get(id)) {
            problem = problem.with("unread", count).with(
                "unread_kinds",
                kinds.iter().cloned().collect::<Vec<_>>().join(", "),
            );
        }
        rows[index].problems.push(problem);
    }
}

/// The instrument cells of a row, normalised: the ticker and the ISIN as the file prints them,
/// before any alias, because an unread row never had one applied.
fn identities(row: &ImportRow, mapping: &ImportMapping) -> Vec<String> {
    [ImportField::Isin, ImportField::Symbol]
        .into_iter()
        .filter_map(|f| cell_of(&row.raw, mapping, f))
        .map(normalize_alias)
        .collect()
}

/// Rows of the file that will not be written — an unmapped wording, a value marked "do not
/// import", a row that failed to read — by instrument: the likeliest place the missing
/// purchase is hiding.
fn unread_rows(rows: &[ImportRow], mapping: &ImportMapping) -> HashMap<String, (usize, BTreeSet<String>)> {
    let mut out: HashMap<String, (usize, BTreeSet<String>)> = HashMap::new();
    for row in rows
        .iter()
        .filter(|r| matches!(r.status, RowStatus::Invalid | RowStatus::Ignored))
    {
        let kind = cell_of(&row.raw, mapping, ImportField::Kind)
            .unwrap_or_default()
            .to_string();
        for id in identities(row, mapping) {
            let entry = out.entry(id).or_default();
            entry.0 += 1;
            if !kind.is_empty() {
                entry.1.insert(kind.clone());
            }
        }
    }
    out
}
