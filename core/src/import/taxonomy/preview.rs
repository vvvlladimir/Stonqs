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
        let mut path: Vec<String> = level_idx
            .iter()
            .map(|k| row.get(*k).map(|v| v.trim().to_string()).unwrap_or_default())
            .collect();
        while path.last().is_some_and(|v| v.is_empty()) {
            path.pop();
        }
        if path.is_empty() {
            continue;
        }
        if config.root_is_name {
            if detected_name.is_empty() {
                detected_name = path[0].clone();
            }

            // A repeated first level is the tree name, not a category.
            if is_unclassified(&path[0]) {
                continue;
            }
            path.remove(0);
            if path.is_empty() {
                continue;
            }
        } else if is_unclassified(&path[0]) {
            continue;
        }

        let symbol = cell(symbol_idx);
        let isin = cell(isin_idx);
        let weight_raw = cell(weight_idx);
        let target_raw = cell(target_idx);
        let is_assignment =
            !symbol.is_empty() || !isin.is_empty() || (!weight_raw.is_empty() && target_raw.is_empty());

        if is_assignment {
            let label = path.pop().unwrap_or_default();
            if path.is_empty() {
                continue;
            }
            let weight = match parse_percent(&weight_raw) {
                Some(v) => v / Decimal::ONE_HUNDRED,

                None if weight_raw.is_empty() => Decimal::ONE,
                None => {
                    problems.push(problem(
                        i + 1,
                        config.weight.clone(),
                        ProblemCode::NotANumber,
                        format!("the share {weight_raw:?} could not be parsed; the row is skipped"),
                        Severity::Error,
                    ));
                    continue;
                }
            };
            if weight <= Decimal::ZERO {
                continue;
            }
            let hit = match_security(&symbol, &isin, &label, securities);
            if hit.is_none() {
                problems.push(problem(
                    i + 1,
                    config.symbol.clone(),
                    ProblemCode::UnknownSecurity,
                    format!(
                        "\"{label}\" ({}) is not in the database — this row's split will not be written",
                        [symbol.as_str(), isin.as_str()]
                            .iter()
                            .filter(|v| !v.is_empty())
                            .cloned()
                            .collect::<Vec<_>>()
                            .join(" · ")
                    ),
                    Severity::Warning,
                ));
            }

            nodes.entry(path.clone()).or_default();
            assignments.push(PreviewAssignment {
                row: i + 1,
                path,
                label,
                symbol,
                isin,
                weight: weight.min(Decimal::ONE),
                security_id: hit.as_ref().map(|(s, _)| s.id.clone()),
                matched_by: hit.map(|(_, how)| how.to_string()),
            });
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

    let name = name
        .map(str::to_string)
        .filter(|n| !n.trim().is_empty())
        .unwrap_or(detected_name);
    let name = if name.trim().is_empty() {
        "Imported classification".to_string()
    } else {
        name
    };

    TaxonomyPreview {
        kind: guess_kind(&name),
        name,
        config: config.clone(),
        nodes,
        assignments,
        problems,
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
