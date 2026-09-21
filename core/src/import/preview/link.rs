//! Pairing the two legs of one internal move.

use super::cells::cell_of;
use super::{ImportRow, RowStatus};
use crate::import::mapping::{ImportField, ImportMapping, normalize_alias};
use crate::model::TransactionKind;
use chrono::NaiveDate;
use std::collections::BTreeMap;

/// Pairs the two legs of one internal move: a conversion, a stake or a wallet-to-wallet
/// transfer is one wording printed on two rows, and only the sign tells the legs apart.
/// `calc` reads an unlinked transfer as money crossing the portfolio boundary, so a pair
/// that is really internal has to say so here. A leg left without a partner stays unlinked.
pub(super) fn internal_transfers(rows: &mut [ImportRow], mapping: &ImportMapping) {
    struct Leg {
        row: usize,
        date: NaiveDate,
        wording: String,
        kind: TransactionKind,
    }

    let legs: Vec<Leg> = rows
        .iter()
        .enumerate()
        .filter_map(|(row, r)| {
            let draft = r.draft.as_ref()?;
            if draft.link_id.is_some()
                || r.status != RowStatus::Ready
                || !matches!(
                    draft.kind,
                    TransactionKind::TransferIn | TransactionKind::TransferOut
                )
            {
                return None;
            }
            Some(Leg {
                row,
                date: draft.date,
                wording: normalize_alias(cell_of(&r.raw, mapping, ImportField::Kind)?),
                kind: draft.kind,
            })
        })
        .collect();

    let mut waiting: BTreeMap<(NaiveDate, String), Vec<usize>> = BTreeMap::new();
    let mut links: Vec<(usize, String)> = Vec::new();

    for (n, leg) in legs.iter().enumerate() {
        let queue = waiting.entry((leg.date, leg.wording.clone())).or_default();
        match queue.iter().position(|&i| legs[i].kind != leg.kind) {
            Some(at) => {
                let other = queue.remove(at);
                // Deterministic: `build_preview` must give the same answer for the same file.
                let link = format!("csv:{}:{}:{}", leg.date, leg.wording, legs[other].row);
                links.push((legs[other].row, link.clone()));
                links.push((leg.row, link));
            }
            None => queue.push(n),
        }
    }

    for (row, link) in links {
        if let Some(draft) = rows[row].draft.as_mut() {
            draft.link_id = Some(link);
        }
    }
}
