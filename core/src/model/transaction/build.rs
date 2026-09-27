//! Constructors and builder methods.

use super::{Transaction, TransactionKind};
use crate::model::new_id;
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;

impl Transaction {
    fn blank(account_id: &str, kind: TransactionKind, date: NaiveDate, currency: &str) -> Self {
        Transaction {
            id: new_id(),
            account_id: account_id.to_string(),
            security_id: None,
            kind,
            date,
            quantity: Decimal::ZERO,
            price: Decimal::ZERO,
            amount: Decimal::ZERO,
            fees: Decimal::ZERO,
            taxes: Decimal::ZERO,
            currency: normalize_currency(currency),
            fee_currency: None,
            tax_currency: None,
            fx_rate_to_base: None,
            link_id: None,
            external_id: None,
            note: None,
            scoped_from: None,
        }
    }

    pub fn buy(
        account_id: &str,
        security_id: &str,
        date: NaiveDate,
        quantity: Decimal,
        price: Decimal,
        currency: &str,
    ) -> Self {
        let mut t = Self::blank(account_id, TransactionKind::Buy, date, currency);
        t.security_id = Some(security_id.to_string());
        t.quantity = quantity;
        t.price = price;
        t.amount = quantity * price;
        t
    }

    /// `quantity` is always positive; direction comes from `kind`.
    pub fn sell(
        account_id: &str,
        security_id: &str,
        date: NaiveDate,
        quantity: Decimal,
        price: Decimal,
        currency: &str,
    ) -> Self {
        let mut t = Self::blank(account_id, TransactionKind::Sell, date, currency);
        t.security_id = Some(security_id.to_string());
        t.quantity = quantity;
        t.price = price;
        t.amount = quantity * price;
        t
    }

    pub fn dividend(
        account_id: &str,
        security_id: &str,
        date: NaiveDate,
        amount: Decimal,
        currency: &str,
    ) -> Self {
        let mut t = Self::blank(account_id, TransactionKind::Dividend, date, currency);
        t.security_id = Some(security_id.to_string());
        t.amount = amount;
        t
    }

    pub fn cash(
        account_id: &str,
        kind: TransactionKind,
        date: NaiveDate,
        amount: Decimal,
        currency: &str,
    ) -> Self {
        let mut t = Self::blank(account_id, kind, date, currency);
        t.amount = amount;
        t
    }

    /// `value` at the arrival date becomes the cost basis.
    pub fn delivery_inbound(
        account_id: &str,
        security_id: &str,
        date: NaiveDate,
        quantity: Decimal,
        value: Decimal,
        currency: &str,
    ) -> Self {
        let mut t = Self::blank(account_id, TransactionKind::DeliveryInbound, date, currency);
        t.security_id = Some(security_id.to_string());
        t.quantity = quantity;
        t.amount = value;
        t.price = if quantity.is_zero() {
            Decimal::ZERO
        } else {
            value / quantity
        };
        t
    }

    pub fn delivery_outbound(
        account_id: &str,
        security_id: &str,
        date: NaiveDate,
        quantity: Decimal,
        value: Decimal,
        currency: &str,
    ) -> Self {
        let mut t = Self::delivery_inbound(account_id, security_id, date, quantity, value, currency);
        t.kind = TransactionKind::DeliveryOutbound;
        t
    }

    /// Returns both linked legs — a one-sided transfer is always a data error, so the constructor can't produce one.
    pub fn cash_transfer(
        from_account: &str,
        to_account: &str,
        date: NaiveDate,
        amount: Decimal,
        currency: &str,
    ) -> (Self, Self) {
        let link = new_id();
        let mut out = Self::blank(from_account, TransactionKind::TransferOut, date, currency);
        out.amount = amount;
        out.link_id = Some(link.clone());
        let mut inc = Self::blank(to_account, TransactionKind::TransferIn, date, currency);
        inc.amount = amount;
        inc.link_id = Some(link);
        (out, inc)
    }

    /// Same linked-transfer shape with different currencies on each leg; rate is derived (`amount_in / amount_out`), never stored.
    pub fn currency_exchange(
        from_account: &str,
        to_account: &str,
        date: NaiveDate,
        amount_out: Decimal,
        currency_out: &str,
        amount_in: Decimal,
        currency_in: &str,
    ) -> (Self, Self) {
        let link = new_id();
        let mut out = Self::blank(from_account, TransactionKind::TransferOut, date, currency_out);
        out.amount = amount_out;
        out.link_id = Some(link.clone());
        let mut inc = Self::blank(to_account, TransactionKind::TransferIn, date, currency_in);
        inc.amount = amount_in;
        inc.link_id = Some(link);
        (out, inc)
    }

    /// Cost basis and purchase dates carry over via `link_id`, handled by `calc::build_holdings`.
    pub fn security_transfer(
        from_account: &str,
        to_account: &str,
        security_id: &str,
        date: NaiveDate,
        quantity: Decimal,
        currency: &str,
    ) -> (Self, Self) {
        let link = new_id();
        let mut out = Self::blank(from_account, TransactionKind::SecurityTransferOut, date, currency);
        out.security_id = Some(security_id.to_string());
        out.quantity = quantity;
        out.link_id = Some(link.clone());
        let mut inc = Self::blank(to_account, TransactionKind::SecurityTransferIn, date, currency);
        inc.security_id = Some(security_id.to_string());
        inc.quantity = quantity;
        inc.link_id = Some(link);
        (out, inc)
    }

    /// `self` is the outgoing leg, `counterpart` the incoming one; `None` if currencies match or `self.amount` is zero.
    pub fn implied_exchange_rate(&self, counterpart: &Transaction) -> Option<Decimal> {
        if self.currency == counterpart.currency || self.amount.is_zero() {
            return None;
        }
        Some(counterpart.amount / self.amount)
    }

    pub fn with_link(mut self, link_id: impl Into<String>) -> Self {
        self.link_id = Some(link_id.into());
        self
    }

    pub fn with_fees(mut self, fees: Decimal) -> Self {
        self.fees = fees;
        self
    }

    pub fn with_taxes(mut self, taxes: Decimal) -> Self {
        self.taxes = taxes;
        self
    }

    /// A commission charged in a currency of its own.
    pub fn with_fees_in(mut self, fees: Decimal, currency: &str) -> Self {
        self.fees = fees;
        self.fee_currency = charge_currency(&self.currency, currency);
        self
    }

    /// Tax withheld in a currency of its own.
    pub fn with_taxes_in(mut self, taxes: Decimal, currency: &str) -> Self {
        self.taxes = taxes;
        self.tax_currency = charge_currency(&self.currency, currency);
        self
    }

    /// The currency `fees` are expressed in, resolved.
    pub fn fees_in(&self) -> &Currency {
        self.fee_currency.as_ref().unwrap_or(&self.currency)
    }

    /// The currency `taxes` are expressed in, resolved.
    pub fn taxes_in(&self) -> &Currency {
        self.tax_currency.as_ref().unwrap_or(&self.currency)
    }

    pub fn with_fx_rate(mut self, rate: Decimal) -> Self {
        self.fx_rate_to_base = Some(rate);
        self
    }

    /// Derives the rate from "this much base currency was actually charged" — easier to enter from a broker statement than a six-decimal rate.
    pub fn with_fx_from_base_total(mut self, base_total: Decimal) -> Self {
        let gross = self.gross_in_transaction_currency();
        if !gross.is_zero() {
            self.fx_rate_to_base = Some(base_total / gross);
        }
        self
    }

    pub fn with_external_id(mut self, id: impl Into<String>) -> Self {
        self.external_id = Some(id.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// Recorded only when it differs from the transaction's, so one row has one spelling of a currency.
fn charge_currency(transaction: &Currency, charge: &str) -> Option<Currency> {
    let charge = normalize_currency(charge);
    (charge != *transaction).then_some(charge)
}
