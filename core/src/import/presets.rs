//! Broker layouts shipped with the application.
//!
//! Detection reads an unknown file; a preset is the answer for a file we have already seen.
//! It is data, not code: adding a broker means one entry in `presets/brokers.json`, which is
//! also the shape a downloaded preset would arrive in.

use super::mapping::{ImportMapping, default_kind_aliases};
use super::parse::ParseConfig;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// One broker's layout: how to read the file and what its columns and wordings mean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrokerPreset {
    pub name: String,

    #[serde(default)]
    pub config: ParseConfig,

    pub mapping: ImportMapping,
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
