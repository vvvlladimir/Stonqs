use super::{SqlDecimal, Store, date_from_sql, date_to_sql, dec_to_sql};
use crate::error::{Error, Result};
use crate::model::{ContributionLimit, Goal};
use rusqlite::{Row, params};

fn row_to_goal(row: &Row<'_>) -> rusqlite::Result<Goal> {
    let target_date: Option<String> = row.get("target_date")?;
    let created_at: String = row.get("created_at")?;
    Ok(Goal {
        id: row.get("id")?,
        name: row.get("name")?,
        target_amount: row.get::<_, SqlDecimal>("target_amount")?.0,
        currency: row.get("currency")?,
        target_date: target_date.as_deref().map(date_from_sql).transpose()?,
        monthly_amount: row.get::<_, Option<SqlDecimal>>("monthly_amount")?.map(|v| v.0),
        expected_return: row.get::<_, SqlDecimal>("expected_return")?.0,
        note: row.get("note")?,
        created_at: date_from_sql(&created_at)?,
        accounts: Vec::new(),
    })
}

fn row_to_limit(row: &Row<'_>) -> rusqlite::Result<ContributionLimit> {
    Ok(ContributionLimit {
        id: row.get("id")?,
        account_id: row.get("account_id")?,
        name: row.get("name")?,
        amount: row.get::<_, SqlDecimal>("amount")?.0,
        currency: row.get("currency")?,
        year_starts_on: row.get("year_starts_on")?,
        note: row.get("note")?,
    })
}

impl Store {
    /// Writes a goal and replaces the accounts it counts. An empty list is the whole portfolio,
    /// so it is stored as no rows rather than as every account of the day.
    pub fn save_goal(&self, portfolio_id: &str, goal: &Goal) -> Result<()> {
        goal.validate()?;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO goals
                 (id, portfolio_id, name, target_amount, currency, target_date, monthly_amount,
                  expected_return, note, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT (id) DO UPDATE SET
                 portfolio_id = excluded.portfolio_id,
                 name = excluded.name,
                 target_amount = excluded.target_amount,
                 currency = excluded.currency,
                 target_date = excluded.target_date,
                 monthly_amount = excluded.monthly_amount,
                 expected_return = excluded.expected_return,
                 note = excluded.note",
            params![
                goal.id,
                portfolio_id,
                goal.name,
                dec_to_sql(goal.target_amount),
                goal.currency,
                goal.target_date.map(date_to_sql),
                goal.monthly_amount.map(dec_to_sql),
                dec_to_sql(goal.expected_return),
                goal.note,
                date_to_sql(goal.created_at),
            ],
        )?;
        tx.execute("DELETE FROM goal_accounts WHERE goal_id = ?1", [&goal.id])?;
        {
            let mut stmt = tx.prepare("INSERT INTO goal_accounts (goal_id, account_id) VALUES (?1, ?2)")?;
            for account_id in &goal.accounts {
                stmt.execute(params![goal.id, account_id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn delete_goal(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM goals WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn get_goal(&self, id: &str) -> Result<Goal> {
        let mut goal = self
            .conn
            .query_row("SELECT * FROM goals WHERE id = ?1", [id], row_to_goal)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("goal {id}")),
                other => Error::from(other),
            })?;
        goal.accounts = self.goal_accounts(id)?;
        Ok(goal)
    }

    /// Every goal of a portfolio, the ones with a date first and then by name.
    pub fn list_goals(&self, portfolio_id: &str) -> Result<Vec<Goal>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM goals WHERE portfolio_id = ?1
             ORDER BY target_date IS NULL, target_date, name, id",
        )?;
        let mut goals = stmt
            .query_map([portfolio_id], row_to_goal)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for goal in &mut goals {
            goal.accounts = self.goal_accounts(&goal.id)?;
        }
        Ok(goals)
    }

    fn goal_accounts(&self, goal_id: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT account_id FROM goal_accounts WHERE goal_id = ?1 ORDER BY account_id")?;
        let accounts = stmt
            .query_map([goal_id], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(accounts)
    }

    pub fn save_limit(&self, limit: &ContributionLimit) -> Result<()> {
        limit.validate()?;
        self.conn.execute(
            "INSERT INTO contribution_limits
                 (id, account_id, name, amount, currency, year_starts_on, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (id) DO UPDATE SET
                 account_id = excluded.account_id,
                 name = excluded.name,
                 amount = excluded.amount,
                 currency = excluded.currency,
                 year_starts_on = excluded.year_starts_on,
                 note = excluded.note",
            params![
                limit.id,
                limit.account_id,
                limit.name,
                dec_to_sql(limit.amount),
                limit.currency,
                limit.year_starts_on,
                limit.note,
            ],
        )?;
        Ok(())
    }

    pub fn delete_limit(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM contribution_limits WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Every limit in the database, by account and then by name. An account may carry two:
    /// somebody with two allowances over one account states both (ADR-0068).
    pub fn list_limits(&self) -> Result<Vec<ContributionLimit>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM contribution_limits ORDER BY account_id, name, id")?;
        let limits = stmt
            .query_map([], row_to_limit)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(limits)
    }
}
