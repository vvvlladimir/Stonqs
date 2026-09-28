//! Reading a taxonomy CSV into nodes and assignments, touching no store.

use super::detect::is_unclassified;
use super::{PreviewAssignment, PreviewNode, TaxonomyCsvConfig, TaxonomyPreview, match_security};
use crate::import::parse::{ImportProblem, ParsedCsv, ProblemCode, Severity};
use crate::model::{Security, TaxonomyKind};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Builds a pure taxonomy preview from parsed rows and known securities.
pub fn build_taxonomy_preview(
    parsed: &ParsedCsv,
    config: &TaxonomyCsvConfig,
    securities: &[Security],
    name: Option<&str>,
) -> TaxonomyPreview {
    let mut problems: Vec<ImportProblem> = parsed
        .problems
        .iter()
        .filter(|p| p.code != ProblemCode::BadDate)
        .cloned()
        .collect();
    if config.levels.is_empty() {
        problems.push(ImportProblem::file(
            ProblemCode::MissingColumn,
            "no level column was found (\"Levels 1\", \"Category\")",
        ));
    }

    let index_of = |column: &Option<String>| -> Option<usize> {
        column
            .as_ref()
            .and_then(|c| parsed.headers.iter().position(|h| h == c))
    };
    let level_idx: Vec<usize> = config
        .levels
        .iter()
        .filter_map(|c| parsed.headers.iter().position(|h| h == c))
        .collect();
    let weight_idx = index_of(&config.weight);
    let target_idx = index_of(&config.target);
    let symbol_idx = index_of(&config.symbol);
    let isin_idx = index_of(&config.isin);

    let mut detected_name = String::new();
    let mut nodes: BTreeMap<Vec<String>, Option<Decimal>> = BTreeMap::new();
    let mut assignments: Vec<PreviewAssignment> = Vec::new();

    for (i, row) in parsed.rows.iter().enumerate() {
        let cell = |idx: Option<usize>| -> String {
            idx.and_then(|k| row.get(k))
                .map(|v| v.trim().to_string())
                .unwrap_or_default()
        };
        let Some(path) = category_path(row, &level_idx, config.root_is_name, &mut detected_name) else {
            continue;
        };
        let cells = Cells {
            symbol: cell(symbol_idx),
            isin: cell(isin_idx),
            weight: cell(weight_idx),
        };
        let target_raw = cell(target_idx);
        let is_assignment = !cells.symbol.is_empty()
            || !cells.isin.is_empty()
            || (!cells.weight.is_empty() && target_raw.is_empty());

        if is_assignment {
            if let Some(assignment) = read_assignment(i + 1, path, cells, config, securities, &mut problems) {
                nodes.entry(assignment.path.clone()).or_default();
                assignments.push(assignment);
            }
        } else {
            let target = parse_percent(&target_raw).filter(|v| *v > Decimal::ZERO);
            let entry = nodes.entry(path).or_default();
            if target.is_some() {
                *entry = target;
            }
        }
    }

    let nodes: Vec<PreviewNode> = nodes
        .into_iter()
        .map(|(path, target)| PreviewNode {
            path,
            target: target.map(|t| t / Decimal::ONE_HUNDRED),
        })
        .collect();
    let name = tree_name(name, detected_name);

    TaxonomyPreview {
        kind: guess_kind(&name),
        name,
        config: config.clone(),
        nodes,
        assignments,
        problems,
    }
}

/// The row's level cells without trailing blanks. With `root_is_name` the first level is the
/// tree's name, repeated on every row: the first one seen is kept and the level dropped.
fn category_path(
    row: &[String],
    level_idx: &[usize],
    root_is_name: bool,
    detected_name: &mut String,
) -> Option<Vec<String>> {
    let mut path: Vec<String> = level_idx
        .iter()
        .map(|k| row.get(*k).map(|v| v.trim().to_string()).unwrap_or_default())
        .collect();
    while path.last().is_some_and(|v| v.is_empty()) {
        path.pop();
    }
    if path.is_empty() {
        return None;
    }
    if root_is_name && detected_name.is_empty() {
        detected_name.clone_from(&path[0]);
    }
    if is_unclassified(&path[0]) {
        return None;
    }
    if root_is_name {
        path.remove(0);
    }
    (!path.is_empty()).then_some(path)
}

/// The cells that make a row an instrument's share rather than a category.
struct Cells {
    symbol: String,
    isin: String,
    weight: String,
}

/// An instrument row: its own name is the last level, and the levels above it are its category.
fn read_assignment(
    number: usize,
    mut path: Vec<String>,
    cells: Cells,
    config: &TaxonomyCsvConfig,
    securities: &[Security],
    problems: &mut Vec<ImportProblem>,
) -> Option<PreviewAssignment> {
    let Cells { symbol, isin, weight } = cells;
    let label = path.pop().unwrap_or_default();
    if path.is_empty() {
        return None;
    }
    let weight = match parse_percent(&weight) {
        Some(v) => v / Decimal::ONE_HUNDRED,
        None if weight.is_empty() => Decimal::ONE,
        None => {
            problems.push(problem(
                number,
                config.weight.clone(),
                ProblemCode::NotANumber,
                format!("the share {weight:?} could not be parsed; the row is skipped"),
                Severity::Error,
            ));
            return None;
        }
    };
    if weight <= Decimal::ZERO {
        return None;
    }
    let hit = match_security(&symbol, &isin, &label, securities);
    if hit.is_none() {
        let codes: Vec<&str> = [symbol.as_str(), isin.as_str()]
            .into_iter()
            .filter(|v| !v.is_empty())
            .collect();
        problems.push(problem(
            number,
            config.symbol.clone(),
            ProblemCode::UnknownSecurity,
            format!(
                "\"{label}\" ({}) is not in the database — this row's split will not be written",
                codes.join(" · ")
            ),
            Severity::Warning,
        ));
    }
    Some(PreviewAssignment {
        row: number,
        path,
        label,
        symbol,
        isin,
        weight: weight.min(Decimal::ONE),
        security_id: hit.as_ref().map(|(s, _)| s.id.clone()),
        matched_by: hit.map(|(_, how)| how.to_string()),
    })
}

/// The caller's name, else the one the file repeats, else a placeholder the user renames.
fn tree_name(given: Option<&str>, detected: String) -> String {
    let name = given
        .map(str::to_string)
        .filter(|n| !n.trim().is_empty())
        .unwrap_or(detected);
    if name.trim().is_empty() {
        "Imported classification".to_string()
    } else {
        name
    }
}

fn problem(
    row: usize,
    column: Option<String>,
    code: ProblemCode,
    message: String,
    severity: Severity,
) -> ImportProblem {
    ImportProblem {
        row: Some(row),
        column,
        severity,
        code,
        message,
        params: Default::default(),
    }
}

fn parse_percent(raw: &str) -> Option<Decimal> {
    let cleaned: String = raw
        .trim()
        .trim_end_matches('%')
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{a0}' && *c != '\'')
        .collect();
    if cleaned.is_empty() {
        return None;
    }
    let lower = cleaned.to_lowercase();
    if lower.contains("nan") || lower.contains("inf") {
        return None;
    }

    let normalized = if cleaned.contains('.') && cleaned.contains(',') {
        cleaned.replace(',', "")
    } else {
        cleaned.replace(',', ".")
    };
    normalized.parse::<Decimal>().ok()
}

fn guess_kind(name: &str) -> TaxonomyKind {
    let lower = name.to_lowercase();
    let has = |keys: &[&str]| keys.iter().any(|k| lower.contains(k));
    if has(&["region", "регион", "country", "стран", "geograph", "географ"]) {
        TaxonomyKind::Region
    } else if has(&["sector", "сектор", "industr", "отрасл", "gics"]) {
        TaxonomyKind::Sector
    } else if has(&["asset", "класс актив", "asset class", "allocation", "аллокац"]) {
        TaxonomyKind::AssetClass
    } else {
        TaxonomyKind::Custom
    }
}
