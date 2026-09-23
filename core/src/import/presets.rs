//! Broker layouts shipped with the application.
//!
//! Detection reads an unknown file; a preset is the answer for a file we have already seen.
//! It is data, not code: adding a broker means one entry in `presets/brokers.json`, which is
//! also the shape a downloaded preset would arrive in.
//!
//! Every entry is written by hand from a broker's published column documentation or from a
//! redacted sample — nothing here is copied from another project, and no broker endorses it.
//! Not all of them have been tried against a real export yet, which is why a preset is only ever
//! a starting point: the import wizard lets the user override every part of it (`import.md`).

use super::mapping::{ImportMapping, default_kind_aliases, normalize_header};
use super::parse::ParseConfig;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// What makes a file recognisable as this broker's export. Everything in it is optional and
/// everything declared must hold, so a rule is a claim the preset makes about the file rather
/// than a guess the app makes about the preset.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresetMatch {
    /// Column names the file must all have. Derived from the layout itself when the preset
    /// declares none: the columns a layout maps *are* that broker's header row.
    #[serde(default)]
    pub headers: Vec<String>,

    /// Text that must occur near the start of the file — a broker's name in its preamble.
    #[serde(default)]
    pub marker: Option<String>,

    /// Part of the file's own name, compared case-insensitively.
    #[serde(default)]
    pub file_name: Option<String>,
}

impl PresetMatch {
    /// The rule a layout implies: its own mapped columns.
    pub fn of_columns(columns: impl IntoIterator<Item = String>) -> Self {
        PresetMatch {
            headers: columns.into_iter().collect(),
            ..PresetMatch::default()
        }
    }

    /// How well the file answers this rule: the number of signals matched, or `None` when
    /// anything the rule declares is absent. A rule declaring nothing recognises nothing.
    pub fn score(&self, headers: &[String], file_name: Option<&str>, head: &str) -> Option<u32> {
        let normalized: Vec<String> = headers.iter().map(|h| normalize_header(h)).collect();
        for wanted in &self.headers {
            let wanted = normalize_header(wanted);
            if !normalized.contains(&wanted) {
                return None;
            }
        }
        let mut score = self.headers.len() as u32;
        if let Some(marker) = &self.marker {
            if !head.to_lowercase().contains(&marker.to_lowercase()) {
                return None;
            }
            score += 2;
        }
        if let Some(part) = &self.file_name {
            if !file_name?.to_lowercase().contains(&part.to_lowercase()) {
                return None;
            }
            score += 2;
        }
        (score > 0).then_some(score)
    }
}

/// Picks the layout a file belongs to. A tie is no answer: two layouts fitting a file equally
/// well means neither has been recognised, and guessing one costs more than asking.
pub fn best_match<'a>(
    candidates: impl IntoIterator<Item = (&'a str, PresetMatch)>,
    headers: &[String],
    file_name: Option<&str>,
    head: &str,
) -> Option<&'a str> {
    let mut best: Option<(&str, u32)> = None;
    let mut tied = false;
    for (name, rule) in candidates {
        let Some(score) = rule.score(headers, file_name, head) else {
            continue;
        };
        match best {
            Some((_, top)) if score < top => {}
            Some((_, top)) if score == top => tied = true,
            _ => {
                best = Some((name, score));
                tied = false;
            }
        }
    }
    if tied { None } else { best.map(|(name, _)| name) }
}

/// One broker's layout: how to read the file and what its columns and wordings mean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokerPreset {
    pub name: String,

    #[serde(default)]
    pub config: ParseConfig,

    pub mapping: ImportMapping,

    /// How a file is recognised as this broker's. Absent means "by the columns it maps".
    #[serde(default, rename = "match", skip_serializing_if = "Option::is_none")]
    pub match_rule: Option<PresetMatch>,
}

impl BrokerPreset {
    /// The mapping as it is applied: the wordings every broker shares are merged back in
    /// under the preset's own. Leaving them out of the file is what keeps it readable.
    pub fn mapping(&self) -> ImportMapping {
        let mut merged = self.mapping.clone();
        for (value, kind) in default_kind_aliases() {
            merged.kind_aliases.entry(value).or_insert(kind);
        }
        merged
    }

    /// The rule this preset is recognised by: its own if it declares one, otherwise the columns
    /// it maps — a layout that names "Wertpapierbezeichnung" already describes its broker.
    pub fn match_rule(&self) -> PresetMatch {
        self.match_rule
            .clone()
            .unwrap_or_else(|| PresetMatch::of_columns(self.mapping.columns.values().cloned()))
    }

    /// Turns a layout made on a real file into a preset: an account belongs to the user who
    /// made it, and a security draft to the file it came from, so neither travels.
    pub fn of(name: &str, config: &ParseConfig, mapping: &ImportMapping) -> Self {
        let defaults = default_kind_aliases();
        let mut mapping = mapping.clone();
        mapping.account_id = None;
        mapping.new_securities.clear();
        mapping
            .kind_aliases
            .retain(|value, kind| defaults.get(value) != Some(kind));
        BrokerPreset {
            name: name.to_string(),
            config: config.clone(),
            mapping,
            match_rule: None,
        }
    }
}

const BUILTIN_JSON: &str = include_str!("../../presets/brokers.json");

/// Layouts compiled into the binary. Parsed once; a malformed file is a build-time mistake,
/// and `presets_parse` in the tests below is what catches it.
pub fn builtin_presets() -> &'static [BrokerPreset] {
    static PRESETS: OnceLock<Vec<BrokerPreset>> = OnceLock::new();
    PRESETS.get_or_init(|| parse_presets(BUILTIN_JSON).expect("presets/brokers.json is malformed"))
}

pub fn parse_presets(json: &str) -> Result<Vec<BrokerPreset>> {
    serde_json::from_str(json).map_err(|e| Error::Invalid(format!("the preset list did not parse: {e}")))
}

/// Serializes presets in the shape of the bundled file, so a layout made by hand in the app
/// can be pasted back into it.
pub fn presets_to_json(presets: &[BrokerPreset]) -> Result<String> {
    serde_json::to_string_pretty(presets).map_err(|e| Error::Invalid(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::ImportField;

    #[test]
    fn every_bundled_preset_is_usable_as_it_stands() {
        let presets = builtin_presets();
        assert!(presets.len() >= 20, "presets: {}", presets.len());

        let mut names: Vec<&str> = presets.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        let unique = names.len();
        names.dedup();
        assert_eq!(names.len(), unique, "preset names must be unique");

        for preset in presets {
            let mapping = preset.mapping();
            assert!(
                mapping.column(ImportField::Date).is_some(),
                "{}: no date column",
                preset.name
            );
            assert!(
                mapping.column(ImportField::Kind).is_some() || preset.name == "DEGIRO",
                "{}: no kind column",
                preset.name
            );
            // A preset carries no account and no security of somebody else's portfolio.
            assert_eq!(mapping.account_id, None, "{}", preset.name);
            assert!(mapping.new_securities.is_empty(), "{}", preset.name);
            // The shared wordings come back when the preset is applied.
            assert_eq!(mapping.kind_of("BUY"), Some(crate::model::TransactionKind::Buy));
        }
    }

    #[test]
    fn a_shipped_layout_recognises_the_file_it_was_written_for() {
        let presets = builtin_presets();
        let republic = presets
            .iter()
            .find(|p| p.name.contains("Trade Republic"))
            .expect("Trade Republic ships as a preset");

        // A preset declares no rule of its own, so the columns it maps are the rule.
        let headers: Vec<String> = republic.mapping.columns.values().cloned().collect();
        let candidates = presets.iter().map(|p| (p.name.as_str(), p.match_rule()));
        assert_eq!(
            best_match(candidates, &headers, None, ""),
            Some(republic.name.as_str())
        );
    }

    #[test]
    fn a_file_no_layout_describes_is_not_guessed_at() {
        let headers = vec!["when".to_string(), "what".to_string(), "how much".to_string()];
        let candidates = builtin_presets()
            .iter()
            .map(|p| (p.name.as_str(), p.match_rule()));
        assert_eq!(best_match(candidates, &headers, None, ""), None);
    }

    #[test]
    fn two_layouts_fitting_equally_well_recognise_nothing() {
        let rule = PresetMatch::of_columns(["Date".to_string(), "Amount".to_string()]);
        let headers = vec!["Date".to_string(), "Amount".to_string()];
        let candidates = [("A", rule.clone()), ("B", rule)];
        assert_eq!(best_match(candidates, &headers, None, ""), None);
    }

    #[test]
    fn a_declared_rule_must_hold_in_full() {
        let rule = PresetMatch {
            headers: vec!["Date".to_string()],
            marker: Some("Interactive Brokers".to_string()),
            file_name: None,
        };
        let headers = vec!["Date".to_string()];
        assert_eq!(
            rule.score(&headers, None, "Interactive Brokers statement"),
            Some(3)
        );
        assert_eq!(rule.score(&headers, None, "Some other broker"), None);
    }

    #[test]
    fn a_layout_made_on_a_file_becomes_a_preset_without_its_owner() {
        let mapping = ImportMapping::detect(&["date".to_string(), "type".to_string()])
            .with_account("acc-1")
            .with_kind_alias("Aankoop", crate::model::TransactionKind::Buy);
        let preset = BrokerPreset::of("X", &ParseConfig::default(), &mapping);

        assert_eq!(preset.mapping.account_id, None);
        // Only the broker's own wording is written down; "BUY" is already common knowledge.
        assert_eq!(preset.mapping.kind_aliases.len(), 1);
        assert_eq!(
            preset.mapping().kind_of("Aankoop"),
            Some(crate::model::TransactionKind::Buy)
        );
        assert_eq!(
            preset.mapping().kind_of("BUY"),
            Some(crate::model::TransactionKind::Buy)
        );
    }
}
