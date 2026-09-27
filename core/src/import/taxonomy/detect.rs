//! Which columns of a taxonomy CSV are levels, weights, targets and identifiers.

use super::TaxonomyCsvConfig;
use crate::import::parse::ParsedCsv;

/// A first level reading like this is the unclassified remainder, not a node.
const UNCLASSIFIED_ROOTS: &[&str] = &[
    "without classification",
    "not classified",
    "unassigned",
    "не классифицировано",
    "без классификации",
];

/// Detects taxonomy columns by header meaning.
pub fn detect_taxonomy_config(parsed: &ParsedCsv) -> TaxonomyCsvConfig {
    let headers = &parsed.headers;

    let mut numbered: Vec<(i64, String)> = headers
        .iter()
        .filter_map(|h| level_number(h).map(|n| (n, h.clone())))
        .collect();
    numbered.sort_by_key(|(n, _)| *n);
    let mut levels: Vec<String> = numbered.into_iter().map(|(_, h)| h).collect();
    if levels.is_empty() {
        levels = headers
            .iter()
            .filter(|h| {
                let l = h.to_lowercase();
                [
                    "category",
                    "категория",
                    "class",
                    "класс",
                    "group",
                    "группа",
                    "node",
                    "узел",
                ]
                .iter()
                .any(|k| l.contains(k))
            })
            .cloned()
            .collect();
    }

    let find = |keys: &[&str], deny: &[&str]| -> Option<String> {
        headers
            .iter()
            .find(|h| {
                let l = h.trim().to_lowercase();
                keys.iter().any(|k| l == *k || l.contains(k)) && !deny.iter().any(|d| l.contains(d))
            })
            .cloned()
    };

    let config = TaxonomyCsvConfig {
        weight: find(
            &["weight", "вес", "доля"],
            &["target", "цель", "value", "стоимость"],
        ),
        target: find(
            &["allocation", "target", "цель", "целевой"],
            &["value", "стоимость", "amount", "shares"],
        ),
        symbol: find(&["symbol", "ticker", "тикер", "символ"], &[]),
        isin: find(&["isin"], &[]),
        root_is_name: false,
        levels,
    };
    TaxonomyCsvConfig {
        root_is_name: root_is_constant(parsed, &config),
        ..config
    }
}

fn level_number(header: &str) -> Option<i64> {
    let lower = header.trim().to_lowercase();
    let rest = ["levels", "level", "уровень", "уровни"]
        .iter()
        .find_map(|p| lower.strip_prefix(p))?;
    rest.trim()
        .trim_start_matches(['_', '-', '.', ' '])
        .parse::<i64>()
        .ok()
}

fn root_is_constant(parsed: &ParsedCsv, config: &TaxonomyCsvConfig) -> bool {
    if config.levels.len() < 2 {
        return false;
    }
    let Some(first) = parsed
        .headers
        .iter()
        .position(|h| Some(h) == config.levels.first())
    else {
        return false;
    };
    let mut seen: Option<&str> = None;
    for row in &parsed.rows {
        let value = row.get(first).map(|v| v.trim()).unwrap_or("");
        if value.is_empty() || is_unclassified(value) {
            continue;
        }
        match seen {
            None => seen = Some(value),
            Some(prev) if prev == value => {}
            Some(_) => return false,
        }
    }
    seen.is_some()
}

pub(super) fn is_unclassified(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    UNCLASSIFIED_ROOTS.iter().any(|k| lower == *k)
}
