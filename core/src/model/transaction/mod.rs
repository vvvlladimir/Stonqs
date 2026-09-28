//! One ledger row. Direction always comes from the kind, never from a sign on a number.

mod build;
mod kind;

pub use kind::TransactionKind;

use crate::error::{Error, Result};
use crate::money::Currency;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub account_id: String,
    /// `None` for cash-only operations (Deposit, Fee, etc.).
    pub security_id: Option<String>,
    pub kind: TransactionKind,
    pub date: NaiveDate,

    /// `quantity`/`price` are zero for cash-only operations.
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    /// Gross amount before fees/taxes; stored separately from `quantity * price` since brokers sometimes round differently.
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fees: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes: Decimal,

    pub currency: Currency,
    /// Currency of `fees`; `None` = the transaction's own (ADR-0064).
    #[serde(default)]
    pub fee_currency: Option<Currency>,
    /// Currency of `taxes`, read like [`Self::fee_currency`].
    #[serde(default)]
    pub tax_currency: Option<Currency>,
    /// Rate actually applied at the time, fixed on the transaction — never rewritten by later rate moves.
    #[serde(default, with = "rust_decimal::serde::str_option")]
    pub fx_rate_to_base: Option<Decimal>,
    /// Links the two legs of a transfer or currency exchange — two rows, since sides can have different accounts, currencies, and dates.
    #[serde(default)]
    pub link_id: Option<String>,
    /// The broker's id: the same id re-imported is a restatement, not a new row (ADR-0065).
    #[serde(default)]
    pub external_id: Option<String>,
    pub note: Option<String>,
    /// The kind before `calc::scope` rewrote it into a delivery, so turnover still counts the
    /// trade (ADR-0044). Never stored, never on the wire.
    #[serde(skip)]
    pub scoped_from: Option<TransactionKind>,
}

impl Transaction {
    /// Non-negative; direction is [`Self::cash_delta`]'s. A charge in another currency is not in
    /// it — that is its own leg in [`Self::foreign_charge_legs`].
    pub fn gross_in_transaction_currency(&self) -> Decimal {
        let sign = Decimal::from(self.kind.charge_sign());
        let fees = if self.fee_currency.is_none() {
            self.fees
        } else {
            Decimal::ZERO
        };
        let taxes = if self.tax_currency.is_none() {
            self.taxes
        } else {
            Decimal::ZERO
        };
        self.amount + sign * (fees + taxes)
    }

    /// Charges in another currency, as negative cash movements; a delivery has none.
    pub fn foreign_charge_legs(&self) -> Vec<(Currency, Decimal)> {
        if self.kind.charge_sign() == 0 || self.kind.cash_sign() == 0 {
            return Vec::new();
        }
        let mut legs = Vec::new();
        for (currency, amount) in [(&self.fee_currency, self.fees), (&self.tax_currency, self.taxes)] {
            if let Some(currency) = currency
                && !amount.is_zero()
            {
                legs.push((currency.clone(), -amount));
            }
        }
        legs
    }

    pub fn cash_delta(&self) -> Decimal {
        self.gross_in_transaction_currency() * Decimal::from(self.kind.cash_sign())
    }

    pub fn validate(&self) -> Result<()> {
        if self.quantity.is_sign_negative() {
            return Err(Error::Invalid("quantity must not be negative".into()));
        }
        if self.price.is_sign_negative() || self.amount.is_sign_negative() {
            return Err(Error::Invalid("price/amount must not be negative".into()));
        }
        // The last gate before the ledger, whatever wrote the row (the wizard, a plugin's
        // reader, an assistant tool, the form): a figure this large overflows the first product
        // taken of it, and by then it is stored and every screen panics on it.
        if [self.quantity, self.price, self.amount, self.fees, self.taxes]
            .into_iter()
            .chain(self.fx_rate_to_base)
            .any(|v| !crate::money::in_range(v))
        {
            return Err(Error::Invalid(format!(
                "amounts must stay within ±{}",
                crate::money::MAX_MAGNITUDE
            )));
        }
        if self.kind.affects_quantity() {
            if self.security_id.is_none() {
                return Err(Error::Invalid(format!("{:?} requires a security", self.kind)));
            }
            if self.quantity.is_zero() {
                return Err(Error::Invalid(format!(
                    "{:?} requires non-zero quantity",
                    self.kind
                )));
            }
        }
        if let Some(r) = self.fx_rate_to_base
            && r <= Decimal::ZERO
        {
            return Err(Error::Invalid("fx rate must be positive".into()));
        }
        if [&self.fee_currency, &self.tax_currency]
            .into_iter()
            .flatten()
            .any(|c| c.trim().is_empty())
        {
            return Err(Error::Invalid("charge currency must not be empty".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
