use super::{SqlDecimal, Store, date_from_sql, date_to_sql, dec_to_sql};
use crate::error::Result;
use crate::fx::{FxRate, RateCache, RateLookup};
use crate::market::DateRange;
use crate::money::normalize_currency;
use chrono::NaiveDate;
use rusqlite::params;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

impl Store {
    /// Rates typed in or seeded by hand; they overwrite whatever a source wrote for those days.
    pub fn save_fx_rates(&self, rates: &[FxRate]) -> Result<usize> {
        self.save_fx_rates_from(rates, "manual", true)
    }

    /// Rates from one source. The pair's own source overwrites (`overwrite`); a fallback only
    /// fills days nothing was stored for, so it never rewrites the primary's series.
    pub fn save_fx_rates_from(&self, rates: &[FxRate], source: &str, overwrite: bool) -> Result<usize> {
        if rates.is_empty() {
            return Ok(0);
        }
        let sql = if overwrite {
            "INSERT INTO fx_rates (base, quote, date, rate, source) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (base, quote, date) DO UPDATE SET rate = excluded.rate, source = excluded.source"
        } else {
            "INSERT OR IGNORE INTO fx_rates (base, quote, date, rate, source) VALUES (?1, ?2, ?3, ?4, ?5)"
        };
        let tx = self.conn.unchecked_transaction()?;
        let mut saved = 0;
        {
            let mut stmt = tx.prepare(sql)?;
            for r in rates {
                saved += stmt.execute(params![
                    r.base,
                    r.quote,
                    date_to_sql(r.date),
                    dec_to_sql(r.rate),
                    source
                ])?;
            }
        }
        tx.commit()?;
        Ok(saved)
    }

    /// Counts one request against `source`'s allowance for `day`, unless it is already spent.
    /// Returns whether the request may go out.
    pub fn take_request(&self, source: &str, day: NaiveDate, per_day: u32) -> Result<bool> {
        let tx = self.conn.unchecked_transaction()?;
        let spent: u32 = tx
            .query_row(
                "SELECT requests FROM source_usage WHERE source = ?1 AND day = ?2",
                params![source, date_to_sql(day)],
                |r| r.get(0),
            )
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(0),
                other => Err(other),
            })?;
        if spent >= per_day {
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO source_usage (source, day, requests) VALUES (?1, ?2, 1)
             ON CONFLICT (source, day) DO UPDATE SET requests = requests + 1",
            params![source, date_to_sql(day)],
        )?;
        tx.commit()?;
        Ok(true)
    }

    /// Requests spent per source on `day`.
    pub fn requests_on(&self, day: NaiveDate) -> Result<BTreeMap<String, u32>> {
        let mut stmt = self
            .conn
            .prepare("SELECT source, requests FROM source_usage WHERE day = ?1")?;
        let rows = stmt.query_map(params![date_to_sql(day)], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<BTreeMap<_, _>>>()?)
    }

    /// Which source wrote each stored day of a pair.
    pub fn fx_rate_sources(&self, base: &str, quote: &str) -> Result<BTreeMap<NaiveDate, String>> {
        let (base, quote) = (normalize_currency(base), normalize_currency(quote));
        let mut stmt = self
            .conn
            .prepare("SELECT date, source FROM fx_rates WHERE base = ?1 AND quote = ?2 ORDER BY date")?;
        let rows = stmt.query_map(params![base, quote], |r| {
            let d: String = r.get(0)?;
            Ok((date_from_sql(&d)?, r.get::<_, String>(1)?))
        })?;
        Ok(rows.collect::<rusqlite::Result<BTreeMap<_, _>>>()?)
    }

    pub fn fx_rates_in_range(&self, base: &str, quote: &str, range: DateRange) -> Result<Vec<FxRate>> {
        let (base, quote) = (normalize_currency(base), normalize_currency(quote));
        let mut stmt = self.conn.prepare(
            "SELECT date, rate FROM fx_rates
             WHERE base = ?1 AND quote = ?2 AND date BETWEEN ?3 AND ?4
             ORDER BY date",
        )?;
        let rows = stmt.query_map(
            params![base, quote, date_to_sql(range.from), date_to_sql(range.to)],
            |r| {
                let d: String = r.get(0)?;
                Ok((date_from_sql(&d)?, r.get::<_, SqlDecimal>(1)?.0))
            },
        )?;
        Ok(rows
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .map(|(d, rate)| FxRate::new(&base, &quote, d, rate))
            .collect())
    }

    /// Returns the full rate series through `upto`.
    pub fn rate_series(
        &self,
        base: &str,
        quote: &str,
        upto: NaiveDate,
    ) -> Result<BTreeMap<NaiveDate, Decimal>> {
        let (base, quote) = (normalize_currency(base), normalize_currency(quote));
        let mut stmt = self.conn.prepare(
            "SELECT date, rate FROM fx_rates
             WHERE base = ?1 AND quote = ?2 AND date <= ?3
             ORDER BY date",
        )?;
        let rows = stmt.query_map(params![base, quote, date_to_sql(upto)], |r| {
            let d: String = r.get(0)?;
            Ok((date_from_sql(&d)?, r.get::<_, SqlDecimal>(1)?.0))
        })?;
        Ok(rows.collect::<rusqlite::Result<BTreeMap<_, _>>>()?)
    }

    /// Builds a cache with one stored direction per currency pair.
    pub fn rate_cache(&self, pairs: &[(String, String)], upto: NaiveDate) -> Result<RateCache> {
        let mut cache = RateCache::new();
        for (from, to) in pairs {
            if normalize_currency(from) == normalize_currency(to) {
                continue;
            }
            let direct = self.rate_series(from, to, upto)?;
            if direct.is_empty() {
                // Store the inverse series; lookup reverses it on demand.
                let inverse = self.rate_series(to, from, upto)?;
                if !inverse.is_empty() {
                    cache.insert_series(to, from, inverse);
                }
            } else {
                cache.insert_series(from, to, direct);
            }
        }
        Ok(cache)
    }

    /// Reads only the direct stored rate.
    fn direct_rate_as_of(&self, base: &str, quote: &str, date: NaiveDate) -> Result<Option<Decimal>> {
        let mut stmt = self.conn.prepare(
            "SELECT rate FROM fx_rates
             WHERE base = ?1 AND quote = ?2 AND date <= ?3
             ORDER BY date DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![base, quote, date_to_sql(date)], |r| {
            Ok(r.get::<_, SqlDecimal>(0)?.0)
        })?;
        rows.next().transpose().map_err(Into::into)
    }
}

impl RateLookup for Store {
    /// Resolves identity, direct, then inverse rates; no synthetic cross-rates.
    fn rate_as_of(&self, from: &str, to: &str, date: NaiveDate) -> Result<Option<Decimal>> {
        let (from, to) = (normalize_currency(from), normalize_currency(to));
        if from == to {
            return Ok(Some(Decimal::ONE));
        }
        if let Some(r) = self.direct_rate_as_of(&from, &to, date)? {
            return Ok(Some(r));
        }
        match self.direct_rate_as_of(&to, &from, date)? {
            Some(r) if !r.is_zero() => Ok(Some(Decimal::ONE / r)),
            _ => Ok(None),
        }
    }
}
