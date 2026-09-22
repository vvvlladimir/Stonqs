use super::{SqlDecimal, Store, date_from_sql, date_to_sql, dec_to_sql};
use crate::error::{Error, Result};
use crate::model::{AccountKind, Transaction, TransactionKind};
use chrono::NaiveDate;
use rusqlite::{Row, params};

fn row_to_transaction(row: &Row<'_>) -> rusqlite::Result<Transaction> {
    let kind: String = row.get("kind")?;
    let date: String = row.get("date")?;
    let fx: Option<SqlDecimal> = row.get("fx_rate_to_base")?;
    let currency: String = row.get("currency")?;
    // A charge in the transaction's own currency is stored as NULL; a row written before that
    // rule existed is folded back to it here, so nothing downstream sees two spellings of one
    // currency.
    let charge_currency = |column| -> rusqlite::Result<Option<String>> {
        Ok(row.get::<_, Option<String>>(column)?.filter(|c| *c != currency))
    };
    Ok(Transaction {
        id: row.get("id")?,
        account_id: row.get("account_id")?,
        security_id: row.get("security_id")?,
        kind: TransactionKind::parse(&kind).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?,
        date: date_from_sql(&date)?,
        quantity: row.get::<_, SqlDecimal>("quantity")?.0,
        price: row.get::<_, SqlDecimal>("price")?.0,
        amount: row.get::<_, SqlDecimal>("amount")?.0,
        fees: row.get::<_, SqlDecimal>("fees")?.0,
        taxes: row.get::<_, SqlDecimal>("taxes")?.0,
        fee_currency: charge_currency("fee_currency")?,
        tax_currency: charge_currency("tax_currency")?,
        currency,
        fx_rate_to_base: fx.map(|d| d.0),
        link_id: row.get("link_id")?,
        external_id: row.get("external_id")?,
        note: row.get("note")?,
        // A row read back is the operation the user entered, never a lens's rewrite of it.
        scoped_from: None,
    })
}

impl Store {
    /// Saves a transaction after validating its invariants.
    pub fn save_transaction(&self, t: &Transaction) -> Result<()> {
        t.validate()?;
        self.check_account_kind(t)?;
        self.conn.execute(
            "INSERT INTO transactions
                 (id, account_id, security_id, kind, date, quantity, price, amount,
                  fees, taxes, currency, fee_currency, tax_currency, fx_rate_to_base, link_id,
                  external_id, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
             ON CONFLICT (id) DO UPDATE SET
                 account_id = excluded.account_id,
                 security_id = excluded.security_id,
                 kind = excluded.kind,
                 date = excluded.date,
                 quantity = excluded.quantity,
                 price = excluded.price,
                 amount = excluded.amount,
                 fees = excluded.fees,
                 taxes = excluded.taxes,
                 currency = excluded.currency,
                 fee_currency = excluded.fee_currency,
                 tax_currency = excluded.tax_currency,
                 fx_rate_to_base = excluded.fx_rate_to_base,
                 link_id = excluded.link_id,
                 external_id = excluded.external_id,
                 note = excluded.note",
            params![
                t.id,
                t.account_id,
                t.security_id,
                t.kind.as_str(),
                date_to_sql(t.date),
                dec_to_sql(t.quantity),
                dec_to_sql(t.price),
                dec_to_sql(t.amount),
                dec_to_sql(t.fees),
                dec_to_sql(t.taxes),
                t.currency,
                t.fee_currency.as_ref().filter(|c| **c != t.currency),
                t.tax_currency.as_ref().filter(|c| **c != t.currency),
                t.fx_rate_to_base.map(dec_to_sql),
                t.link_id,
                t.external_id,
                t.note,
            ],
        )?;
        Ok(())
    }

    /// Rejects transactions whose account kind cannot hold their operation.
    fn check_account_kind(&self, t: &Transaction) -> Result<()> {
        let account = self.get_account(&t.account_id)?;
        match (account.kind, t.kind.requires_security()) {
            (AccountKind::Deposit, true) => Err(Error::Invalid(format!(
                "{:?} on cash account {:?}: instrument transactions belong on a securities account",
                t.kind, account.name
            ))),
            (AccountKind::Securities, false) => Err(Error::Invalid(format!(
                "{:?} on securities account {:?}: cash transactions belong on its cash account",
                t.kind, account.name
            ))),
            _ => Ok(()),
        }
    }

    pub fn delete_transaction(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM transactions WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Returns account transactions in stable `(date, id)` order for FIFO.
    pub fn transactions_for_accounts(
        &self,
        account_ids: &[String],
        until: Option<NaiveDate>,
    ) -> Result<Vec<Transaction>> {
        if account_ids.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", account_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT * FROM transactions
             WHERE account_id IN ({placeholders}) AND (?{n} IS NULL OR date <= ?{n})
             ORDER BY date, id",
            n = account_ids.len() + 1
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = account_ids
            .iter()
            .map(|a| Box::new(a.clone()) as Box<dyn rusqlite::ToSql>)
            .collect();
        values.push(Box::new(until.map(date_to_sql)));
        let refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(refs.as_slice(), row_to_transaction)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Returns both sides of a linked transaction.
    pub fn linked_transactions(&self, link_id: &str) -> Result<Vec<Transaction>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM transactions WHERE link_id = ?1 ORDER BY date, id")?;
        let rows = stmt.query_map([link_id], row_to_transaction)?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn transactions_for_account(&self, account_id: &str) -> Result<Vec<Transaction>> {
        let ids = vec![account_id.to_string()];
        self.transactions_for_accounts(&ids, None)
    }
}
