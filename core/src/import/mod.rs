//! CSV import for transactions, quotes, and taxonomies.
//! Detection stays overridable and preview remains deterministic until commit.

mod attributes;
mod canonical;
mod checks;
mod dedupe;
mod grouping;
mod ibflex;
mod mapping;

mod parse;
mod presets;
mod preview;
mod prices;
mod securities;
mod service;
mod taxonomy;

pub use attributes::{
    AttributeCsvConfig, AttributeImportResult, AttributePreview, AttributeRow, PreviewAttribute,
    attributes_to_csv, build_attribute_preview, commit_attributes, detect_attribute_config,
};
pub use canonical::{CanonicalFile, CanonicalRow, canonical_to_file, is_canonical, parse_canonical};
pub use checks::{
    BasisVote, CheckContext, Direction, SignVote, decide_amount_basis, decide_amount_sign, resolve_direction,
};
pub use dedupe::{KnownRow, fingerprint, fingerprint_of};
pub use grouping::group_by_attribute;
pub use ibflex::{is_flex, parse_flex};
pub use mapping::{
    AmountBasis, AmountSign, Condition, Emit, ImportField, ImportMapping, ImportRule, Sign, Test,
    default_kind_aliases, normalize_alias,
};
pub use parse::{
    ImportProblem, KNOWN_DATE_FORMATS, ParseConfig, ParsedCsv, ProblemCode, Severity, parse_csv,
    parse_date_with, parse_decimal,
};

/// Reads a broker file of either shape. A Flex statement is XML and carries its own layout, so
/// the file itself decides which reader runs — the caller never has to ask (ADR-0061).
pub fn parse_file(content: &[u8], config: &ParseConfig) -> crate::error::Result<ParsedCsv> {
    if is_canonical(content) {
        parse_canonical(content)
    } else if is_flex(content) {
        parse_flex(content)
    } else {
        parse_csv(content, config)
    }
}
pub use presets::{BrokerPreset, PresetMatch, best_match, builtin_presets, parse_presets, presets_to_json};
pub use preview::{
    AccountMapping, ImportContext, ImportPreview, ImportRow, ImportSummary, KindMapping, RowOverride,
    RowStatus, SymbolMapping, TransactionDraft, build_preview,
};
pub use prices::{PriceField, PriceImport, PriceMapping, build_price_import};
pub use securities::SecurityDraft;
pub use service::{ImportOptions, ImportResult, ImportService};
pub use taxonomy::{
    PreviewAssignment, PreviewNode, TaxonomyCsvConfig, TaxonomyPreview, build_taxonomy_preview,
    commit_taxonomy, detect_taxonomy_config, taxonomy_to_csv,
};
