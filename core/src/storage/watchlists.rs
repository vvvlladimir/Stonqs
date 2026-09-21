use super::Store;
use crate::error::{Error, Result};
use crate::model::Watchlist;
use rusqlite::params;

impl Store {
    /// Writes a list and replaces its instruments, keeping the order given.
    pub fn save_watchlist(&self, list: &Watchlist) -> Result<()> {
        list.validate()?;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO watchlists (id, name) VALUES (?1, ?2)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name",
            params![list.id, list.name.trim()],
        )?;
        tx.execute("DELETE FROM watchlist_items WHERE watchlist_id = ?1", [&list.id])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO watchlist_items (watchlist_id, security_id, position) VALUES (?1, ?2, ?3)",
            )?;
            for (position, security_id) in list.security_ids.iter().enumerate() {
                stmt.execute(params![list.id, security_id, position as i64])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn delete_watchlist(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM watchlists WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn get_watchlist(&self, id: &str) -> Result<Watchlist> {
        let (id, name) = self
            .conn
            .query_row("SELECT id, name FROM watchlists WHERE id = ?1", [id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("watchlist {id}")),
                other => other.into(),
            })?;
        let security_ids = self.watchlist_items(&id)?;
        Ok(Watchlist {
            id,
            name,
            security_ids,
        })
    }

    /// Every list in the order it was created.
    pub fn list_watchlists(&self) -> Result<Vec<Watchlist>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name FROM watchlists ORDER BY rowid")?;
        let heads = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        heads
            .into_iter()
            .map(|(id, name)| {
                let security_ids = self.watchlist_items(&id)?;
                Ok(Watchlist {
                    id,
                    name,
                    security_ids,
                })
            })
            .collect()
    }

    fn watchlist_items(&self, watchlist_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT security_id FROM watchlist_items WHERE watchlist_id = ?1 ORDER BY position")?;
        let ids = stmt
            .query_map([watchlist_id], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        Ok(ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Security, SecurityKind};

    fn store() -> Store {
        let store = Store::open_in_memory().unwrap();
        for (id, symbol) in [("sec-a", "AAA"), ("sec-b", "BBB"), ("sec-c", "CCC")] {
            let mut security = Security::new(symbol, symbol, "EUR", SecurityKind::Stock);
            security.id = id.into();
            store.save_security(&security).unwrap();
        }
        store
    }

    #[test]
    fn a_list_keeps_its_order_and_a_resave_replaces_it() {
        let store = store();
        let mut first = Watchlist::new("Candidates").with("sec-b").with("sec-a");
        let second = Watchlist::new("Dividends").with("sec-c");
        store.save_watchlist(&first).unwrap();
        store.save_watchlist(&second).unwrap();

        first.name = "Ideas".into();
        first.security_ids = vec!["sec-c".into(), "sec-b".into()];
        store.save_watchlist(&first).unwrap();

        let lists = store.list_watchlists().unwrap();
        assert_eq!(lists.len(), 2);
        assert_eq!(lists[0].name, "Ideas", "a resave keeps the list where it was");
        assert_eq!(lists[0].security_ids, ["sec-c", "sec-b"]);
        assert_eq!(lists[1].security_ids, ["sec-c"]);
    }

    #[test]
    fn deleting_an_instrument_takes_it_off_every_list() {
        let store = store();
        let list = Watchlist::new("Candidates").with("sec-a").with("sec-b");
        store.save_watchlist(&list).unwrap();

        store.delete_security("sec-a").unwrap();

        assert_eq!(store.get_watchlist(&list.id).unwrap().security_ids, ["sec-b"]);
    }

    #[test]
    fn an_instrument_is_on_a_list_once() {
        let store = store();
        let list = Watchlist::new("Candidates").with("sec-a").with("sec-a");
        assert!(matches!(store.save_watchlist(&list), Err(Error::Invalid(_))));
        assert!(store.list_watchlists().unwrap().is_empty());
    }
}
