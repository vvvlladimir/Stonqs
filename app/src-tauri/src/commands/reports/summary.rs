//! The reports screen's one reading: realised gains, dividends and charges over a window, each
//! beside the same window a step earlier.
//!
//! Three unrelated reports share one command because they share one build of the holdings and
//! one comparison window. They do not share anything else, so each is assembled on its own and
//! only spread into the payload at the end.

use super::names::Names;
use super::types::*;
use crate::commands::parse_date;
use crate::error::UiResult;
use crate::scope::DataScope;
use crate::state::AppState;
use chrono::{Duration, NaiveDate};
use rust_decimal::Decimal;
use sq_core::calc::{
    ChargeRecord, DividendSummary, Holdings, IncomeRecord, RealizedGain, RealizedSummary,
    capital_gains_by_security, capital_gains_by_year, capital_gains_total, charges_between,
    charges_by_account, charges_by_kind, charges_by_year, charges_total, dividend_profiles, dividend_records,
    dividend_yield, dividends_by_security, dividends_by_year, dividends_total, income_between,
    realized_between, yield_on_cost,
};
use tauri::State;

/// The window immediately before `[from, to]`, of the same length — what "versus the previous
/// window" means on this screen.
fn previous_window(from: NaiveDate, to: NaiveDate) -> (NaiveDate, NaiveDate) {
    let span = (to - from) + Duration::days(1);
    let previous_to = from - Duration::days(1);
    (previous_to - span + Duration::days(1), previous_to)
}

#[tauri::command]
pub fn reports_summary(
    state: State<AppState>,
    from: String,
    to: String,
    source: Option<DataScope>,
) -> UiResult<ReportsData> {
    let from = parse_date(&from)?;
    let to = parse_date(&to)?;
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let analytics = scope.analytics(&store)?;

    // One build covers both windows: the earlier one ends before `from`.
    let holdings = analytics.holdings_at(to)?;
    let (previous_from, previous_to) = previous_window(from, to);
    let names = Names::of(&store)?;

    let gains = gains(
        &realized_between(&holdings, from, to),
        &realized_between(&holdings, previous_from, previous_to),
        &names,
    );
    let dividends = dividends(
        &income_between(&holdings, from, to),
        &income_between(&holdings, previous_from, previous_to),
        &holdings,
        to,
        &names,
    );
    let charges = charges(
        &charges_between(&holdings, from, to),
        &charges_between(&holdings, previous_from, previous_to),
        &names,
    );

    Ok(ReportsData {
        from: from.to_string(),
        to: to.to_string(),
        previous_from: previous_from.to_string(),
        previous_to: previous_to.to_string(),
        base_currency: analytics.base_currency().to_string(),

        gains_by_year: gains.by_year,
        gains_by_security: gains.by_security,
        gains_change_base: gains.change_base,
        gains_return_on_cost: gains.return_on_cost,
        gains_total: gains.total,
        gains_previous: gains.previous,
        disposals: gains.disposals,

        dividends_by_year: dividends.by_year,
        dividends_by_security: dividends.by_security,
        dividends_change_base: dividends.change_base,
        dividends_total: dividends.total,
        dividends_previous: dividends.previous,
        payments: dividends.payments,

        charges_by_year: charges.by_year,
        charges_by_kind: charges.by_kind,
        charges_by_account: charges.by_account,
        charges_change_base: charges.change_base,
        charges_total_base: charges.total_base,
        charges_total: charges.total,
        charges_previous: charges.previous,
        charge_rows: charges.rows,
    })
}

struct Gains {
    by_year: Vec<YearGains>,
    by_security: Vec<SecurityGains>,
    change_base: Decimal,
    return_on_cost: Option<Decimal>,
    total: RealizedSummary,
    previous: RealizedSummary,
    disposals: Vec<DisposalRow>,
}

fn gains(realized: &[RealizedGain], earlier: &[RealizedGain], names: &Names) -> Gains {
    let gains = capital_gains_total(realized);
    let gains_was = capital_gains_total(earlier);
    Gains {
        by_year: capital_gains_by_year(realized)
            .into_iter()
            .map(|(year, summary)| YearGains {
                year,
                return_on_cost: summary.return_on_cost(),
                summary,
            })
            .collect(),
        by_security: capital_gains_by_security(realized)
            .into_iter()
            .map(|(id, summary)| {
                let (symbol, name) = names.security(&id);
                SecurityGains {
                    security_id: id,
                    symbol,
                    name,
                    return_on_cost: summary.return_on_cost(),
                    summary,
                }
            })
            .collect(),
        change_base: gains.gain_base - gains_was.gain_base,
        return_on_cost: gains.return_on_cost(),
        total: gains,
        previous: gains_was,
        disposals: realized
            .iter()
            .map(|gain| {
                let (symbol, name) = names.security(&gain.security_id);
                DisposalRow {
                    symbol,
                    name,
                    return_on_cost: gain.return_on_cost(),
                    gain: gain.clone(),
                }
            })
            .collect(),
    }
}

struct Dividends {
    by_year: Vec<YearDividends>,
    by_security: Vec<SecurityDividends>,
    change_base: Decimal,
    total: DividendSummary,
    previous: DividendSummary,
    payments: Vec<PaymentRow>,
}

fn dividends(
    income: &[IncomeRecord],
    earlier: &[IncomeRecord],
    holdings: &Holdings,
    to: NaiveDate,
    names: &Names,
) -> Dividends {
    // Schedules come from the whole history; every other figure here is the window's.
    let profiles = dividend_profiles(&holdings.income, to);
    let dividends = dividends_total(income);
    let dividends_was = dividends_total(earlier);
    Dividends {
        by_year: dividends_by_year(income)
            .into_iter()
            .map(|(year, summary)| YearDividends { year, summary })
            .collect(),
        by_security: dividends_by_security(income)
            .into_iter()
            .map(|(id, summary)| {
                let (symbol, name) = names.security(&id);
                let profile = profiles.get(&id).cloned().unwrap_or_default();
                let cost_base = holdings
                    .positions
                    .get(&id)
                    .map(|p| p.cost_basis_base)
                    .unwrap_or_default();
                SecurityDividends {
                    yield_on_cost: yield_on_cost(holdings, &id),
                    annual_yield_on_cost: dividend_yield(profile.trailing_year_base, cost_base),
                    frequency: profile.frequency,
                    last_payment: profile.last_payment.map(|d| d.to_string()),
                    payments_total: profile.payments,
                    trailing_year_base: profile.trailing_year_base,
                    security_id: id,
                    symbol,
                    name,
                    summary,
                }
            })
            .collect(),
        change_base: dividends.net_base - dividends_was.net_base,
        total: dividends,
        previous: dividends_was,
        payments: dividend_records(income)
            .map(|record| {
                let (symbol, name) = record
                    .security_id
                    .as_deref()
                    .map(|id| names.security(id))
                    .unwrap_or_else(|| (String::new(), String::new()));
                PaymentRow {
                    symbol,
                    name,
                    account: names.account(&record.account_id),
                    record: record.clone(),
                }
            })
            .collect(),
    }
}

struct Charges {
    by_year: Vec<YearCharges>,
    by_kind: Vec<KindCharges>,
    by_account: Vec<AccountCharges>,
    change_base: Decimal,
    total_base: Decimal,
    total: sq_core::calc::ChargeSummary,
    previous: sq_core::calc::ChargeSummary,
    rows: Vec<ChargeRow>,
}

fn charges(charges: &[ChargeRecord], earlier: &[ChargeRecord], names: &Names) -> Charges {
    let paid = charges_total(charges);
    let paid_was = charges_total(earlier);
    Charges {
        by_year: charges_by_year(charges)
            .into_iter()
            .map(|(year, summary)| YearCharges {
                year,
                total_base: summary.total_base(),
                summary,
            })
            .collect(),
        by_kind: charges_by_kind(charges)
            .into_iter()
            .map(|(kind, summary)| KindCharges {
                kind,
                total_base: summary.total_base(),
                summary,
            })
            .collect(),
        by_account: charges_by_account(charges)
            .into_iter()
            .map(|(id, summary)| AccountCharges {
                account: names.account(&id),
                account_id: id,
                total_base: summary.total_base(),
                summary,
            })
            .collect(),
        change_base: paid.total_base() - paid_was.total_base(),
        total_base: paid.total_base(),
        total: paid,
        previous: paid_was,
        rows: charges
            .iter()
            .map(|record| {
                let (symbol, name) = record
                    .security_id
                    .as_deref()
                    .map(|id| names.security(id))
                    .unwrap_or_else(|| (String::new(), String::new()));
                ChargeRow {
                    symbol,
                    name,
                    account: names.account(&record.account_id),
                    record: record.clone(),
                }
            })
            .collect(),
    }
}
