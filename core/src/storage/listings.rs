use super::{Store, dec_to_sql};
use crate::error::Result;
use crate::market::Listing;
use rusqlite::{Row, params};
use rust_decimal::Decimal;
use std::str::FromStr;

fn row_to_listing(row: &Row<'_>) -> rusqlite::Result<Listing> {
    let last: Option<String> = row.get("last_close")?;
    Ok(Listing {
        isin: row.get("isin")?,
        mic: row.get("mic")?,
        ticker: row.get("ticker")?,
        exchange: crate::market::mic::market_name(&row.get::<_, String>("mic")?).map(str::to_string),
        name: row.get("name")?,
        symbol: row.get("symbol")?,
        currency: row.get("currency")?,
        has_history: row.get::<_, Option<i64>>("has_history")?.map(|v| v != 0),
        last_close: last.as_deref().and_then(|s| Decimal::from_str(s).ok()),
        source: row.get("source")?,
    })
}

impl Store {
    /// Replaces the stored listing snapshot for an instrument.
    pub fn save_listings(&self, isin: &str, listings: &[Listing]) -> Result<usize> {
        let isin = isin.trim().to_uppercase();
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM listings WHERE isin = ?1", [&isin])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO listings
                     (isin, mic, ticker, name, symbol, currency, has_history, source, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, datetime('now'))",
            )?;
            for l in listings {
                stmt.execute(params![
                    isin,
                    l.mic,
                    l.ticker,
                    l.name,
                    l.symbol,
                    l.currency,
                    l.has_history.map(i64::from),
                    l.source,
                ])?;
            }
        }
        tx.commit()?;
        Ok(listings.len())
    }

    /// Returns known listings; empty means the directory was not queried.
    pub fn listings(&self, isin: &str) -> Result<Vec<Listing>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM listings WHERE isin = ?1 ORDER BY rowid")?;
        let rows = stmt.query_map([isin.trim().to_uppercase()], row_to_listing)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Stores quote-provider metadata discovered for a listing.
    pub fn update_listing_probe(&self, listing: &Listing) -> Result<()> {
        self.conn.execute(
            "UPDATE listings
             SET currency = ?4, has_history = ?5, last_close = ?6, name = coalesce(?7, name)
             WHERE isin = ?1 AND mic = ?2 AND ticker = ?3",
            params![
                listing.isin.trim().to_uppercase(),
                listing.mic,
                listing.ticker,
                listing.currency,
                listing.has_history.map(i64::from),
                listing.last_close.map(dec_to_sql),
                listing.name,
            ],
        )?;
        Ok(())
    }
}
