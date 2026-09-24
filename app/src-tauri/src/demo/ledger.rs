//! The demo's operations: three years of a monthly savings habit, with the irregular things a
//! real ledger also holds — a portfolio moved in from another broker, currency exchanges, a
//! closed trade, dividends, custody fees and one withdrawal.

use super::World;
use chrono::{Datelike, Months, NaiveDate};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sq_core::model::floor_to_step;
use sq_core::prelude::*;
use std::collections::BTreeMap;

pub fn write(store: &Store, world: &World) -> Result<()> {
    let mut book = Book {
        world,
        rows: Vec::new(),
        held: BTreeMap::new(),
        savings: Decimal::ZERO,
    };
    book.run();
    for tx in &book.rows {
        store.save_transaction(tx)?;
    }
    Ok(())
}

struct Book<'a> {
    world: &'a World,
    rows: Vec<Transaction>,
    /// Quantity per security id, so a dividend is paid on what was actually held that month.
    held: BTreeMap<String, Decimal>,
    savings: Decimal,
}

impl Book<'_> {
    fn run(&mut self) {
        let first = self.world.days[0];
        for month in 0u32.. {
            let Some(anchor) = first
                .with_day(1)
                .and_then(|d| d.checked_add_months(Months::new(month)))
                .and_then(|d| d.with_day(5))
            else {
                break;
            };
            if anchor > self.world.today {
                break;
            }
            if anchor >= first {
                self.month(month, anchor);
            }
        }
    }

    fn month(&mut self, month: u32, anchor: NaiveDate) {
        let euro_cash = self.world.accounts.euro_cash.id.clone();
        let i = self.world.index_on(anchor);
        let rate = self.world.rate(i);

        if month == 0 {
            self.push(Transaction::cash(
                &euro_cash,
                TransactionKind::Deposit,
                anchor,
                dec!(6000),
                "EUR",
            ));
            // The shares a new user brings with them: quantity and cost, no cash leg.
            let core_world = self.world.instruments.core_world.id.clone();
            let price = self.world.prices[&core_world][i];
            let quantity = dec!(60);
            let depot = self.world.accounts.euro_depot.id.clone();
            self.push(
                Transaction::delivery_inbound(&depot, &core_world, anchor, quantity, quantity * price, "EUR")
                    .with_note("Transferred from a previous broker"),
            );
            self.add(&core_world, quantity);
            self.exchange(anchor, dec!(1500), rate);
        } else {
            self.push(Transaction::cash(
                &euro_cash,
                TransactionKind::Deposit,
                anchor,
                dec!(1150),
                "EUR",
            ));
        }

        let allworld = self.world.instruments.allworld.id.clone();
        let bonds = self.world.instruments.bonds.id.clone();
        self.buy_eur(&allworld, anchor, i, dec!(450));
        self.buy_eur(&bonds, anchor, i, dec!(200));

        // Dollars are bought when they are about to be spent, so neither account sits on a
        // balance the portfolio is not using.
        if month % 3 == 1 {
            self.exchange(anchor, dec!(1150), rate);
            let security = if month % 6 == 1 {
                self.world.instruments.apple.id.clone()
            } else {
                self.world.instruments.microsoft.id.clone()
            };
            self.buy_usd(&security, anchor, i, dec!(1200), rate);
        }

        let netflix = self.world.instruments.netflix.id.clone();
        let bitcoin = self.world.instruments.bitcoin.id.clone();
        let apple = self.world.instruments.apple.id.clone();
        let microsoft = self.world.instruments.microsoft.id.clone();
        match month {
            4 => {
                self.exchange(anchor, dec!(1300), rate);
                self.buy_usd(&netflix, anchor, i, dec!(1300), rate);
            }
            6 => self.buy_eur(&bitcoin, anchor, i, dec!(800)),
            20 => self.buy_eur(&bitcoin, anchor, i, dec!(600)),
            // A sale is a rotation, not a pile of idle dollars: the proceeds go back in.
            26 => {
                self.sell_usd(&netflix, anchor, i, None, rate);
                self.buy_usd(&microsoft, anchor, i, dec!(3200), rate);
            }
            30 => {
                self.sell_usd(&apple, anchor, i, Some(dec!(0.4)), rate);
                self.buy_usd(&microsoft, anchor, i, dec!(3600), rate);
            }
            _ => {}
        }

        match month {
            2 => self.transfer_to_savings(anchor, dec!(3000)),
            22 => self.transfer_from_savings(anchor, dec!(1200)),
            33 => self.push(Transaction::cash(
                &euro_cash,
                TransactionKind::Withdrawal,
                anchor,
                dec!(1500),
                "EUR",
            )),
            _ => {}
        }

        self.dividends(anchor);
        self.interest(anchor);

        if anchor.month() == 12 {
            self.push(
                Transaction::cash(&euro_cash, TransactionKind::Fee, anchor, dec!(24.90), "EUR")
                    .with_note("Annual custody fee"),
            );
        }
        if anchor.month() == 3 {
            self.push(
                Transaction::cash(&euro_cash, TransactionKind::Tax, anchor, dec!(61.40), "EUR")
                    .with_note("Settlement of last year's investment income tax"),
            );
        }
    }

    /// Payments land on the 15th, which is not the day anything was bought: an income calendar
    /// where every figure shares a date says nothing about seasonality.
    fn dividends(&mut self, anchor: NaiveDate) {
        let Some(pay_day) = anchor.with_day(15) else {
            return;
        };
        if pay_day > self.world.today {
            return;
        }
        let i = self.world.index_on(pay_day);
        let rate = self.world.rate(i);
        let quarter = anchor.month() % 3;

        let apple = self.world.instruments.apple.id.clone();
        let microsoft = self.world.instruments.microsoft.id.clone();
        let bonds = self.world.instruments.bonds.id.clone();
        if quarter == 2 {
            self.dividend_usd(&apple, pay_day, dec!(0.25), rate);
        }
        if quarter == 0 {
            self.dividend_usd(&microsoft, pay_day, dec!(0.83), rate);
        }
        if quarter == 1 {
            let amount = (self.quantity(&bonds) * dec!(0.032)).round_dp(2);
            if amount > Decimal::ZERO {
                let depot = self.world.accounts.euro_depot.id.clone();
                self.push(Transaction::dividend(&depot, &bonds, pay_day, amount, "EUR"));
            }
        }
    }

    fn dividend_usd(&mut self, security_id: &str, date: NaiveDate, per_share: Decimal, rate: Decimal) {
        let amount = (self.quantity(security_id) * per_share).round_dp(2);
        if amount <= Decimal::ZERO {
            return;
        }
        let depot = self.world.accounts.usd_depot.id.clone();
        self.push(
            Transaction::dividend(&depot, security_id, date, amount, "USD")
                .with_taxes((amount * dec!(0.15)).round_dp(2))
                .with_fx_rate(rate),
        );
    }

    /// Interest on what the savings account holds, paid monthly at roughly 2.6% a year.
    fn interest(&mut self, anchor: NaiveDate) {
        let amount = (self.savings * dec!(0.00215)).round_dp(2);
        if amount <= Decimal::ZERO {
            return;
        }
        self.savings += amount;
        let savings = self.world.accounts.savings.id.clone();
        self.push(Transaction::cash(
            &savings,
            TransactionKind::Interest,
            anchor,
            amount,
            "EUR",
        ));
    }

    fn buy_eur(&mut self, security_id: &str, date: NaiveDate, index: usize, budget: Decimal) {
        let price = self.world.prices[security_id][index];
        let quantity = self.quantity_for(security_id, budget, price);
        if quantity <= Decimal::ZERO {
            return;
        }
        let depot = self.world.accounts.euro_depot.id.clone();
        self.push(Transaction::buy(&depot, security_id, date, quantity, price, "EUR").with_fees(dec!(1)));
        self.add(security_id, quantity);
    }

    fn buy_usd(&mut self, security_id: &str, date: NaiveDate, index: usize, budget: Decimal, rate: Decimal) {
        let price = self.world.prices[security_id][index];
        let quantity = self.quantity_for(security_id, budget, price);
        if quantity <= Decimal::ZERO {
            return;
        }
        let depot = self.world.accounts.usd_depot.id.clone();
        self.push(
            Transaction::buy(&depot, security_id, date, quantity, price, "USD")
                .with_fees(dec!(1))
                .with_fx_rate(rate),
        );
        self.add(security_id, quantity);
    }

    /// `share` of the position, or all of it: a closed trade is what the trades list is about.
    fn sell_usd(
        &mut self,
        security_id: &str,
        date: NaiveDate,
        index: usize,
        share: Option<Decimal>,
        rate: Decimal,
    ) {
        let held = self.quantity(security_id);
        let quantity = match share {
            Some(part) => (held * part).trunc(),
            None => held,
        };
        if quantity <= Decimal::ZERO {
            return;
        }
        let price = self.world.prices[security_id][index];
        let depot = self.world.accounts.usd_depot.id.clone();
        self.push(
            Transaction::sell(&depot, security_id, date, quantity, price, "USD")
                .with_fees(dec!(1))
                .with_fx_rate(rate),
        );
        self.add(security_id, -quantity);
    }

    /// Euros into the dollar account: two linked legs, so it is money moving inside the
    /// portfolio rather than a withdrawal and a deposit.
    fn exchange(&mut self, date: NaiveDate, amount_eur: Decimal, rate: Decimal) {
        let amount_usd = (amount_eur / rate).round_dp(2);
        let (out, inc) = Transaction::currency_exchange(
            &self.world.accounts.euro_cash.id,
            &self.world.accounts.usd_cash.id,
            date,
            amount_eur,
            "EUR",
            amount_usd,
            "USD",
        );
        self.push(out);
        self.push(inc.with_fx_rate(rate));
    }

    fn transfer_to_savings(&mut self, date: NaiveDate, amount: Decimal) {
        let (out, inc) = Transaction::cash_transfer(
            &self.world.accounts.euro_cash.id,
            &self.world.accounts.savings.id,
            date,
            amount,
            "EUR",
        );
        self.push(out);
        self.push(inc);
        self.savings += amount;
    }

    fn transfer_from_savings(&mut self, date: NaiveDate, amount: Decimal) {
        let (out, inc) = Transaction::cash_transfer(
            &self.world.accounts.savings.id,
            &self.world.accounts.euro_cash.id,
            date,
            amount,
            "EUR",
        );
        self.push(out);
        self.push(inc);
        self.savings -= amount;
    }

    fn quantity_for(&self, security_id: &str, budget: Decimal, price: Decimal) -> Decimal {
        if price <= Decimal::ZERO {
            return Decimal::ZERO;
        }
        let step = self
            .world
            .instruments
            .all()
            .into_iter()
            .find(|s| s.id == security_id)
            .map(|s| s.effective_quantity_step())
            .unwrap_or(Decimal::ONE);
        floor_to_step(budget / price, step)
    }

    fn quantity(&self, security_id: &str) -> Decimal {
        self.held.get(security_id).copied().unwrap_or_default()
    }

    fn add(&mut self, security_id: &str, quantity: Decimal) {
        *self.held.entry(security_id.to_string()).or_default() += quantity;
    }

    fn push(&mut self, tx: Transaction) {
        self.rows.push(tx);
    }
}
