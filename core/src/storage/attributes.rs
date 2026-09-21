use super::Store;
use crate::error::{Error, Result};
use crate::model::{AttributeKind, SecurityAttributeDef};
use rusqlite::{Row, params};
use std::collections::BTreeMap;

/// Values of one instrument, keyed by attribute id.
pub type AttributeValues = BTreeMap<String, String>;

fn row_to_def(row: &Row<'_>) -> rusqlite::Result<SecurityAttributeDef> {
    let kind: String = row.get("kind")?;
    Ok(SecurityAttributeDef {
        id: row.get("id")?,
        name: row.get("name")?,
        kind: AttributeKind::parse(&kind).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        unit: row.get("unit")?,
        position: row.get("position")?,
    })
}

impl Store {
    pub fn save_attribute_def(&self, def: &SecurityAttributeDef) -> Result<()> {
        def.validate()?;
        self.conn.execute(
            "INSERT INTO security_attribute_defs (id, name, kind, unit, position)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT (id) DO UPDATE SET
                 name = excluded.name,
                 kind = excluded.kind,
                 unit = excluded.unit,
                 position = excluded.position",
            params![def.id, def.name.trim(), def.kind.as_str(), def.unit, def.position],
        )?;
        Ok(())
    }

    /// Deleting an attribute takes its values with it — the column is gone, not emptied.
    pub fn delete_attribute_def(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM security_attribute_defs WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn list_attribute_defs(&self) -> Result<Vec<SecurityAttributeDef>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM security_attribute_defs ORDER BY position, name")?;
        let rows = stmt.query_map([], row_to_def)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn get_attribute_def(&self, id: &str) -> Result<SecurityAttributeDef> {
        self.conn
            .query_row(
                "SELECT * FROM security_attribute_defs WHERE id = ?1",
                [id],
                row_to_def,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("attribute {id}")),
                other => other.into(),
            })
    }

    pub fn attributes_for_security(&self, security_id: &str) -> Result<AttributeValues> {
        let mut stmt = self
            .conn
            .prepare("SELECT attribute_id, value FROM security_attributes WHERE security_id = ?1")?;
        let rows = stmt.query_map([security_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?;
        Ok(rows.collect::<rusqlite::Result<AttributeValues>>()?)
    }

    /// Every instrument's values in one pass, for the directory listing.
    pub fn security_attributes(&self) -> Result<BTreeMap<String, AttributeValues>> {
        let mut stmt = self
            .conn
            .prepare("SELECT security_id, attribute_id, value FROM security_attributes")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut out: BTreeMap<String, AttributeValues> = BTreeMap::new();
        for row in rows {
            let (security_id, attribute_id, value) = row?;
            out.entry(security_id).or_default().insert(attribute_id, value);
        }
        Ok(out)
    }

    /// Replaces every value of one instrument: a blank clears the attribute rather than
    /// storing an empty string, so "not filled in" has one spelling.
    pub fn set_security_attributes(&self, security_id: &str, values: &AttributeValues) -> Result<()> {
        let mut normalized = Vec::with_capacity(values.len());
        for (attribute_id, value) in values {
            if value.trim().is_empty() {
                continue;
            }
            let def = self.get_attribute_def(attribute_id)?;
            normalized.push((attribute_id.clone(), def.kind.normalize(value)?));
        }

        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "DELETE FROM security_attributes WHERE security_id = ?1",
            [security_id],
        )?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO security_attributes (security_id, attribute_id, value) VALUES (?1, ?2, ?3)",
            )?;
            for (attribute_id, value) in normalized {
                stmt.execute(params![security_id, attribute_id, value])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::model::{AttributeKind, Security, SecurityAttributeDef, SecurityKind};
    use crate::storage::Store;
    use std::collections::BTreeMap;

    fn values(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn values_round_trip_and_a_blank_clears_one() {
        let store = Store::open_in_memory().unwrap();
        let security = Security::new("VWCE", "FTSE All-World", "EUR", SecurityKind::Etf);
        store.save_security(&security).unwrap();

        let ter = SecurityAttributeDef::new("TER", AttributeKind::Number).with_unit("%");
        let country = SecurityAttributeDef::new("Country", AttributeKind::Text);
        store.save_attribute_def(&ter).unwrap();
        store.save_attribute_def(&country).unwrap();

        store
            .set_security_attributes(
                &security.id,
                &values(&[(&ter.id, "0.2200"), (&country.id, "Ireland")]),
            )
            .unwrap();
        let stored = store.attributes_for_security(&security.id).unwrap();
        assert_eq!(stored.get(&ter.id).map(String::as_str), Some("0.22"));
        assert_eq!(stored.get(&country.id).map(String::as_str), Some("Ireland"));

        store
            .set_security_attributes(&security.id, &values(&[(&ter.id, "0.22"), (&country.id, "  ")]))
            .unwrap();
        let stored = store.attributes_for_security(&security.id).unwrap();
        assert_eq!(stored.len(), 1);
        assert!(!stored.contains_key(&country.id));
    }

    #[test]
    fn deleting_the_attribute_deletes_its_values() {
        let store = Store::open_in_memory().unwrap();
        let security = Security::new("IWDA", "MSCI World", "EUR", SecurityKind::Etf);
        store.save_security(&security).unwrap();
        let ter = SecurityAttributeDef::new("TER", AttributeKind::Number);
        store.save_attribute_def(&ter).unwrap();
        store
            .set_security_attributes(&security.id, &values(&[(&ter.id, "0.2")]))
            .unwrap();

        store.delete_attribute_def(&ter.id).unwrap();
        assert!(store.attributes_for_security(&security.id).unwrap().is_empty());
        assert!(store.list_attribute_defs().unwrap().is_empty());
    }

    #[test]
    fn a_value_of_the_wrong_shape_is_refused() {
        let store = Store::open_in_memory().unwrap();
        let security = Security::new("AAPL", "Apple", "USD", SecurityKind::Stock);
        store.save_security(&security).unwrap();
        let listed = SecurityAttributeDef::new("Listed since", AttributeKind::Date);
        store.save_attribute_def(&listed).unwrap();

        let bad = values(&[(&listed.id, "12/12/1980")]);
        assert!(store.set_security_attributes(&security.id, &bad).is_err());
        // The refused write left nothing behind.
        assert!(store.attributes_for_security(&security.id).unwrap().is_empty());
    }
}
