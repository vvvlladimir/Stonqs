use super::{SqlDecimal, Store, date_from_sql, date_to_sql, dec_to_sql};
use crate::error::{Error, Result};
use crate::model::{Interval, InvestmentPlan, PlanLeg, Schedule};
use chrono::NaiveDate;
use rusqlite::{Row, params};
use std::collections::BTreeSet;

fn row_to_plan(row: &Row<'_>) -> rusqlite::Result<InvestmentPlan> {
    let start: String = row.get("start_date")?;
    let end: Option<String> = row.get("end_date")?;
    let unit: String = row.get("interval_unit")?;
    let convert =
        |e: Error| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e));
    Ok(InvestmentPlan {
        id: row.get("id")?,
        portfolio_id: row.get("portfolio_id")?,
        account_id: row.get("account_id")?,
        name: row.get("name")?,
        amount: row.get::<_, SqlDecimal>("amount")?.0,
        currency: row.get("currency")?,
        fees: row.get::<_, SqlDecimal>("fees")?.0,
        taxes: row.get::<_, SqlDecimal>("taxes")?.0,
        schedule: Schedule {
            start: date_from_sql(&start)?,
            end: end.as_deref().map(date_from_sql).transpose()?,
            unit: Interval::parse(&unit).map_err(convert)?,
            count: row.get::<_, i64>("interval_count")? as u32,
        },
        active: row.get::<_, i64>("active")? != 0,
        note: row.get("note")?,
        legs: Vec::new(),
    })
}

impl Store {
    /// Writes a plan and replaces its legs. Leg order is the user's and is kept.
    pub fn save_plan(&self, plan: &InvestmentPlan) -> Result<()> {
        plan.validate()?;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO investment_plans
                 (id, portfolio_id, account_id, name, amount, currency, fees, taxes,
                  start_date, end_date, interval_unit, interval_count, active, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT (id) DO UPDATE SET
                 portfolio_id = excluded.portfolio_id,
                 account_id = excluded.account_id,
                 name = excluded.name,
                 amount = excluded.amount,
                 currency = excluded.currency,
                 fees = excluded.fees,
                 taxes = excluded.taxes,
                 start_date = excluded.start_date,
                 end_date = excluded.end_date,
                 interval_unit = excluded.interval_unit,
                 interval_count = excluded.interval_count,
                 active = excluded.active,
                 note = excluded.note",
            params![
                plan.id,
                plan.portfolio_id,
                plan.account_id,
                plan.name,
                dec_to_sql(plan.amount),
                plan.currency,
                dec_to_sql(plan.fees),
                dec_to_sql(plan.taxes),
                date_to_sql(plan.schedule.start),
                plan.schedule.end.map(date_to_sql),
                plan.schedule.unit.as_str(),
                plan.schedule.count as i64,
                plan.active as i64,
                plan.note,
            ],
        )?;
        tx.execute("DELETE FROM plan_legs WHERE plan_id = ?1", [&plan.id])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO plan_legs (plan_id, security_id, weight, position) VALUES (?1, ?2, ?3, ?4)",
            )?;
            for (position, leg) in plan.legs.iter().enumerate() {
                stmt.execute(params![
                    plan.id,
                    leg.security_id,
                    dec_to_sql(leg.weight),
                    position as i64
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn delete_plan(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM investment_plans WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn get_plan(&self, id: &str) -> Result<InvestmentPlan> {
        let mut plan = self
            .conn
            .query_row("SELECT * FROM investment_plans WHERE id = ?1", [id], row_to_plan)
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Error::NotFound(format!("plan {id}")),
                other => Error::from(other),
            })?;
        plan.legs = self.plan_legs(id)?;
        Ok(plan)
    }

    /// Every plan of a portfolio, active ones first and then by name.
    pub fn list_plans(&self, portfolio_id: &str) -> Result<Vec<InvestmentPlan>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM investment_plans WHERE portfolio_id = ?1 ORDER BY active DESC, name, id",
        )?;
        let mut plans = stmt
            .query_map([portfolio_id], row_to_plan)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for plan in &mut plans {
            plan.legs = self.plan_legs(&plan.id)?;
        }
        Ok(plans)
    }

    fn plan_legs(&self, plan_id: &str) -> Result<Vec<PlanLeg>> {
        let mut stmt = self
            .conn
            .prepare("SELECT security_id, weight FROM plan_legs WHERE plan_id = ?1 ORDER BY position")?;
        let legs = stmt
            .query_map([plan_id], |r| {
                Ok(PlanLeg {
                    security_id: r.get(0)?,
                    weight: r.get::<_, SqlDecimal>(1)?.0,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(legs)
    }

    /// Occurrence dates already committed. A row whose transaction was deleted is gone with it
    /// (`ON DELETE CASCADE`), which is what makes deleting the transaction offer the month again.
    pub fn plan_executions(&self, plan_id: &str) -> Result<BTreeSet<NaiveDate>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT occurrence_date FROM plan_executions WHERE plan_id = ?1")?;
        let dates = stmt
            .query_map([plan_id], |r| {
                let raw: String = r.get(0)?;
                date_from_sql(&raw)
            })?
            .collect::<rusqlite::Result<BTreeSet<_>>>()?;
        Ok(dates)
    }

    /// Transactions committed for one occurrence, so the screen can link back to them.
    pub fn plan_execution_transactions(&self, plan_id: &str, date: NaiveDate) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT transaction_id FROM plan_executions WHERE plan_id = ?1 AND occurrence_date = ?2",
        )?;
        let ids = stmt
            .query_map(params![plan_id, date_to_sql(date)], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        Ok(ids)
    }

    /// Records that these transactions are the plan's answer to one occurrence. The caller has
    /// already written them; this is the link, not the write.
    pub fn record_plan_execution(
        &self,
        plan_id: &str,
        date: NaiveDate,
        transaction_ids: &[String],
    ) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO plan_executions (plan_id, occurrence_date, transaction_id)
                 VALUES (?1, ?2, ?3)",
            )?;
            for id in transaction_ids {
                stmt.execute(params![plan_id, date_to_sql(date), id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Account, AccountKind, Portfolio, Security, SecurityKind, Transaction};
    use rust_decimal_macros::dec;

    fn d(s: &str) -> NaiveDate {
        s.parse().unwrap()
    }

    fn store() -> Store {
        let store = Store::open_in_memory().unwrap();
        let portfolio = Portfolio::new("Main", "EUR");
        store.save_portfolio(&portfolio).unwrap();
        let cash = Account::deposit("Cash", "EUR");
        store.save_account(&cash).unwrap();
        let depot = Account::securities("Depot", "EUR", &cash.id);
        store.save_account(&depot).unwrap();
        let mut vwce = Security::new("VWCE", "Vanguard All-World", "EUR", SecurityKind::Etf);
        vwce.id = "sec-vwce".into();
        store.save_security(&vwce).unwrap();
        store
    }

    fn plan(store: &Store) -> InvestmentPlan {
        let portfolio = store.list_portfolios().unwrap().remove(0);
        let depot = store
            .list_accounts()
            .unwrap()
            .into_iter()
            .find(|a| a.kind == AccountKind::Securities)
            .unwrap();
        InvestmentPlan::new(
            &portfolio.id,
            &depot.id,
            "Monthly",
            dec!(500),
            "EUR",
            Schedule::monthly(d("2024-01-05")),
        )
        .with_leg("sec-vwce", dec!(1))
    }

    #[test]
    fn a_plan_round_trips_with_its_legs() {
        let store = store();
        let plan = plan(&store);
        store.save_plan(&plan).unwrap();
        let back = store.get_plan(&plan.id).unwrap();
        assert_eq!(back, plan);
    }

    #[test]
    fn saving_twice_replaces_the_legs_rather_than_adding_to_them() {
        let store = store();
        let mut plan = plan(&store);
        store.save_plan(&plan).unwrap();
        plan.legs[0].weight = dec!(2);
        store.save_plan(&plan).unwrap();
        assert_eq!(store.get_plan(&plan.id).unwrap().legs.len(), 1);
    }

    #[test]
    fn deleting_the_transaction_un_executes_the_occurrence() {
        let store = store();
        let plan = plan(&store);
        store.save_plan(&plan).unwrap();
        let t = Transaction::buy(
            &plan.account_id,
            "sec-vwce",
            d("2024-01-05"),
            dec!(4),
            dec!(100),
            "EUR",
        );
        store.save_transaction(&t).unwrap();
        store
            .record_plan_execution(&plan.id, d("2024-01-05"), std::slice::from_ref(&t.id))
            .unwrap();
        assert!(
            store
                .plan_executions(&plan.id)
                .unwrap()
                .contains(&d("2024-01-05"))
        );

        store.delete_transaction(&t.id).unwrap();
        assert!(store.plan_executions(&plan.id).unwrap().is_empty());
    }
}
