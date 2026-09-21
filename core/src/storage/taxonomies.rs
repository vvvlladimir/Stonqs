use super::{SqlDecimal, Store, dec_to_sql};
use crate::error::{Error, Result};
use crate::model::{CashClassification, SecurityClassification, Taxonomy, TaxonomyKind, TaxonomyNode};
use crate::money::normalize_currency;
use rusqlite::{Row, params};

fn row_to_taxonomy(row: &Row<'_>) -> rusqlite::Result<Taxonomy> {
    let kind: String = row.get("kind")?;
    Ok(Taxonomy {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: TaxonomyKind::parse(&kind).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
    })
}

fn row_to_node(row: &Row<'_>) -> rusqlite::Result<TaxonomyNode> {
    Ok(TaxonomyNode {
        id: row.get("id")?,
        taxonomy_id: row.get("taxonomy_id")?,
        parent_id: row.get("parent_id")?,
        name: row.get("name")?,
        rank: row.get("rank")?,
        color: row.get("color")?,
    })
}

impl Store {
    pub fn save_taxonomy(&self, t: &Taxonomy) -> Result<()> {
        self.conn.execute(
            "INSERT INTO taxonomies (id, name, kind) VALUES (?1, ?2, ?3)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, kind = excluded.kind",
            params![t.id, t.name, t.kind.as_str()],
        )?;
        Ok(())
    }

    pub fn get_taxonomy(&self, id: &str) -> Result<Taxonomy> {
        self.conn
            .query_row("SELECT * FROM taxonomies WHERE id = ?1", [id], row_to_taxonomy)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("taxonomy {id}")),
                other => other.into(),
            })
    }

    pub fn list_taxonomies(&self) -> Result<Vec<Taxonomy>> {
        let mut stmt = self.conn.prepare("SELECT * FROM taxonomies ORDER BY name")?;
        let rows = stmt.query_map([], row_to_taxonomy)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Deletes a taxonomy and its dependent nodes, classifications, and targets.
    pub fn delete_taxonomy(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM taxonomies WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn save_taxonomy_node(&self, n: &TaxonomyNode) -> Result<()> {
        self.conn.execute(
            "INSERT INTO taxonomy_nodes (id, taxonomy_id, parent_id, name, rank, color)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (id) DO UPDATE SET
                 taxonomy_id = excluded.taxonomy_id,
                 parent_id = excluded.parent_id,
                 name = excluded.name,
                 rank = excluded.rank,
                 color = excluded.color",
            params![n.id, n.taxonomy_id, n.parent_id, n.name, n.rank, n.color],
        )?;
        Ok(())
    }

    /// Returns nodes with roots before children for one-pass tree construction.
    pub fn taxonomy_nodes(&self, taxonomy_id: &str) -> Result<Vec<TaxonomyNode>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM taxonomy_nodes WHERE taxonomy_id = ?1
             ORDER BY (parent_id IS NOT NULL), parent_id, rank, name",
        )?;
        let rows = stmt.query_map([taxonomy_id], row_to_node)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn delete_taxonomy_node(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM taxonomy_nodes WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn save_classification(&self, c: &SecurityClassification) -> Result<()> {
        c.validate()?;
        self.conn.execute(
            "INSERT INTO security_classifications (security_id, node_id, weight)
             VALUES (?1, ?2, ?3)
             ON CONFLICT (security_id, node_id) DO UPDATE SET weight = excluded.weight",
            params![c.security_id, c.node_id, dec_to_sql(c.weight)],
        )?;
        Ok(())
    }

    pub fn delete_classification(&self, security_id: &str, node_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM security_classifications WHERE security_id = ?1 AND node_id = ?2",
            params![security_id, node_id],
        )?;
        Ok(())
    }

    /// Returns all security classifications for one taxonomy.
    pub fn classifications_for_taxonomy(&self, taxonomy_id: &str) -> Result<Vec<SecurityClassification>> {
        let mut stmt = self.conn.prepare(
            "SELECT sc.security_id, sc.node_id, sc.weight
             FROM security_classifications sc
             JOIN taxonomy_nodes n ON n.id = sc.node_id
             WHERE n.taxonomy_id = ?1",
        )?;
        let rows = stmt.query_map([taxonomy_id], |r| {
            Ok(SecurityClassification {
                security_id: r.get(0)?,
                node_id: r.get(1)?,
                weight: r.get::<_, SqlDecimal>(2)?.0,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Saves a cash classification for one account and currency.
    pub fn save_cash_classification(&self, c: &CashClassification) -> Result<()> {
        c.validate()?;
        self.conn.execute(
            "INSERT INTO cash_classifications (account_id, currency, node_id, weight)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (account_id, currency, node_id) DO UPDATE SET weight = excluded.weight",
            params![c.account_id, c.currency, c.node_id, dec_to_sql(c.weight)],
        )?;
        Ok(())
    }

    pub fn delete_cash_classification(&self, account_id: &str, currency: &str, node_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM cash_classifications WHERE account_id = ?1 AND currency = ?2 AND node_id = ?3",
            params![account_id, normalize_currency(currency), node_id],
        )?;
        Ok(())
    }

    pub fn cash_classifications_for_taxonomy(&self, taxonomy_id: &str) -> Result<Vec<CashClassification>> {
        let mut stmt = self.conn.prepare(
            "SELECT cc.account_id, cc.currency, cc.node_id, cc.weight
             FROM cash_classifications cc
             JOIN taxonomy_nodes n ON n.id = cc.node_id
             WHERE n.taxonomy_id = ?1",
        )?;
        let rows = stmt.query_map([taxonomy_id], |r| {
            Ok(CashClassification {
                account_id: r.get(0)?,
                currency: r.get(1)?,
                node_id: r.get(2)?,
                weight: r.get::<_, SqlDecimal>(3)?.0,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Returns subjects excluded from one taxonomy.
    pub fn taxonomy_exclusions(&self, taxonomy_id: &str) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT subject_id FROM taxonomy_exclusions WHERE taxonomy_id = ?1 ORDER BY subject_id",
        )?;
        let rows = stmt.query_map([taxonomy_id], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Toggles an exclusion without deleting its classifications.
    pub fn set_taxonomy_exclusion(&self, taxonomy_id: &str, subject_id: &str, excluded: bool) -> Result<()> {
        if excluded {
            self.conn.execute(
                "INSERT INTO taxonomy_exclusions (taxonomy_id, subject_id) VALUES (?1, ?2)
                 ON CONFLICT (taxonomy_id, subject_id) DO NOTHING",
                params![taxonomy_id, subject_id],
            )?;
        } else {
            self.conn.execute(
                "DELETE FROM taxonomy_exclusions WHERE taxonomy_id = ?1 AND subject_id = ?2",
                params![taxonomy_id, subject_id],
            )?;
        }
        Ok(())
    }

    /// Removes all currency-specific cash subjects for an account.
    pub fn forget_cash_subjects(&self, account_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM taxonomy_exclusions WHERE subject_id LIKE ?1",
            [format!("{}%", crate::model::cash_subject_key(account_id, ""))],
        )?;
        Ok(())
    }

    /// Removes exclusions for a subject because `subject_id` has two owners.
    pub fn forget_taxonomy_subject(&self, subject_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM taxonomy_exclusions WHERE subject_id = ?1",
            [subject_id],
        )?;
        Ok(())
    }
}
