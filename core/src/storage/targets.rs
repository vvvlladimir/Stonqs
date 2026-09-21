use super::{SqlDecimal, Store, dec_to_sql};
use crate::error::{Error, Result};
use crate::model::{AllocationTarget, TargetWeight};
use rusqlite::params;

impl Store {
    /// Saves a target and replaces its weights after validating the taxonomy.
    pub fn save_target(&self, t: &AllocationTarget) -> Result<()> {
        t.validate_tree(&self.taxonomy_nodes(&t.taxonomy_id)?)?;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO targets (id, portfolio_id, taxonomy_id, name)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (id) DO UPDATE SET
                 portfolio_id = excluded.portfolio_id,
                 taxonomy_id = excluded.taxonomy_id,
                 name = excluded.name",
            params![t.id, t.portfolio_id, t.taxonomy_id, t.name],
        )?;
        tx.execute("DELETE FROM target_weights WHERE target_id = ?1", [&t.id])?;
        {
            let mut stmt =
                tx.prepare("INSERT INTO target_weights (target_id, node_id, weight) VALUES (?1, ?2, ?3)")?;
            for w in &t.weights {
                stmt.execute(params![t.id, w.node_id, dec_to_sql(w.weight)])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn get_target(&self, id: &str) -> Result<AllocationTarget> {
        let (portfolio_id, taxonomy_id, name): (String, String, String) = self
            .conn
            .query_row(
                "SELECT portfolio_id, taxonomy_id, name FROM targets WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("target {id}")),
                other => Error::from(other),
            })?;

        let mut stmt = self
            .conn
            .prepare("SELECT node_id, weight FROM target_weights WHERE target_id = ?1 ORDER BY node_id")?;
        let weights = stmt
            .query_map([id], |r| {
                Ok(TargetWeight {
                    node_id: r.get(0)?,
                    weight: r.get::<_, SqlDecimal>(1)?.0,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(AllocationTarget {
            id: id.to_string(),
            portfolio_id,
            taxonomy_id,
            name,
            weights,
        })
    }

    pub fn targets_for_portfolio(&self, portfolio_id: &str) -> Result<Vec<AllocationTarget>> {
        let ids = {
            let mut stmt = self
                .conn
                .prepare("SELECT id FROM targets WHERE portfolio_id = ?1 ORDER BY name")?;
            stmt.query_map([portfolio_id], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        ids.iter().map(|id| self.get_target(id)).collect()
    }

    pub fn delete_target(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM targets WHERE id = ?1", [id])?;
        Ok(())
    }
}
