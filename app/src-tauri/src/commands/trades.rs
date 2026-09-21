use crate::commands::named;
use crate::commands::parse_date;
use crate::error::{UiError, UiResult};
use crate::scope::DataScope;
use crate::state::AppState;
use rust_decimal::Decimal;
use serde::Serialize;
use sq_core::calc::{Trade, TradeStats, TradingVolume, trade_stats};
use sq_core::market::DateRange;
use std::collections::HashMap;
use tauri::State;

/// One trade named the way the rest of the app names instruments.
#[derive(Debug, Serialize)]
pub struct TradeRow {
    #[serde(flatten)]
    pub trade: Trade,
    pub symbol: String,
    pub name: String,
}

/// The trade ledger of a window: what is still held, what was closed inside it, and what the
/// trading itself moved. Open trades are as of `to`; a closed one belongs to the window its
/// disposal fell in.
#[derive(Debug, Serialize)]
pub struct TradesData {
    pub from: String,
    pub to: String,
    pub base_currency: String,
    pub open: Vec<TradeRow>,
    pub closed: Vec<TradeRow>,
    pub open_stats: TradeStats,
    pub closed_stats: TradeStats,
    pub volume: TradingVolume,
    /// Traded volume against the capital at work — what the portfolio's own trading amounts to.
    #[serde(with = "rust_decimal::serde::str_option")]
    pub turnover_rate: Option<Decimal>,
}

#[tauri::command]
pub fn trades_summary(
    state: State<AppState>,
    from: String,
    to: String,
    source: Option<DataScope>,
) -> UiResult<TradesData> {
    let from = parse_date(&from)?;
    let to = parse_date(&to)?;
    if to < from {
        return Err(UiError::invalid("the period ends before it starts"));
    }
    let store = state.store()?;
    let scope = state.scope_selection_in(&store, source.as_ref())?;
    let analytics = scope.analytics(&store)?;

    let book = analytics.trades(to).map_err(|e| named(&store, e))?;
    let volume = analytics.trading_volume(from, to)?;
    let summary = analytics
        .period_summary(DateRange::new(from, to))
        .map_err(|e| named(&store, e))?;

    let securities = store.list_securities()?;
    let named_by_id: HashMap<&str, (&str, &str)> = securities
        .iter()
        .map(|s| (s.id.as_str(), (s.symbol.as_str(), s.name.as_str())))
        .collect();
    let row = |trade: Trade| {
        let (symbol, name) = named_by_id
            .get(trade.security_id.as_str())
            .map(|(symbol, name)| ((*symbol).to_string(), (*name).to_string()))
            .unwrap_or_else(|| (trade.security_id.clone(), String::new()));
        TradeRow { trade, symbol, name }
    };

    let closed: Vec<Trade> = book
        .closed
        .into_iter()
        .filter(|t| t.closed_at.is_some_and(|date| date >= from && date <= to))
        .collect();

    Ok(TradesData {
        from: from.to_string(),
        to: to.to_string(),
        base_currency: analytics.base_currency().to_string(),
        open_stats: trade_stats(&book.open),
        closed_stats: trade_stats(&closed),
        turnover_rate: summary.rate_of(volume.volume_base),
        volume,
        open: book.open.into_iter().map(&row).collect(),
        closed: closed.into_iter().map(&row).collect(),
    })
}
