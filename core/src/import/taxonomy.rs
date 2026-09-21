//! Taxonomy CSV import and export.

use super::parse::{ImportProblem, ParsedCsv, ProblemCode, Severity};
use crate::error::{Error, Result};
use crate::model::{
    AllocationTarget, Security, SecurityClassification, TargetWeight, Taxonomy, TaxonomyKind, TaxonomyNode,
    new_id,
};
use crate::storage::Store;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const UNCLASSIFIED_ROOTS: &[&str] = &[
    "without classification",
    "not classified",
    "unassigned",
    "не классифицировано",
    "без классификации",
];

/// Column mapping detected for a taxonomy CSV.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonomyCsvConfig {
    pub levels: Vec<String>,

    pub weight: Option<String>,

    pub target: Option<String>,
    pub symbol: Option<String>,
    pub isin: Option<String>,

    pub root_is_name: bool,
}

/// A taxonomy node planned by the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewNode {
    pub path: Vec<String>,

    #[serde(with = "rust_decimal::serde::str_option")]
    pub target: Option<Decimal>,
}

/// A security assignment planned by the preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewAssignment {
    pub row: usize,
    pub path: Vec<String>,

    pub label: String,
    pub symbol: String,
    pub isin: String,

    #[serde(with = "rust_decimal::serde::str")]
    pub weight: Decimal,

    pub security_id: Option<String>,

    /// How the security was found: `isin`, `symbol`, `symbol_base` or `name` for a CSV,
    /// `attribute` or `already_classified` when a tree is grouped from an attribute. A stable
    /// code, not a label — the wording belongs to the UI.
    pub matched_by: Option<String>,
}

/// Complete taxonomy import preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaxonomyPreview {
    pub name: String,
    pub kind: TaxonomyKind,
    pub config: TaxonomyCsvConfig,
    pub nodes: Vec<PreviewNode>,
    pub assignments: Vec<PreviewAssignment>,
    pub problems: Vec<ImportProblem>,
}

impl TaxonomyPreview {
    pub fn matched(&self) -> usize {
        self.assignments
            .iter()
            .filter(|a| a.security_id.is_some())
            .count()
    }

    pub fn unmatched(&self) -> usize {
        self.assignments.len() - self.matched()
    }

    pub fn targets(&self) -> usize {
        self.nodes.iter().filter(|n| n.target.is_some()).count()
    }
}

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

pub fn commit_taxonomy(
    store: &Store,
    preview: &TaxonomyPreview,
    into: Option<&str>,
    portfolio_id: Option<&str>,
) -> Result<Taxonomy> {
    let taxonomy = match into {
        Some(id) => store.get_taxonomy(id)?,
        None => {
            let taxonomy = Taxonomy::new(preview.name.trim(), preview.kind);
            if taxonomy.name.is_empty() {
                return Err(Error::Invalid("the classification has an empty name".into()));
            }
            store.save_taxonomy(&taxonomy)?;
            taxonomy
        }
    };

    let existing = store.taxonomy_nodes(&taxonomy.id)?;
    let mut ids: BTreeMap<Vec<String>, String> = BTreeMap::new();
    for node in &existing {
        let mut path = vec![node.name.trim().to_lowercase()];
        let mut current = node;
        while let Some(parent) = current
            .parent_id
            .as_deref()
            .and_then(|p| existing.iter().find(|n| n.id == p))
        {
            path.insert(0, parent.name.trim().to_lowercase());
            current = parent;
        }
        ids.insert(path, node.id.clone());
    }
    let key = |path: &[String]| -> Vec<String> { path.iter().map(|p| p.trim().to_lowercase()).collect() };
    let mut rank = existing.len();

    for node in preview.nodes.iter() {
        for depth in 1..=node.path.len() {
            let path = key(&node.path[..depth]);
            if ids.contains_key(&path) {
                continue;
            }
            let parent = if depth > 1 {
                ids.get(&key(&node.path[..depth - 1])).cloned()
            } else {
                None
            };
            let saved = TaxonomyNode {
                id: new_id(),
                taxonomy_id: taxonomy.id.clone(),
                parent_id: parent,
                name: node.path[depth - 1].clone(),
                rank: rank as i64,

                color: None,
            };
            store.save_taxonomy_node(&saved)?;
            ids.insert(path, saved.id);
            rank += 1;
        }
    }

    for a in &preview.assignments {
        let (Some(security_id), Some(node_id)) = (a.security_id.as_deref(), ids.get(&key(&a.path))) else {
            continue;
        };
        store.save_classification(&SecurityClassification::new(security_id, node_id, a.weight))?;
    }

    if let Some(portfolio_id) = portfolio_id {
        let mut weights: Vec<TargetWeight> = preview
            .nodes
            .iter()
            .filter_map(|n| {
                let weight = n.target?;
                Some(TargetWeight {
                    node_id: ids.get(&key(&n.path))?.clone(),
                    weight,
                })
            })
            .collect();

        let existing_target = store
            .targets_for_portfolio(portfolio_id)?
            .into_iter()
            .find(|t| t.taxonomy_id == taxonomy.id);
        if let Some(previous) = &existing_target {
            for old in &previous.weights {
                if !weights.iter().any(|w| w.node_id == old.node_id) {
                    weights.push(old.clone());
                }
            }
        }
        if !weights.is_empty() {
            let target = AllocationTarget {
                id: existing_target.map(|t| t.id).unwrap_or_else(new_id),
                portfolio_id: portfolio_id.to_string(),
                taxonomy_id: taxonomy.id.clone(),
                name: format!("Target · {}", taxonomy.name),
                weights,
            };

            let saved_nodes = store.taxonomy_nodes(&taxonomy.id)?;
            if target.validate_tree(&saved_nodes).is_ok() {
                store.save_target(&target)?;
            }
        }
    }

    Ok(taxonomy)
}

pub fn taxonomy_to_csv(
    taxonomy: &Taxonomy,
    nodes: &[TaxonomyNode],
    classifications: &[SecurityClassification],
    securities: &[Security],
    target: Option<&AllocationTarget>,
) -> String {
    let depth_of = |node: &TaxonomyNode| -> usize {
        let mut depth = 1;
        let mut current = node;
        while let Some(parent) = current
            .parent_id
            .as_deref()
            .and_then(|p| nodes.iter().find(|n| n.id == p))
        {
            depth += 1;
            current = parent;
        }
        depth
    };

    let levels = nodes.iter().map(depth_of).max().unwrap_or(0) + 2;

    let mut out = String::new();
    for i in 1..=levels {
        out.push_str(&format!("Levels {i},"));
    }
    out.push_str("Weight,Allocation,Symbol,ISIN,Name\n");

    fn path_of(node: &TaxonomyNode, nodes: &[TaxonomyNode]) -> Vec<String> {
        let mut path = vec![node.name.clone()];
        let mut current = node;
        while let Some(parent) = current
            .parent_id
            .as_deref()
            .and_then(|p| nodes.iter().find(|n| n.id == p))
        {
            path.insert(0, parent.name.clone());
            current = parent;
        }
        path
    }

    let mut write =
        |path: &[String], weight: &str, allocation: &str, symbol: &str, isin: &str, name: &str| {
            let mut cells: Vec<String> = vec![taxonomy.name.clone()];
            cells.extend(path.iter().cloned());
            while cells.len() < levels {
                cells.push(String::new());
            }
            let mut line: Vec<String> = cells.iter().map(|c| quote(c)).collect();
            for extra in [weight, allocation, symbol, isin, name] {
                line.push(quote(extra));
            }
            out.push_str(&line.join(","));
            out.push('\n');
        };

    fn walk(
        parent: Option<&str>,
        nodes: &[TaxonomyNode],
        classifications: &[SecurityClassification],
        securities: &[Security],
        target: Option<&AllocationTarget>,
        write: &mut impl FnMut(&[String], &str, &str, &str, &str, &str),
    ) {
        let mut children: Vec<&TaxonomyNode> = nodes
            .iter()
            .filter(|n| n.parent_id.as_deref() == parent)
            .collect();
        children.sort_by_key(|n| (n.rank, n.name.clone()));
        for node in children {
            let path = path_of(node, nodes);
            let allocation = target
                .and_then(|t| t.weights.iter().find(|w| w.node_id == node.id))
                .map(|w| percent(w.weight))
                .unwrap_or_default();
            write(&path, "", &allocation, "", "", "");

            let mut mine: Vec<&SecurityClassification> =
                classifications.iter().filter(|c| c.node_id == node.id).collect();
            mine.sort_by_key(|c| std::cmp::Reverse(c.weight));
            for c in mine {
                let security = securities.iter().find(|s| s.id == c.security_id);
                let name = security.map(|s| s.name.clone()).unwrap_or_default();
                let symbol = security.map(|s| s.symbol.clone()).unwrap_or_default();
                let isin = security.and_then(|s| s.isin.clone()).unwrap_or_default();
                let mut deeper = path.clone();

                deeper.push(if name.is_empty() {
                    symbol.clone()
                } else {
                    name.clone()
                });
                write(&deeper, &percent(c.weight), "", &symbol, &isin, &name);
            }

            walk(Some(&node.id), nodes, classifications, securities, target, write);
        }
    }

    walk(None, nodes, classifications, securities, target, &mut write);
    out
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

fn is_unclassified(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    UNCLASSIFIED_ROOTS.iter().any(|k| lower == *k)
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

fn percent(weight: Decimal) -> String {
    (weight * Decimal::ONE_HUNDRED)
        .round_dp(4)
        .normalize()
        .to_string()
}

fn quote(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn match_security<'a>(
    symbol: &str,
    isin: &str,
    label: &str,
    securities: &'a [Security],
) -> Option<(&'a Security, &'static str)> {
    let eq = |a: &str, b: &str| a.eq_ignore_ascii_case(b) && !a.is_empty();
    let base = |s: &str| s.split('.').next().unwrap_or(s).to_string();

    if !isin.is_empty()
        && let Some(s) = securities
            .iter()
            .find(|s| s.isin.as_deref().is_some_and(|v| eq(v, isin)))
    {
        return Some((s, "isin"));
    }
    if !symbol.is_empty() {
        if let Some(s) = securities.iter().find(|s| eq(&s.symbol, symbol)) {
            return Some((s, "symbol"));
        }
        if let Some(s) = securities.iter().find(|s| eq(&base(&s.symbol), &base(symbol))) {
            return Some((s, "symbol_base"));
        }
    }
    if !label.is_empty()
        && let Some(s) = securities.iter().find(|s| eq(&s.name, label))
    {
        return Some((s, "name"));
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::{ParseConfig, parse_csv};
    use crate::model::SecurityKind;
    use rust_decimal_macros::dec;

    const PP: &str = "Levels 1,Levels 2,Levels 3,Levels 4,Weight,Allocation,TARGET Value,Symbol,ISIN\n\
Asset Allocation,,,,,100.00,\"2,585.16\",,\n\
Asset Allocation,Global Equities,,,,42.00,\"1,085.77\",,\n\
Asset Allocation,Global Equities,Core,,,88.00,955.48,,\n\
Asset Allocation,Global Equities,Core,Core MSCI World USD (Acc),72.60,,,EUNL.DE,IE00B4L5Y983\n\
Asset Allocation,Bonds,,,,5.00,129.26,,\n\
Asset Allocation,Bonds,Global Aggregate Bond EUR (Acc),,100.00,,,EUNA.DE,IE00BDBRDM35\n\
Without Classification,Apple,,,100.00,,,APC.DE,US0378331005\n";

    fn securities() -> Vec<Security> {
        let mut world = Security::new("IWDA.L", "iShares Core MSCI World", "USD", SecurityKind::Etf);
        world.isin = Some("IE00B4L5Y983".into());
        let mut bond = Security::new("EUNA.DE", "Global Aggregate Bond", "EUR", SecurityKind::Etf);
        bond.isin = None;
        vec![world, bond]
    }

    #[test]
    fn portfolio_performance_export_is_read_without_a_single_setting() {
        let parsed = parse_csv(PP.as_bytes(), &ParseConfig::default()).unwrap();
        let config = detect_taxonomy_config(&parsed);

        assert_eq!(config.levels, ["Levels 1", "Levels 2", "Levels 3", "Levels 4"]);
        assert_eq!(config.weight.as_deref(), Some("Weight"));
        assert_eq!(
            config.target.as_deref(),
            Some("Allocation"),
            "\"TARGET Value\" is money, not a target"
        );
        assert!(config.root_is_name, "the first level repeats on every row");

        let securities = securities();
        let preview = build_taxonomy_preview(&parsed, &config, &securities, None);

        assert_eq!(preview.name, "Asset Allocation");

        assert!(!preview.nodes.iter().any(|n| n.path[0].contains("Without")));

        let target_of = |path: &[&str]| {
            preview
                .nodes
                .iter()
                .find(|n| n.path == path.iter().map(|s| s.to_string()).collect::<Vec<_>>())
                .and_then(|n| n.target)
        };
        assert_eq!(target_of(&["Global Equities", "Core"]), Some(dec!(0.88)));
        assert_eq!(target_of(&["Global Equities"]), Some(dec!(0.42)));
        assert_eq!(target_of(&["Bonds"]), Some(dec!(0.05)));

        let world = preview
            .assignments
            .iter()
            .find(|a| a.symbol == "EUNL.DE")
            .unwrap();
        assert_eq!(world.path, ["Global Equities", "Core"]);
        assert_eq!(world.weight, dec!(0.726));
        assert_eq!(world.matched_by.as_deref(), Some("isin"));

        let bond = preview
            .assignments
            .iter()
            .find(|a| a.symbol == "EUNA.DE")
            .unwrap();
        assert_eq!(bond.path, ["Bonds"]);
        assert_eq!(bond.matched_by.as_deref(), Some("symbol"));
        assert_eq!(preview.matched(), 2);
    }

    #[test]
    fn targets_are_stored_relative_to_the_parent() {
        let store = Store::open_in_memory().unwrap();
        let portfolio = crate::model::Portfolio::new("Mine", "EUR");
        store.save_portfolio(&portfolio).unwrap();
        let securities = securities();
        for s in &securities {
            store.save_security(s).unwrap();
        }
        let parsed = parse_csv(PP.as_bytes(), &ParseConfig::default()).unwrap();
        let config = detect_taxonomy_config(&parsed);
        let preview = build_taxonomy_preview(&parsed, &config, &securities, None);

        let taxonomy = commit_taxonomy(&store, &preview, None, Some(&portfolio.id)).unwrap();
        let nodes = store.taxonomy_nodes(&taxonomy.id).unwrap();
        let id_of = |name: &str| nodes.iter().find(|n| n.name == name).unwrap().id.clone();
        let target = store
            .targets_for_portfolio(&portfolio.id)
            .unwrap()
            .into_iter()
            .find(|t| t.taxonomy_id == taxonomy.id)
            .unwrap();

        assert_eq!(target.weight_of(&id_of("Global Equities")), Some(dec!(0.42)));
        assert_eq!(target.weight_of(&id_of("Core")), Some(dec!(0.88)));
        let absolute = target.absolute_weights(&nodes);
        assert_eq!(absolute[&id_of("Core")], dec!(0.3696));
        assert_eq!(absolute[&id_of("Bonds")], dec!(0.05));
    }

    #[test]
    fn importing_the_same_file_twice_extends_the_tree_instead_of_doubling_it() {
        let store = Store::open_in_memory().unwrap();
        let securities = securities();
        for s in &securities {
            store.save_security(s).unwrap();
        }
        let parsed = parse_csv(PP.as_bytes(), &ParseConfig::default()).unwrap();
        let config = detect_taxonomy_config(&parsed);
        let preview = build_taxonomy_preview(&parsed, &config, &securities, None);

        let taxonomy = commit_taxonomy(&store, &preview, None, None).unwrap();
        let after_first = store.taxonomy_nodes(&taxonomy.id).unwrap().len();
        assert_eq!(after_first, 3, "Global Equities, Core inside it, and Bonds");

        let before = store.list_taxonomies().unwrap().len();
        commit_taxonomy(&store, &preview, Some(&taxonomy.id), None).unwrap();
        assert_eq!(
            store.list_taxonomies().unwrap().len(),
            before,
            "no second tree was created"
        );
        assert_eq!(store.taxonomy_nodes(&taxonomy.id).unwrap().len(), after_first);

        assert_eq!(store.classifications_for_taxonomy(&taxonomy.id).unwrap().len(), 2);
    }

    #[test]
    fn export_round_trips_through_the_importer() {
        let taxonomy = Taxonomy::new("Regions", TaxonomyKind::Region);
        let us = TaxonomyNode::root(&taxonomy.id, "United States");
        let de = TaxonomyNode::child(&us, "Germany");
        let mut security = Security::new("IWDA.L", "iShares Core MSCI World", "USD", SecurityKind::Etf);
        security.isin = Some("IE00B4L5Y983".into());
        let classifications = vec![SecurityClassification::new(&security.id, &de.id, dec!(0.35))];
        let target =
            AllocationTarget::new("portfolio", &taxonomy.id, "Target").with_weight(&de.id, dec!(0.2));

        let csv = taxonomy_to_csv(
            &taxonomy,
            &[us.clone(), de.clone()],
            &classifications,
            std::slice::from_ref(&security),
            Some(&target),
        );

        let parsed = parse_csv(csv.as_bytes(), &ParseConfig::default()).unwrap();
        let config = detect_taxonomy_config(&parsed);
        let preview = build_taxonomy_preview(&parsed, &config, std::slice::from_ref(&security), None);

        assert_eq!(preview.name, "Regions");
        assert_eq!(preview.kind, TaxonomyKind::Region);
        let deepest = preview
            .nodes
            .iter()
            .find(|n| n.path == ["United States", "Germany"])
            .unwrap();
        assert_eq!(deepest.target, Some(dec!(0.2)));
        assert_eq!(preview.assignments.len(), 1);
        assert_eq!(preview.assignments[0].weight, dec!(0.35));
        assert_eq!(preview.assignments[0].path, ["United States", "Germany"]);
    }
}
