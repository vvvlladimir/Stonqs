//! Income, rolled up every way the screen offers it.

use crate::commands::parse_date;
use crate::error::UiResult;
use crate::scope::DataScope;
use crate::state::AppState;
use rust_decimal::Decimal;
use serde::Serialize;
use sq_core::calc::{
    IncomeRecord, IncomeSummary, TaxonomyIncome, income_between, income_by_kind, income_by_month,
    income_by_month_of_year, income_by_security, income_by_year, income_by_year_kind, income_of_kind,
    income_total,
};
use sq_core::model::TransactionKind;
use tauri::State;

#[derive(Debug, Serialize)]
pub struct MonthIncome {
    pub year: i32,
    pub month: u32,
    #[serde(flatten)]
    pub summary: IncomeSummary,
}

#[derive(Debug, Serialize)]
pub struct SecurityIncome {
    pub security_id: String,
    pub symbol: String,
    pub name: String,
    #[serde(flatten)]
    pub summary: IncomeSummary,
}

/// One month index with every year folded in: the calendar's bottom row.
#[derive(Debug, Serialize)]
pub struct MonthOfYearIncome {
    pub month: u32,
    #[serde(flatten)]
    pub summary: IncomeSummary,
}

/// One kind inside one year: the composition of a single year bar.
#[derive(Debug, Serialize)]
pub struct YearKindIncome {
    pub year: i32,
    pub kind: TransactionKind,
    #[serde(flatten)]
    pub summary: IncomeSummary,
}

#[derive(Debug, Serialize)]
pub struct KindIncome {
    pub kind: TransactionKind,
    #[serde(flatten)]
    pub summary: IncomeSummary,
}

#[derive(Debug, Serialize)]
pub struct IncomeEvent {
    #[serde(flatten)]
    pub record: IncomeRecord,
    pub symbol: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct IncomeData {
    pub from: String,
    pub to: String,
    pub base_currency: String,
    pub events: Vec<IncomeEvent>,
    pub by_month: Vec<MonthIncome>,
    pub by_month_of_year: Vec<MonthOfYearIncome>,
    pub by_security: Vec<SecurityIncome>,
    pub by_kind: Vec<KindIncome>,
    pub total: IncomeSummary,
    pub previous_total: IncomeSummary,
    pub previous_from: String,
    pub previous_to: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub change_base: Decimal,
    pub by_year: Vec<YearIncome>,
    pub by_year_kind: Vec<YearKindIncome>,
    /// Kind the report was narrowed to, echoed back so the UI can trust its own selector.
    pub kind: Option<TransactionKind>,
}

#[derive(Debug, Serialize)]
pub struct YearIncome {
    pub year: i32,
    #[serde(flatten)]
    pub summary: IncomeSummary,
}

/// The same period's income, read through one classification tree.
#[derive(Debug, Serialize)]
pub struct TaxonomyIncomeData {
    pub taxonomy_id: String,
    pub base_currency: String,
    #[serde(flatten)]
    pub income: TaxonomyIncome,
}

/// Income of a period split by a taxonomy. Its own command rather than a field of
/// [`income_summary`]: the tree is picked separately, and switching it must not refetch
/// every other rollup on the screen.
#[tauri::command]
pub fn income_taxonomy(
    state: State<AppState>,
    taxonomy_id: String,
    from: String,
    to: String,
    kind: Option<String>,
    source: Option<DataScope>,
) -> UiResult<TaxonomyIncomeData> {
    let from = parse_date(&from)?;
    let to = parse_date(&to)?;
    let kind = kind.as_deref().map(TransactionKind::parse).transpose()?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let analytics = scope.analytics(&store)?;
    Ok(TaxonomyIncomeData {
        base_currency: analytics.base_currency().to_string(),
        income: analytics.income_by_taxonomy(&taxonomy_id, from, to, kind)?,
        taxonomy_id,
    })
}

#[tauri::command]
pub fn income_summary(
    state: State<AppState>,
    from: String,
    to: String,
    kind: Option<String>,
    source: Option<DataScope>,
) -> UiResult<IncomeData> {
    let from = parse_date(&from)?;
    let to = parse_date(&to)?;
    let kind = kind.as_deref().map(TransactionKind::parse).transpose()?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let analytics = scope.analytics(&store)?;

    let holdings = analytics.holdings_at(to)?;
    let income = income_of_kind(&income_between(&holdings, from, to), kind);
    let history = income_of_kind(&holdings.income, kind);

    let span = (to - from) + chrono::Duration::days(1);
    let previous_to = from - chrono::Duration::days(1);
    let previous_from = previous_to - span + chrono::Duration::days(1);
    let previous = income_of_kind(&income_between(&holdings, previous_from, previous_to), kind);
    let previous_total = income_total(&previous);
    let total = income_total(&income);

    let securities = store.list_securities()?;
    let named = |id: &str| {
        securities
            .iter()
            .find(|s| s.id == id)
            .map(|s| (s.symbol.clone(), s.name.clone()))
            .unwrap_or_else(|| (id.to_string(), String::new()))
    };

    Ok(IncomeData {
        from: from.to_string(),
        to: to.to_string(),
        base_currency: analytics.base_currency().to_string(),
        by_month: income_by_month(&income)
            .into_iter()
            .map(|((year, month), summary)| MonthIncome { year, month, summary })
            .collect(),
        by_month_of_year: income_by_month_of_year(&income)
            .into_iter()
            .map(|(month, summary)| MonthOfYearIncome { month, summary })
            .collect(),
        by_security: income_by_security(&income)
            .into_iter()
            .map(|(id, summary)| {
                let (symbol, name) = named(&id);
                SecurityIncome {
                    security_id: id,
                    symbol,
                    name,
                    summary,
                }
            })
            .collect(),
        by_kind: income_by_kind(&income)
            .into_iter()
            .map(|(kind, summary)| KindIncome { kind, summary })
            .collect(),
        change_base: total.net_base - previous_total.net_base,
        total,
        previous_total,
        previous_from: previous_from.to_string(),
        previous_to: previous_to.to_string(),
        by_year: income_by_year(&history)
            .into_iter()
            .map(|(year, summary)| YearIncome { year, summary })
            .collect(),
        by_year_kind: income_by_year_kind(&history)
            .into_iter()
            .map(|((year, kind), summary)| YearKindIncome { year, kind, summary })
            .collect(),
        kind,
        events: income
            .into_iter()
            .map(|record| {
                let (symbol, name) = record
                    .security_id
                    .as_deref()
                    .map(named)
                    .unwrap_or_else(|| (String::new(), String::new()));
                IncomeEvent { record, symbol, name }
            })
            .collect(),
    })
}
