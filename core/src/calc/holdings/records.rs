//! What building holdings records along the way, for the reports.

use crate::model::{Lot, TransactionKind};
use crate::money::Currency;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Cash or security movement across the portfolio boundary in base currency.
/// `+` enters and `-` leaves; internal trades are not external flows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CashFlow {
    pub date: NaiveDate,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_base: Decimal,
}

/// One disposal with its realized result, recorded while lots are removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RealizedGain {
    pub date: NaiveDate,
    pub security_id: String,
    /// Disposal cause: sale or outbound delivery.
    pub kind: TransactionKind,
    #[serde(with = "rust_decimal::serde::str")]
    pub quantity: Decimal,
    /// Gross proceeds before fees and taxes, in base currency.
    #[serde(with = "rust_decimal::serde::str")]
    pub proceeds_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fees_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes_base: Decimal,
    /// Historical base-currency cost of removed lots.
    #[serde(with = "rust_decimal::serde::str")]
    pub cost_base: Decimal,
    /// Same cost in the settlement currency, before any FX conversion.
    #[serde(default, with = "rust_decimal::serde::str")]
    pub cost_in_currency: Decimal,
    /// Gain: proceeds - fees - taxes - cost.
    #[serde(with = "rust_decimal::serde::str")]
    pub gain_base: Decimal,
    /// The part of `gain_base` the exchange rate made, not the instrument — see ADR-0028.
    #[serde(default, with = "rust_decimal::serde::str")]
    pub currency_gain_base: Decimal,
    /// Lots consumed, oldest first: a trade's holding period and IRR come from them (ADR-0027).
    #[serde(default)]
    pub lots: Vec<Lot>,
}

impl RealizedGain {
    /// Result against this disposal's own cost basis; see [`crate::calc::return_on_cost`].
    pub fn return_on_cost(&self) -> Option<Decimal> {
        crate::calc::return_on_cost(self.gain_base, self.cost_base)
    }

    /// What the instrument itself earned: the result with the currency move taken out.
    pub fn instrument_gain_base(&self) -> Decimal {
        self.gain_base - self.currency_gain_base
    }

    /// Proceeds after the costs of selling — the exit value of a closed trade.
    pub fn net_proceeds_base(&self) -> Decimal {
        self.proceeds_base - self.fees_base - self.taxes_base
    }
}

/// One income event: dividend, coupon, account interest, or interest charge.
/// Charges are negative so net interest is a direct sum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncomeRecord {
    pub date: NaiveDate,
    pub account_id: String,
    /// `None` means account interest or a dividend without a security ID.
    pub security_id: Option<String>,
    /// Dividend, Interest, InterestCharge, Cashback or Reward.
    pub kind: TransactionKind,
    /// Gross amount before withholding tax.
    #[serde(with = "rust_decimal::serde::str")]
    pub gross_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub fees_base: Decimal,
    /// Net amount credited to the account.
    #[serde(with = "rust_decimal::serde::str")]
    pub net_base: Decimal,
    /// Transaction currency and original broker-reported amount.
    pub currency: Currency,
    #[serde(with = "rust_decimal::serde::str")]
    pub gross_in_currency: Decimal,
}

/// One standalone fee or tax event. Trade costs stay in cost/proceeds;
/// refunds are negative so period totals can be summed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChargeRecord {
    pub date: NaiveDate,
    pub account_id: String,
    /// Account-level charge has no security ID.
    pub security_id: Option<String>,
    /// Fee, FeeRefund, Tax, or TaxRefund.
    pub kind: TransactionKind,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_base: Decimal,
    pub currency: Currency,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount_in_currency: Decimal,
}
