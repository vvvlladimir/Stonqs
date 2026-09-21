use super::Store;
use crate::error::{Error, Result};
use crate::model::{CostBasisMethod, Portfolio};
use rusqlite::params;

impl Store {
    /// Saves a portfolio and replaces its account membership atomically.
    pub fn save_portfolio(&self, p: &Portfolio) -> Result<()> {
        let tx = begin(&self.conn)?;
        tx.execute(
            "INSERT INTO portfolios (id, name, base_currency, cost_basis_method, inflation_region)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (id) DO UPDATE SET
                 name = excluded.name,
                 base_currency = excluded.base_currency,
                 cost_basis_method = excluded.cost_basis_method,
                 inflation_region = excluded.inflation_region",
            params![
                p.id,
                p.name,
                p.base_currency,
                p.cost_basis_method.as_str(),
                p.inflation_region
            ],
        )?;
        tx.execute("DELETE FROM portfolio_accounts WHERE portfolio_id = ?1", [&p.id])?;
        {
            let mut stmt =
                tx.prepare("INSERT INTO portfolio_accounts (portfolio_id, account_id) VALUES (?1, ?2)")?;
            for acc in &p.account_ids {
                stmt.execute(params![p.id, acc])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_portfolio(&self, id: &str) -> Result<Portfolio> {
        let (name, base_currency, method, inflation_region): (String, String, String, Option<String>) = self
            .conn
            .query_row(
                "SELECT name, base_currency, cost_basis_method, inflation_region
                     FROM portfolios WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("portfolio {id}")),
                other => Error::from(other),
            })?;
        let cost_basis_method = CostBasisMethod::parse(&method)?;

        let mut stmt = self.conn.prepare(
            "SELECT account_id FROM portfolio_accounts WHERE portfolio_id = ?1 ORDER BY account_id",
        )?;
        let account_ids = stmt
            .query_map([id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(Portfolio {
            id: id.to_string(),
            name,
            base_currency,
            account_ids,
            cost_basis_method,
            inflation_region,
        })
    }

    pub fn list_portfolios(&self) -> Result<Vec<Portfolio>> {
        let ids = {
            let mut stmt = self.conn.prepare("SELECT id FROM portfolios ORDER BY name")?;
            stmt.query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        ids.iter().map(|id| self.get_portfolio(id)).collect()
    }
}

/// Uses an unchecked transaction because `Store` exposes an immutable connection.
fn begin(conn: &rusqlite::Connection) -> rusqlite::Result<rusqlite::Transaction<'_>> {
    conn.unchecked_transaction()
}
