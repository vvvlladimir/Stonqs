//! The period axis: the seven shipped presets plus whatever the user added, resolved to dates
//! by the core. A screen picks an id off this list and hands the dates to a query — it never
//! learns which of the two kinds it picked.

use crate::commands::parse_date;
use crate::error::{UiError, UiResult};
use crate::settings::{AppSettings, PeriodSettings, UserPeriod};
use crate::state::AppState;
use serde::Serialize;
use sq_core::calc::{PeriodPreset, PeriodSpec};
use tauri::State;

/// Every shipped preset with the id it travels under. The id is stored UI state
/// (`Widget.cfg.period`), so a variant is added, never renamed — see ADR-0018.
pub const BUILTIN_PRESETS: &[(&str, PeriodPreset)] = &[
    ("ONE_MONTH", PeriodPreset::OneMonth),
    ("THREE_MONTHS", PeriodPreset::ThreeMonths),
    ("YTD", PeriodPreset::Ytd),
    ("ONE_YEAR", PeriodPreset::OneYear),
    ("THREE_YEARS", PeriodPreset::ThreeYears),
    ("FIVE_YEARS", PeriodPreset::FiveYears),
    ("SINCE_INCEPTION", PeriodPreset::SinceInception),
];

#[derive(Debug, Serialize)]
pub struct PeriodRange {
    /// A shipped preset's code, or a user period's id.
    pub id: String,
    /// The user's own wording. `None` for a shipped preset, whose label the frontend owns —
    /// a name the user typed is their data, not text the host invented (ADR-0023).
    pub name: Option<String>,
    pub from: String,
    pub to: String,
}

#[tauri::command]
pub fn period_ranges(state: State<AppState>, as_of: String) -> UiResult<Vec<PeriodRange>> {
    let as_of = parse_date(&as_of)?;
    let store = state.store()?;
    let scope = state.scope_selection(&store)?;
    let inception = scope.analytics(&store)?.inception()?;
    let settings = state.settings()?;

    // A period the current history cannot answer is dropped, not offered broken: a fixed
    // window before the first transaction, or "since inception" on an empty portfolio.
    let builtin = BUILTIN_PRESETS
        .iter()
        .filter(|(id, _)| !settings.hidden_presets.iter().any(|h| h == id))
        .filter_map(|(id, preset)| {
            preset.range(as_of, inception).ok().map(|range| PeriodRange {
                id: (*id).to_string(),
                name: None,
                from: range.from.to_string(),
                to: range.to.to_string(),
            })
        });

    let user = settings.periods.iter().filter_map(|period| {
        period.spec.range(as_of, inception).ok().map(|range| PeriodRange {
            id: period.id.clone(),
            name: Some(period.name.clone()),
            from: range.from.to_string(),
            to: range.to.to_string(),
        })
    });

    Ok(builtin.chain(user).collect())
}

#[tauri::command]
pub fn periods_get(state: State<AppState>) -> UiResult<PeriodSettings> {
    let settings = state.settings()?;
    Ok(PeriodSettings {
        periods: settings.periods.clone(),
        hidden_presets: settings.hidden_presets.clone(),
    })
}

/// Adds a period, or replaces the one with the same id. The spec is resolved once against
/// today so an impossible window is refused at the point the user typed it, not silently
/// dropped from the strip later.
#[tauri::command]
pub fn period_save(state: State<AppState>, period: UserPeriod) -> UiResult<PeriodSettings> {
    let name = period.name.trim().to_string();
    if name.is_empty() {
        return Err(UiError::invalid("a period must have a name"));
    }
    if period.id.trim().is_empty() {
        return Err(UiError::invalid("a period must have an id"));
    }
    if BUILTIN_PRESETS.iter().any(|(id, _)| *id == period.id) {
        return Err(UiError::invalid("that id belongs to a shipped preset"));
    }
    // An open end is not a missing one: it means "up to today", so only a written end is checked.
    if let PeriodSpec::Fixed { from, to: Some(to) } = period.spec
        && to < from
    {
        return Err(UiError::invalid("the period ends before it starts"));
    }

    let period = UserPeriod { name, ..period };
    {
        let mut settings = state.settings()?;
        settings.periods.retain(|p| p.id != period.id);
        settings.periods.push(period);
    }
    state.persist_settings()?;
    periods_get(state)
}

/// Removes what the user sees under this id. Their own period goes for good; a shipped preset
/// is only written down as hidden, so `periods_restore` can bring it back — the same bargain
/// as the shipped import layouts.
#[tauri::command]
pub fn period_delete(state: State<AppState>, id: String) -> UiResult<PeriodSettings> {
    {
        let mut settings = state.settings()?;
        // Every screen picks its window off this list, so the last period is not removable.
        if remaining_after(&settings, &id) == 0 {
            return Err(UiError::invalid("the period strip cannot be left empty"));
        }
        let had_own = settings.periods.iter().any(|p| p.id == id);
        settings.periods.retain(|p| p.id != id);
        if !had_own
            && BUILTIN_PRESETS.iter().any(|(preset, _)| *preset == id)
            && !settings.hidden_presets.contains(&id)
        {
            settings.hidden_presets.push(id);
        }
    }
    state.persist_settings()?;
    periods_get(state)
}

/// How many periods the strip would still offer once `id` is gone. Counted on the stored axis
/// rather than on resolved ranges: what a given history can answer changes with the data, and
/// an empty portfolio would otherwise let the user delete everything.
fn remaining_after(settings: &AppSettings, id: &str) -> usize {
    let builtin = BUILTIN_PRESETS
        .iter()
        .filter(|(preset, _)| *preset != id && !settings.hidden_presets.iter().any(|h| h == preset))
        .count();
    builtin + settings.periods.iter().filter(|p| p.id != id).count()
}

/// Brings back every shipped preset the user hid. Their own periods are untouched.
#[tauri::command]
pub fn periods_restore(state: State<AppState>) -> UiResult<PeriodSettings> {
    state.settings()?.hidden_presets.clear();
    state.persist_settings()?;
    periods_get(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sq_core::calc::Period;

    fn settings(hidden: &[&str], own: &[&str]) -> AppSettings {
        AppSettings {
            hidden_presets: hidden.iter().map(|h| h.to_string()).collect(),
            periods: own
                .iter()
                .map(|id| UserPeriod {
                    id: (*id).to_string(),
                    name: (*id).to_string(),
                    spec: PeriodSpec::Relative {
                        unit: Period::Year,
                        count: 1,
                    },
                })
                .collect(),
            ..AppSettings::default()
        }
    }

    /// Seven shipped presets, minus the ones already hidden, minus the one being removed.
    #[test]
    fn hiding_a_preset_leaves_the_others() {
        let all = settings(&[], &[]);
        assert_eq!(remaining_after(&all, "YTD"), 6);
        assert_eq!(remaining_after(&all, "p-mine"), 7);

        let thinned = settings(&["ONE_MONTH", "THREE_MONTHS"], &[]);
        assert_eq!(remaining_after(&thinned, "YTD"), 4);
    }

    /// The last preset is only removable while a period of the user's own stands in for it.
    #[test]
    fn the_strip_cannot_be_emptied() {
        let hidden: Vec<&str> = BUILTIN_PRESETS.iter().map(|(id, _)| *id).skip(1).collect();
        let last = settings(&hidden, &[]);
        assert_eq!(remaining_after(&last, "ONE_MONTH"), 0);

        let with_own = settings(&hidden, &["p-mine"]);
        assert_eq!(remaining_after(&with_own, "ONE_MONTH"), 1);
        assert_eq!(remaining_after(&with_own, "p-mine"), 1);
    }
}
