//! Taxonomy CSV import and export, read by meaning rather than by template.

mod commit;
mod detect;
mod export;
mod preview;

pub use commit::commit_taxonomy;
pub use detect::detect_taxonomy_config;
pub use export::taxonomy_to_csv;
pub use preview::build_taxonomy_preview;

use super::parse::ImportProblem;
use crate::model::{Security, TaxonomyKind, TaxonomyNode};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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

    /// A code, not a label: `isin`, `symbol`, `symbol_base`, `name`, `attribute` or
    /// `already_classified`.
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

/// Names from the root down to `node`.
fn node_path<'a>(node: &'a TaxonomyNode, nodes: &'a [TaxonomyNode]) -> Vec<&'a str> {
    let mut path = vec![node.name.as_str()];
    let mut current = node;
    while let Some(parent) = current
        .parent_id
        .as_deref()
        .and_then(|p| nodes.iter().find(|n| n.id == p))
    {
        path.insert(0, parent.name.as_str());
        current = parent;
    }
    path
}

pub(super) fn quote(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// ISIN first (it names the instrument, a ticker only a listing), then ticker, then name.
/// Shared with the attribute import.
pub(super) fn match_security<'a>(
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

#[cfg(test)]
mod tests;
