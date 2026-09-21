use crate::commands::named;
use crate::commands::parse_date;
use crate::commands::portfolio::require_text;
use crate::error::{UiError, UiResult};
use crate::events::emit_changed;
use crate::scope::DataScope;
use crate::state::AppState;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sq_core::calc::{Allocation, Assignment, NodeMember};
use sq_core::import::{
    ParseConfig, TaxonomyCsvConfig, TaxonomyPreview, build_taxonomy_preview, commit_taxonomy,
    detect_taxonomy_config, group_by_attribute, parse_csv, taxonomy_to_csv,
};
use sq_core::model::{
    CashClassification, SecurityClassification, TargetWeight, Taxonomy, TaxonomyKind, TaxonomyNode,
    parse_cash_subject,
};
use sq_core::prelude::*;
use std::str::FromStr;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn allocation(
    state: State<AppState>,
    cut: String,
    taxonomy_id: Option<String>,
    date: String,
    source: Option<DataScope>,
) -> UiResult<Allocation> {
    let date = parse_date(&date)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let analytics = scope.analytics(&store)?;

    Ok(match cut.as_str() {
        "currency" => analytics
            .allocation_by_currency(date)
            .map_err(|e| named(&store, e))?,
        "account" => analytics
            .allocation_by_account(date)
            .map_err(|e| named(&store, e))?,
        "security" => analytics
            .allocation_by_security(date)
            .map_err(|e| named(&store, e))?,
        "taxonomy" => {
            let id = taxonomy_id.ok_or_else(|| UiError::invalid("no taxonomy is selected"))?;
            analytics
                .allocation_by_taxonomy(&id, date)
                .map_err(|e| named(&store, e))?
        }
        other => return Err(UiError::invalid(format!("unknown breakdown {other:?}"))),
    })
}

#[tauri::command]
pub fn allocation_tree(
    state: State<AppState>,
    taxonomy_id: String,
    date: String,
    source: Option<DataScope>,
) -> UiResult<Allocation> {
    let date = parse_date(&date)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    scope
        .analytics(&store)?
        .allocation_tree(&taxonomy_id, date)
        .map_err(|e| named(&store, e))
}

#[tauri::command]
pub fn allocation_members(
    state: State<AppState>,
    taxonomy_id: String,
    node_id: Option<String>,
    date: String,
    source: Option<DataScope>,
) -> UiResult<Vec<NodeMember>> {
    let date = parse_date(&date)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    scope
        .analytics(&store)?
        .allocation_members(&taxonomy_id, node_id.as_deref(), date)
        .map_err(|e| named(&store, e))
}

#[derive(Debug, Serialize)]
pub struct TaxonomyData {
    #[serde(flatten)]
    pub taxonomy: Taxonomy,
    pub nodes: Vec<TaxonomyNode>,
    pub classifications: Vec<Assignment>,
    pub excluded: Vec<String>,
}

#[tauri::command]
pub fn taxonomies_list(state: State<AppState>) -> UiResult<Vec<TaxonomyData>> {
    let store = state.store()?;
    store
        .list_taxonomies()?
        .into_iter()
        .map(|taxonomy| {
            let mut classifications: Vec<Assignment> = store
                .classifications_for_taxonomy(&taxonomy.id)?
                .iter()
                .map(Assignment::from)
                .collect();
            classifications.extend(
                store
                    .cash_classifications_for_taxonomy(&taxonomy.id)?
                    .iter()
                    .map(Assignment::from),
            );
            Ok(TaxonomyData {
                nodes: store.taxonomy_nodes(&taxonomy.id)?,
                classifications,
                excluded: store.taxonomy_exclusions(&taxonomy.id)?,
                taxonomy,
            })
        })
        .collect()
}

#[derive(Debug, Deserialize)]
pub struct TaxonomyInput {
    pub id: Option<String>,
    pub name: String,
    /// Absent keeps the kind a tree already has, and makes a new one `Custom`: the kind is a
    /// label the seeded trees carry, not something the user is asked for.
    pub kind: Option<TaxonomyKind>,
}

#[tauri::command]
pub fn taxonomy_save(app: AppHandle, state: State<AppState>, input: TaxonomyInput) -> UiResult<Taxonomy> {
    let name = require_text(&input.name, "taxonomy name")?;
    let store = state.store()?;

    let taxonomy = match &input.id {
        Some(id) => {
            let existing = store.get_taxonomy(id)?;
            Taxonomy {
                name,
                kind: input.kind.unwrap_or(existing.kind),
                ..existing
            }
        }
        None => Taxonomy {
            name,
            ..Taxonomy::new("", input.kind.unwrap_or(TaxonomyKind::Custom))
        },
    };
    store.save_taxonomy(&taxonomy)?;
    drop(store);

    emit_changed(&app, "taxonomies")?;
    Ok(taxonomy)
}

#[tauri::command]
pub fn taxonomy_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_taxonomy(&id)?;
    emit_changed(&app, "taxonomies")
}

#[tauri::command]
pub fn taxonomy_import_preview(
    state: State<AppState>,
    content: Vec<u8>,
    config: Option<TaxonomyCsvConfig>,
    name: Option<String>,
) -> UiResult<TaxonomyPreview> {
    let store = state.store()?;
    let parsed = parse_csv(&content, &ParseConfig::default())?;
    let config = config.unwrap_or_else(|| detect_taxonomy_config(&parsed));
    Ok(build_taxonomy_preview(
        &parsed,
        &config,
        &store.list_securities()?,
        name.as_deref(),
    ))
}

#[tauri::command]
pub fn taxonomy_import_preview_path(
    state: State<AppState>,
    path: String,
    config: Option<TaxonomyCsvConfig>,
    name: Option<String>,
) -> UiResult<TaxonomyPreview> {
    let content = std::fs::read(&path).map_err(|e| UiError::invalid(format!("cannot read {path}: {e}")))?;
    taxonomy_import_preview(state, content, config, name)
}

#[tauri::command]
pub fn taxonomy_import_commit(
    app: AppHandle,
    state: State<AppState>,
    content: Vec<u8>,
    config: Option<TaxonomyCsvConfig>,
    name: Option<String>,
    into: Option<String>,
    with_targets: Option<bool>,
) -> UiResult<Taxonomy> {
    let store = state.store()?;
    let parsed = parse_csv(&content, &ParseConfig::default())?;
    let config = config.unwrap_or_else(|| detect_taxonomy_config(&parsed));
    let preview = build_taxonomy_preview(&parsed, &config, &store.list_securities()?, name.as_deref());

    let portfolio_id = if with_targets.unwrap_or(true) {
        Some(state.portfolio()?.id.clone())
    } else {
        None
    };
    let taxonomy = commit_taxonomy(&store, &preview, into.as_deref(), portfolio_id.as_deref())?;
    drop(store);

    emit_changed(&app, "taxonomies")?;
    Ok(taxonomy)
}

/// Plans a tree from one attribute: a node per distinct value, every instrument carrying that
/// value assigned whole. The tree it would extend decides what is left alone, so the preview a
/// dialog shows and the commit that follows agree.
#[tauri::command]
pub fn taxonomy_group_preview(
    state: State<AppState>,
    attribute_id: String,
    into: Option<String>,
) -> UiResult<TaxonomyPreview> {
    let store = state.store()?;
    plan_grouping(&store, &attribute_id, into.as_deref(), None)
}

#[tauri::command]
pub fn taxonomy_group_commit(
    app: AppHandle,
    state: State<AppState>,
    attribute_id: String,
    into: Option<String>,
    name: Option<String>,
) -> UiResult<Taxonomy> {
    let taxonomy = {
        let store = state.store()?;
        let preview = plan_grouping(&store, &attribute_id, into.as_deref(), name.as_deref())?;
        // Grouping plans no targets, so no portfolio is named: a target is set on the tree itself.
        commit_taxonomy(&store, &preview, into.as_deref(), None)?
    };

    emit_changed(&app, "taxonomies")?;
    Ok(taxonomy)
}

fn plan_grouping(
    store: &Store,
    attribute_id: &str,
    into: Option<&str>,
    name: Option<&str>,
) -> UiResult<TaxonomyPreview> {
    let def = store.get_attribute_def(attribute_id)?;
    let classified = match into {
        Some(id) => store
            .classifications_for_taxonomy(id)?
            .into_iter()
            .map(|c| c.security_id)
            .collect(),
        None => std::collections::BTreeSet::new(),
    };
    let mut preview = group_by_attribute(
        &def,
        &store.list_securities()?,
        &store.security_attributes()?,
        &classified,
    )?;
    if let Some(name) = name {
        preview.name = require_text(name, "name")?;
    }
    Ok(preview)
}

#[tauri::command]
pub fn taxonomy_import_commit_path(
    app: AppHandle,
    state: State<AppState>,
    path: String,
    config: Option<TaxonomyCsvConfig>,
    name: Option<String>,
    into: Option<String>,
    with_targets: Option<bool>,
) -> UiResult<Taxonomy> {
    let content = std::fs::read(&path).map_err(|e| UiError::invalid(format!("cannot read {path}: {e}")))?;
    taxonomy_import_commit(app, state, content, config, name, into, with_targets)
}

#[tauri::command]
pub fn taxonomy_export_csv(state: State<AppState>, taxonomy_id: String) -> UiResult<String> {
    let store = state.store()?;
    let taxonomy = store.get_taxonomy(&taxonomy_id)?;
    let portfolio = state.portfolio()?;
    let targets = store.targets_for_portfolio(&portfolio.id)?;
    Ok(taxonomy_to_csv(
        &taxonomy,
        &store.taxonomy_nodes(&taxonomy_id)?,
        &store.classifications_for_taxonomy(&taxonomy_id)?,
        &store.list_securities()?,
        targets.iter().find(|t| t.taxonomy_id == taxonomy_id),
    ))
}

#[tauri::command]
pub fn taxonomy_export_save(state: State<AppState>, taxonomy_id: String, path: String) -> UiResult<()> {
    let csv = taxonomy_export_csv(state, taxonomy_id)?;
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(csv.as_bytes());
    std::fs::write(&path, bytes).map_err(|e| UiError::invalid(format!("cannot write {path}: {e}")))
}

#[derive(Debug, Deserialize)]
pub struct TaxonomyNodeInput {
    pub id: Option<String>,
    pub taxonomy_id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub rank: i64,
    pub color: Option<i64>,
}

#[tauri::command]
pub fn taxonomy_node_save(
    app: AppHandle,
    state: State<AppState>,
    input: TaxonomyNodeInput,
) -> UiResult<TaxonomyNode> {
    let name = require_text(&input.name, "node name")?;
    let node = TaxonomyNode {
        id: input.id.unwrap_or_else(sq_core::model::new_id),
        taxonomy_id: input.taxonomy_id,
        parent_id: input.parent_id.filter(|p| !p.is_empty()),
        name,
        rank: input.rank,
        color: input.color,
    };

    state.store()?.save_taxonomy_node(&node)?;
    emit_changed(&app, "taxonomies")?;
    Ok(node)
}

#[tauri::command]
pub fn taxonomy_node_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_taxonomy_node(&id)?;
    emit_changed(&app, "taxonomies")
}

#[derive(Debug, Deserialize)]
pub struct ClassificationInput {
    pub subject_id: String,
    pub node_id: String,
    pub weight: String,
}

#[tauri::command]
pub fn classification_save(
    app: AppHandle,
    state: State<AppState>,
    input: ClassificationInput,
) -> UiResult<()> {
    let weight = Decimal::from_str(input.weight.trim().replace(',', ".").as_str())
        .map_err(|e| UiError::invalid(format!("invalid weight {:?}: {e}", input.weight)))?;
    let store = state.store()?;
    match parse_cash_subject(&input.subject_id) {
        Some((account_id, currency)) => {
            let c = CashClassification::new(account_id, currency, &input.node_id, weight);
            c.validate()?;
            store.save_cash_classification(&c)?;
        }
        None => {
            let c = SecurityClassification::new(&input.subject_id, &input.node_id, weight);
            c.validate()?;
            store.save_classification(&c)?;
        }
    }
    drop(store);
    emit_changed(&app, "taxonomies")
}

#[tauri::command]
pub fn classification_delete(
    app: AppHandle,
    state: State<AppState>,
    subject_id: String,
    node_id: String,
) -> UiResult<()> {
    let store = state.store()?;
    match parse_cash_subject(&subject_id) {
        Some((account_id, currency)) => store.delete_cash_classification(account_id, currency, &node_id)?,
        None => store.delete_classification(&subject_id, &node_id)?,
    }
    drop(store);
    emit_changed(&app, "taxonomies")
}

#[tauri::command]
pub fn taxonomy_exclude(
    app: AppHandle,
    state: State<AppState>,
    taxonomy_id: String,
    subject_id: String,
    excluded: bool,
) -> UiResult<()> {
    state
        .store()?
        .set_taxonomy_exclusion(&taxonomy_id, &subject_id, excluded)?;
    emit_changed(&app, "taxonomies")
}

#[tauri::command]
pub fn targets_list(state: State<AppState>) -> UiResult<Vec<AllocationTarget>> {
    let store = state.store()?;
    let portfolio = state.portfolio()?;
    Ok(store.targets_for_portfolio(&portfolio.id)?)
}

#[derive(Debug, Deserialize)]
pub struct TargetInput {
    pub id: Option<String>,
    pub taxonomy_id: String,
    pub name: String,
    pub weights: Vec<(String, String)>,
}

#[tauri::command]
pub fn target_save(app: AppHandle, state: State<AppState>, input: TargetInput) -> UiResult<AllocationTarget> {
    let name = require_text(&input.name, "target name")?;
    let portfolio_id = state.portfolio()?.id.clone();

    let mut weights = Vec::with_capacity(input.weights.len());
    for (node_id, raw) in &input.weights {
        let weight = Decimal::from_str(raw.trim().replace(',', ".").as_str())
            .map_err(|e| UiError::invalid(format!("invalid weight {raw:?}: {e}")))?;
        weights.push(TargetWeight {
            node_id: node_id.clone(),
            weight,
        });
    }

    let target = AllocationTarget {
        id: input.id.unwrap_or_else(sq_core::model::new_id),
        portfolio_id,
        taxonomy_id: input.taxonomy_id,
        name,
        weights,
    };
    target.validate()?;

    state.store()?.save_target(&target)?;
    emit_changed(&app, "targets")?;
    Ok(target)
}

#[tauri::command]
pub fn target_delete(app: AppHandle, state: State<AppState>, id: String) -> UiResult<()> {
    state.store()?.delete_target(&id)?;
    emit_changed(&app, "targets")
}

#[tauri::command]
pub fn rebalance_plan(
    state: State<AppState>,
    target_id: String,
    date: String,
    cash_to_invest: Option<String>,
    allow_sell: Option<bool>,
    source: Option<DataScope>,
) -> UiResult<sq_core::calc::RebalancePlan> {
    let date = parse_date(&date)?;
    let cash = match cash_to_invest.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        Some(raw) => Decimal::from_str(&raw.replace(',', "."))
            .map_err(|e| UiError::invalid(format!("invalid cash to invest {raw:?}: {e}")))?,
        None => Decimal::ZERO,
    };
    if cash.is_sign_negative() {
        return Err(UiError::invalid("cash to invest cannot be negative"));
    }
    let options = RebalanceOptions {
        cash_to_invest: cash,
        allow_sell: allow_sell.unwrap_or(true),
    };

    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let target = store.get_target(&target_id)?;
    scope
        .analytics(&store)?
        .rebalance(&target, date, options)
        .map_err(|e| named(&store, e))
}
