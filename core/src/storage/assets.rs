use super::{SqlDecimal, Store, date_from_sql, date_to_sql, dec_to_sql};
use crate::error::{Error, Result};
use crate::model::{Amortization, Asset, AssetKind, AssetValue};
use rusqlite::{Row, params};

fn row_to_asset(row: &Row<'_>) -> rusqlite::Result<Asset> {
    let kind: String = row.get("kind")?;
    let closed_on: Option<String> = row.get("closed_on")?;
    let ends_on: Option<String> = row.get("ends_on")?;
    let rate: Option<SqlDecimal> = row.get("rate")?;
    // The column pair is kept together by a CHECK, so a rate is enough to know there is a schedule.
    let schedule = match (rate, row.get::<_, Option<SqlDecimal>>("monthly_payment")?) {
        (Some(rate), Some(payment)) => Some(Amortization {
            rate: rate.0,
            monthly_payment: payment.0,
            ends_on: ends_on.as_deref().map(date_from_sql).transpose()?,
        }),
        _ => None,
    };
    Ok(Asset {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: AssetKind::parse(&kind).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        currency: row.get("currency")?,
        secured_by: row.get("secured_by")?,
        schedule,
        note: row.get("note")?,
        closed_on: closed_on.as_deref().map(date_from_sql).transpose()?,
    })
}

fn row_to_value(row: &Row<'_>) -> rusqlite::Result<AssetValue> {
    let date: String = row.get("date")?;
    Ok(AssetValue {
        asset_id: row.get("asset_id")?,
        date: date_from_sql(&date)?,
        amount: row.get::<_, SqlDecimal>("amount")?.0,
        note: row.get("note")?,
    })
}

impl Store {
    /// Writes a thing owned or owed. Its valuations are separate rows: saving the asset never
    /// touches what it was worth (ADR-0092).
    pub fn save_asset(&self, portfolio_id: &str, asset: &Asset) -> Result<()> {
        asset.validate()?;
        let schedule = asset.schedule.as_ref();
        self.conn.execute(
            "INSERT INTO assets
                 (id, portfolio_id, name, kind, currency, secured_by, rate, monthly_payment, ends_on,
                  note, closed_on)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT (id) DO UPDATE SET
                 portfolio_id = excluded.portfolio_id,
                 name = excluded.name,
                 kind = excluded.kind,
                 currency = excluded.currency,
                 secured_by = excluded.secured_by,
                 rate = excluded.rate,
                 monthly_payment = excluded.monthly_payment,
                 ends_on = excluded.ends_on,
                 note = excluded.note,
                 closed_on = excluded.closed_on",
            params![
                asset.id,
                portfolio_id,
                asset.name,
                asset.kind.as_str(),
                asset.currency,
                asset.secured_by,
                schedule.map(|s| dec_to_sql(s.rate)),
                schedule.map(|s| dec_to_sql(s.monthly_payment)),
                schedule.and_then(|s| s.ends_on).map(date_to_sql),
                asset.note,
                asset.closed_on.map(date_to_sql),
            ],
        )?;
        Ok(())
    }

    /// Takes its valuations with it, and leaves whatever it secured standing.
    pub fn delete_asset(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM assets WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn get_asset(&self, id: &str) -> Result<Asset> {
        self.conn
            .query_row("SELECT * FROM assets WHERE id = ?1", [id], row_to_asset)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("asset {id}")),
                other => Error::from(other),
            })
    }

    /// Every asset of a portfolio, by name. Which are owned and which are owed is the kind's
    /// business, so the order does not split them.
    pub fn list_assets(&self, portfolio_id: &str) -> Result<Vec<Asset>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM assets WHERE portfolio_id = ?1 ORDER BY name, id")?;
        let assets = stmt
            .query_map([portfolio_id], row_to_asset)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(assets)
    }

    /// One figure per day: re-valuing a day the owner already answered replaces that answer.
    pub fn save_asset_value(&self, value: &AssetValue) -> Result<()> {
        value.validate()?;
        self.conn.execute(
            "INSERT INTO asset_values (asset_id, date, amount, note)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (asset_id, date) DO UPDATE SET
                 amount = excluded.amount,
                 note = excluded.note",
            params![
                value.asset_id,
                date_to_sql(value.date),
                dec_to_sql(value.amount),
                value.note,
            ],
        )?;
        Ok(())
    }

    pub fn delete_asset_value(&self, asset_id: &str, date: chrono::NaiveDate) -> Result<()> {
        self.conn.execute(
            "DELETE FROM asset_values WHERE asset_id = ?1 AND date = ?2",
            params![asset_id, date_to_sql(date)],
        )?;
        Ok(())
    }

    /// The valuation history of one asset, oldest first.
    pub fn asset_values(&self, asset_id: &str) -> Result<Vec<AssetValue>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM asset_values WHERE asset_id = ?1 ORDER BY date")?;
        let values = stmt
            .query_map([asset_id], row_to_value)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(values)
    }

    /// Every valuation of every asset of a portfolio, oldest first. `calc::networth` reads the
    /// whole series at once rather than asking per day, so this is the shape it wants.
    pub fn list_asset_values(&self, portfolio_id: &str) -> Result<Vec<AssetValue>> {
        let mut stmt = self.conn.prepare(
            "SELECT v.* FROM asset_values v
             JOIN assets a ON a.id = v.asset_id
             WHERE a.portfolio_id = ?1
             ORDER BY v.date, v.asset_id",
        )?;
        let values = stmt
            .query_map([portfolio_id], row_to_value)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(values)
    }
}
