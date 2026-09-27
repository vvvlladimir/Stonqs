//! How concentrated a portfolio is — an assistant tool, and an example of the contract
//! (ADR-0085). It is handed the positions the manifest asked for and the model's `top`, and
//! answers with figures of its own (ADR-0082): the app itself states weights, not this.
//!
//! `f64` because these are statistics of weights, not money; nothing here is added to a balance.

use serde::Deserialize;
use serde_json::json;

wit_bindgen::generate!({
    path: "wit",
    world: "tool",
});

#[derive(Deserialize)]
struct Row {
    symbol: String,
    weight: String,
}

#[derive(Deserialize)]
struct Positions {
    rows: Vec<Row>,
}

#[derive(Deserialize)]
struct Data {
    positions: Positions,
}

#[derive(Deserialize)]
struct Args {
    top: u32,
}

struct Concentration;

impl Guest for Concentration {
    fn call(args: String, data: String) -> Result<String, String> {
        let args: Args = serde_json::from_str(&args).map_err(|e| format!("arguments: {e}"))?;
        let data: Data = serde_json::from_str(&data).map_err(|e| format!("data: {e}"))?;
        let top = args.top.clamp(1, 20) as usize;

        let mut rows: Vec<(String, f64)> = data
            .positions
            .rows
            .into_iter()
            .map(|r| (r.symbol, r.weight.parse::<f64>().unwrap_or(0.0)))
            .filter(|(_, w)| *w > 0.0)
            .collect();
        if rows.is_empty() {
            return Ok(json!({ "positions": 0 }).to_string());
        }
        rows.sort_by(|a, b| b.1.total_cmp(&a.1));

        let squares: f64 = rows.iter().map(|(_, w)| w * w).sum();
        let shown = &rows[..top.min(rows.len())];
        let share: f64 = shown.iter().map(|(_, w)| w).sum();
        let percent = |w: f64| format!("{:.1}", w * 100.0);

        Ok(json!({
            "positions": rows.len(),
            "effective_positions": format!("{:.1}", 1.0 / squares),
            "top": shown.len(),
            "top_share_percent": percent(share),
            "largest": shown
                .iter()
                .map(|(symbol, w)| json!({ "symbol": symbol, "weight_percent": percent(*w) }))
                .collect::<Vec<_>>(),
        })
        .to_string())
    }
}

export!(Concentration);
