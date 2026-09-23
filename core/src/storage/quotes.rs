use super::{SqlDecimal, Store, date_from_sql, date_to_sql, dec_to_sql};
use crate::error::Result;
use crate::market::{DateRange, PriceCache, PriceLookup, PricePoint, Quote};
use chrono::NaiveDate;
use rusqlite::{Row, params};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Quote data actually stored for one security.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuoteStats {
    pub count: usize,
    /// Range from the first to the last stored quote.
    pub range: DateRange,
    /// Currency of the last stored quote.
    pub currency: String,
    /// Last close in `currency`, never in an unrelated security currency.
    pub last_close: Decimal,
}

fn row_to_quote(row: &Row<'_>) -> rusqlite::Result<Quote> {
    let date: String = row.get("date")?;
    Ok(Quote {
        security_id: row.get("security_id")?,
        date: date_from_sql(&date)?,
        close: row.get::<_, SqlDecimal>("close")?.0,
        currency: row.get("currency")?,
        source: row.get("source")?,
    })
}

impl Store {
    /// Saves quotes in one transaction to avoid one fsync per row.
    pub fn save_quotes(&self, quotes: &[Quote]) -> Result<usize> {
        self.write_quotes(
            quotes,
            "INSERT INTO quotes (security_id, date, close, currency, source)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (security_id, date) DO UPDATE SET
                 close = excluded.close,
                 currency = excluded.currency,
                 source = excluded.source",
        )
    }

    /// Saves a fallback source's quotes only on days nothing is stored for, so the instrument's
    /// own source is never overwritten by one that answered in its place. Returns rows written.
    pub fn fill_quotes(&self, quotes: &[Quote]) -> Result<usize> {
        self.write_quotes(
            quotes,
            "INSERT OR IGNORE INTO quotes (security_id, date, close, currency, source)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )
    }

    fn write_quotes(&self, quotes: &[Quote], sql: &str) -> Result<usize> {
        if quotes.is_empty() {
            return Ok(0);
        }
        let tx = self.conn.unchecked_transaction()?;
        let mut saved = 0;
        {
            let mut stmt = tx.prepare(sql)?;
            for q in quotes {
                saved += stmt.execute(params![
                    q.security_id,
                    date_to_sql(q.date),
                    dec_to_sql(q.close),
                    q.currency,
                    q.source
                ])?;
            }
        }
        tx.commit()?;
        Ok(saved)
    }

    pub fn quotes_in_range(&self, security_id: &str, range: DateRange) -> Result<Vec<Quote>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM quotes WHERE security_id = ?1 AND date BETWEEN ?2 AND ?3 ORDER BY date",
        )?;
        let rows = stmt.query_map(
            params![security_id, date_to_sql(range.from), date_to_sql(range.to)],
            row_to_quote,
        )?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Date range already requested from the provider for this security.
    pub fn quote_coverage(&self, security_id: &str) -> Result<Option<DateRange>> {
        let mut stmt = self
            .conn
            .prepare("SELECT from_date, to_date FROM quote_coverage WHERE security_id = ?1")?;
        let mut rows = stmt.query_map([security_id], |r| {
            let f: String = r.get(0)?;
            let t: String = r.get(1)?;
            Ok(DateRange::new(date_from_sql(&f)?, date_from_sql(&t)?))
        })?;
        rows.next().transpose().map_err(Into::into)
    }

    /// First and last quote actually stored, which is not what `quote_coverage` records: a
    /// provider asked for five years and answering with one day leaves coverage wide and this
    /// span empty. That gap is the shape a wrong ticker leaves behind.
    pub fn quote_span(&self, security_id: &str) -> Result<Option<DateRange>> {
        let row: Option<(String, String)> = self.conn.query_row(
            "SELECT min(date), max(date) FROM quotes WHERE security_id = ?1",
            [security_id],
            |r| {
                let from: Option<String> = r.get(0)?;
                let to: Option<String> = r.get(1)?;
                Ok(from.zip(to))
            },
        )?;
        match row {
            Some((from, to)) => Ok(Some(DateRange::new(date_from_sql(&from)?, date_from_sql(&to)?))),
            None => Ok(None),
        }
    }

    /// Extends the single continuous requested range.
    pub fn extend_quote_coverage(&self, security_id: &str, range: DateRange) -> Result<()> {
        let merged = match self.quote_coverage(security_id)? {
            Some(old) => DateRange::new(old.from.min(range.from), old.to.max(range.to)),
            None => range,
        };
        self.conn.execute(
            "INSERT INTO quote_coverage (security_id, from_date, to_date, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))
             ON CONFLICT (security_id) DO UPDATE SET
                 from_date = excluded.from_date,
                 to_date = excluded.to_date,
                 updated_at = excluded.updated_at",
            params![security_id, date_to_sql(merged.from), date_to_sql(merged.to)],
        )?;
        Ok(())
    }

    /// Returns all prices through `upto` for in-memory forward-fill.
    pub fn quote_series(
        &self,
        security_id: &str,
        upto: NaiveDate,
    ) -> Result<BTreeMap<NaiveDate, PricePoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT date, close, currency FROM quotes
             WHERE security_id = ?1 AND date <= ?2 ORDER BY date",
        )?;
        let rows = stmt.query_map(params![security_id, date_to_sql(upto)], |r| {
            let d: String = r.get(0)?;
            Ok((
                date_from_sql(&d)?,
                PricePoint::new(r.get::<_, SqlDecimal>(1)?.0, r.get::<_, String>(2)?),
            ))
        })?;
        Ok(rows.collect::<rusqlite::Result<BTreeMap<_, _>>>()?)
    }

    /// Builds the price cache used by daily portfolio calculations.
    pub fn price_cache(&self, security_ids: &[String], upto: NaiveDate) -> Result<PriceCache> {
        let mut cache = PriceCache::new();
        for id in security_ids {
            cache.insert_series(id, self.quote_series(id, upto)?);
        }
        Ok(cache)
    }

    /// Returns quote count, range, and latest quote metadata for each security.
    pub fn quote_stats(&self) -> Result<BTreeMap<String, QuoteStats>> {
        let mut stmt = self.conn.prepare(
            "SELECT security_id, count(*) AS n, min(date) AS d_from, max(date) AS d_to,
                    (SELECT currency FROM quotes AS q2
                      WHERE q2.security_id = q.security_id
                      ORDER BY q2.date DESC LIMIT 1) AS currency,
                    (SELECT close FROM quotes AS q3
                      WHERE q3.security_id = q.security_id
                      ORDER BY q3.date DESC LIMIT 1) AS last_close
             FROM quotes AS q GROUP BY security_id",
        )?;
        let rows = stmt.query_map([], |r| {
            let from: String = r.get("d_from")?;
            let to: String = r.get("d_to")?;
            Ok((
                r.get::<_, String>("security_id")?,
                QuoteStats {
                    count: r.get::<_, i64>("n")? as usize,
                    range: DateRange::new(date_from_sql(&from)?, date_from_sql(&to)?),
                    currency: r.get("currency")?,
                    last_close: r.get::<_, SqlDecimal>("last_close")?.0,
                },
            ))
        })?;
        Ok(rows.collect::<rusqlite::Result<BTreeMap<_, _>>>()?)
    }

    /// Returns the latest quote currency for detecting listing mismatches.
    pub fn latest_quote_currency(&self, security_id: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT currency FROM quotes WHERE security_id = ?1 ORDER BY date DESC LIMIT 1")?;
        let mut rows = stmt.query_map([security_id], |r| r.get::<_, String>(0))?;
        rows.next().transpose().map_err(Into::into)
    }

    /// Deletes quotes and coverage together when a listing changes. Events the provider reported
    /// go too — another venue reports them in its own currency — while the user's notes stay.
    pub fn delete_quotes(&self, security_id: &str) -> Result<usize> {
        let tx = self.conn.unchecked_transaction()?;
        let removed = tx.execute("DELETE FROM quotes WHERE security_id = ?1", [security_id])?;
        tx.execute("DELETE FROM quote_coverage WHERE security_id = ?1", [security_id])?;
        tx.execute(
            "DELETE FROM security_events WHERE security_id = ?1 AND source IS NOT NULL",
            [security_id],
        )?;
        tx.execute("DELETE FROM event_coverage WHERE security_id = ?1", [security_id])?;
        // Another venue is another ticker at every source, not only at the own one.
        tx.execute(
            "DELETE FROM security_symbols WHERE security_id = ?1",
            [security_id],
        )?;
        tx.commit()?;
        Ok(removed)
    }

    /// Deletes one source's quotes of a security, leaving coverage alone — the provider was never
    /// asked for them. Used to take back quotes a debug build simulated.
    pub fn delete_quotes_from(&self, security_id: &str, source: &str) -> Result<usize> {
        Ok(self.conn.execute(
            "DELETE FROM quotes WHERE security_id = ?1 AND source = ?2",
            params![security_id, source],
        )?)
    }

    pub fn latest_quote_date(&self, security_id: &str) -> Result<Option<NaiveDate>> {
        let s: Option<String> = self.conn.query_row(
            "SELECT max(date) FROM quotes WHERE security_id = ?1",
            [security_id],
            |r| r.get(0),
        )?;
        Ok(s.as_deref().map(date_from_sql).transpose()?)
    }
}

/// Date lookups stay in SQL and use the `(security_id, date)` index.
impl PriceLookup for Store {
    fn price_as_of(&self, security_id: &str, date: NaiveDate) -> Result<Option<PricePoint>> {
        self.price_query(
            "SELECT close, currency FROM quotes
             WHERE security_id = ?1 AND date <= ?2
             ORDER BY date DESC LIMIT 1",
            security_id,
            date,
        )
    }

    /// Returns the previous quote strictly before the latest quote on `date`.
    fn price_before(&self, security_id: &str, date: NaiveDate) -> Result<Option<PricePoint>> {
        self.price_query(
            "SELECT close, currency FROM quotes
             WHERE security_id = ?1
               AND date < (SELECT max(date) FROM quotes WHERE security_id = ?1 AND date <= ?2)
             ORDER BY date DESC LIMIT 1",
            security_id,
            date,
        )
    }
}

impl Store {
    /// Shared query implementation for as-of and previous-price lookups.
    fn price_query(&self, sql: &str, security_id: &str, date: NaiveDate) -> Result<Option<PricePoint>> {
        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = stmt.query_map(params![security_id, date_to_sql(date)], |r| {
            Ok(PricePoint::new(
                r.get::<_, SqlDecimal>(0)?.0,
                r.get::<_, String>(1)?,
            ))
        })?;
        rows.next().transpose().map_err(Into::into)
    }
}
