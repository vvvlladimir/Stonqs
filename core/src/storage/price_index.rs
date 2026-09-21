use super::{SqlDecimal, Store, date_from_sql, date_to_sql, dec_to_sql};
use crate::error::Result;
use crate::inflation::{IndexLookup, IndexPoint, first_of_month};
use crate::market::DateRange;
use crate::model::normalize_region;
use chrono::NaiveDate;
use rusqlite::params;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

impl Store {
    /// Index levels from one source. A source other than the one the region is already stored
    /// under replaces the whole series rather than filling into it: publishers use different
    /// bases, and a ratio taken across two of them measures the rebasing, not inflation.
    pub fn save_index_from(
        &self,
        region: &str,
        points: &[IndexPoint],
        source: &str,
        asked: DateRange,
    ) -> Result<usize> {
        let region = normalize_region(region);
        let tx = self.conn.unchecked_transaction()?;
        let stored: Option<String> = tx
            .query_row(
                "SELECT source FROM index_coverage WHERE region = ?1",
                [&region],
                |r| r.get(0),
            )
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(other),
            })?;
        if stored.as_deref().is_some_and(|s| s != source) {
            tx.execute("DELETE FROM price_index WHERE region = ?1", [&region])?;
            tx.execute("DELETE FROM index_coverage WHERE region = ?1", [&region])?;
        }

        let mut saved = 0;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO price_index (region, month, value, source) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT (region, month) DO UPDATE SET
                     value = excluded.value, source = excluded.source",
            )?;
            for p in points {
                saved += stmt.execute(params![region, date_to_sql(p.month), dec_to_sql(p.value), source])?;
            }
        }

        // Coverage records what was asked, not what came back: a month nobody has published yet
        // must not make the next refresh ask for the same window again.
        let merged = match coverage_in(&tx, &region)? {
            Some(old) => DateRange::new(old.from.min(asked.from), old.to.max(asked.to)),
            None => asked,
        };
        tx.execute(
            "INSERT INTO index_coverage (region, from_date, to_date, source, updated_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT (region) DO UPDATE SET
                 from_date = excluded.from_date, to_date = excluded.to_date,
                 source = excluded.source, updated_at = excluded.updated_at",
            params![region, date_to_sql(merged.from), date_to_sql(merged.to), source],
        )?;
        tx.commit()?;
        Ok(saved)
    }

    /// The window already asked for, and which source answered it.
    pub fn index_coverage(&self, region: &str) -> Result<Option<(DateRange, String)>> {
        let region = normalize_region(region);
        let mut stmt = self
            .conn
            .prepare("SELECT from_date, to_date, source FROM index_coverage WHERE region = ?1")?;
        let mut rows = stmt.query_map([&region], |r| {
            let (from, to): (String, String) = (r.get(0)?, r.get(1)?);
            Ok((
                DateRange::new(date_from_sql(&from)?, date_from_sql(&to)?),
                r.get::<_, String>(2)?,
            ))
        })?;
        Ok(rows.next().transpose()?)
    }

    /// Every stored level of a region up to `through`, oldest first.
    pub fn index_series(&self, region: &str, through: NaiveDate) -> Result<BTreeMap<NaiveDate, Decimal>> {
        let region = normalize_region(region);
        let mut stmt = self.conn.prepare(
            "SELECT month, value FROM price_index WHERE region = ?1 AND month <= ?2 ORDER BY month",
        )?;
        let rows = stmt.query_map(params![region, date_to_sql(through)], |r| {
            let month: String = r.get(0)?;
            Ok((date_from_sql(&month)?, r.get::<_, SqlDecimal>(1)?.0))
        })?;
        Ok(rows.collect::<rusqlite::Result<BTreeMap<_, _>>>()?)
    }

    /// Levels typed in or seeded by hand. They are their own source, so seeding a region the
    /// user maintains never gets replaced by a provider behind their back.
    pub fn save_index(&self, region: &str, points: &[IndexPoint], asked: DateRange) -> Result<usize> {
        self.save_index_from(region, points, "manual", asked)
    }
}

impl IndexLookup for Store {
    fn index_as_of(&self, region: &str, date: NaiveDate) -> Result<Option<Decimal>> {
        let region = normalize_region(region);
        // An index is a monthly fact: the level holds from the month's first day until the next
        // one is published, so the lookup steps rather than interpolating a daily rate.
        let month = first_of_month(date);
        let mut stmt = self.conn.prepare(
            "SELECT value FROM price_index WHERE region = ?1 AND month <= ?2 ORDER BY month DESC LIMIT 1",
        )?;
        let mut rows = stmt.query_map(params![region, date_to_sql(month)], |r| {
            Ok(r.get::<_, SqlDecimal>(0)?.0)
        })?;
        Ok(rows.next().transpose()?)
    }

    fn index_through(&self, region: &str) -> Result<Option<NaiveDate>> {
        let region = normalize_region(region);
        let mut stmt = self
            .conn
            .prepare("SELECT MAX(month) FROM price_index WHERE region = ?1")?;
        let mut rows = stmt.query_map([&region], |r| {
            r.get::<_, Option<String>>(0)?
                .map(|m| date_from_sql(&m))
                .transpose()
        })?;
        Ok(rows.next().transpose()?.flatten())
    }
}

/// Coverage read inside an open transaction.
fn coverage_in(tx: &rusqlite::Transaction<'_>, region: &str) -> rusqlite::Result<Option<DateRange>> {
    let mut stmt = tx.prepare("SELECT from_date, to_date FROM index_coverage WHERE region = ?1")?;
    let mut rows = stmt.query_map([region], |r| {
        let (from, to): (String, String) = (r.get(0)?, r.get(1)?);
        Ok(DateRange::new(date_from_sql(&from)?, date_from_sql(&to)?))
    })?;
    rows.next().transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn store() -> Store {
        let store = Store::open_in_memory().unwrap();
        let points = vec![
            IndexPoint::new("DE", d("2024-01-01"), dec!(118.0)),
            IndexPoint::new("DE", d("2024-02-01"), dec!(118.6)),
        ];
        store
            .save_index_from(
                "DE",
                &points,
                "eurostat",
                DateRange::new(d("2024-01-01"), d("2024-03-31")),
            )
            .unwrap();
        store
    }

    #[test]
    fn a_level_holds_until_the_next_one_is_published() {
        let store = store();
        assert_eq!(
            store.index_as_of("DE", d("2024-01-01")).unwrap(),
            Some(dec!(118.0))
        );
        assert_eq!(
            store.index_as_of("DE", d("2024-01-31")).unwrap(),
            Some(dec!(118.0))
        );
        assert_eq!(
            store.index_as_of("DE", d("2024-02-01")).unwrap(),
            Some(dec!(118.6))
        );
        // March was asked for but not published; February's level is the last known one.
        assert_eq!(
            store.index_as_of("DE", d("2024-03-20")).unwrap(),
            Some(dec!(118.6))
        );
        assert_eq!(store.index_through("DE").unwrap(), Some(d("2024-02-01")));
    }

    #[test]
    fn nothing_is_read_from_before_the_series() {
        let store = store();
        assert_eq!(store.index_as_of("DE", d("2023-12-31")).unwrap(), None);
        assert_eq!(store.index_as_of("FR", d("2024-02-01")).unwrap(), None);
        assert_eq!(store.index_through("FR").unwrap(), None);
    }

    #[test]
    fn coverage_records_the_window_asked_for() {
        let store = store();
        let (range, source) = store.index_coverage("de").unwrap().unwrap();
        assert_eq!((range.from, range.to), (d("2024-01-01"), d("2024-03-31")));
        assert_eq!(source, "eurostat");
    }

    #[test]
    fn another_source_replaces_the_series_and_its_coverage() {
        let store = store();
        store
            .save_index_from(
                "DE",
                &[IndexPoint::new("DE", d("2024-02-01"), dec!(130.5))],
                "imf",
                DateRange::new(d("2024-02-01"), d("2024-02-29")),
            )
            .unwrap();
        assert_eq!(store.index_as_of("DE", d("2024-01-15")).unwrap(), None);
        assert_eq!(
            store.index_as_of("DE", d("2024-02-15")).unwrap(),
            Some(dec!(130.5))
        );
        let (range, source) = store.index_coverage("DE").unwrap().unwrap();
        assert_eq!((range.from, range.to), (d("2024-02-01"), d("2024-02-29")));
        assert_eq!(source, "imf");
    }
}
