use crate::error::{Error, Result};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// One enum, not a type hierarchy: Rust has no inheritance, and `match` makes the compiler flag unhandled new kinds.
/// `Ord` derive order matters: reports group by declaration order below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionKind {
    Buy,
    Sell,
    /// Shares arrived with no money attached (transfer-in, inheritance, gift); `amount` becomes cost basis.
    DeliveryInbound,
    /// Shares left with no money attached; still realizes P/L at the given value.
    DeliveryOutbound,
    /// Cost basis and purchase date carry over from the paired leg via `link_id`.
    SecurityTransferIn,
    SecurityTransferOut,

    Dividend,
    Interest,
    InterestCharge,
    /// Money handed back for spending: card cashback, rebates, promotions. Income, but it
    /// comes from turnover rather than from holding anything.
    Cashback,
    /// Income for holding: staking, lock-up and loyalty rewards. Paid in whatever the
    /// position is denominated in, which on a crypto account is the asset itself.
    Reward,
    Fee,
    FeeRefund,
    Tax,
    TaxRefund,
    /// External flow for TWR/XIRR.
    Deposit,
    Withdrawal,
    /// Internal at the portfolio level, so it doesn't distort TWR; a paired `TransferOut`/`TransferIn` in different currencies is a currency exchange.
    TransferIn,
    TransferOut,
}

impl TransactionKind {
    /// Every operation the model has, in declaration order — which is the order reports group
    /// by. Used where a list has to be complete rather than merely long.
    pub const ALL: &'static [TransactionKind] = &[
        TransactionKind::Buy,
        TransactionKind::Sell,
        TransactionKind::DeliveryInbound,
        TransactionKind::DeliveryOutbound,
        TransactionKind::SecurityTransferIn,
        TransactionKind::SecurityTransferOut,
        TransactionKind::Dividend,
        TransactionKind::Interest,
        TransactionKind::InterestCharge,
        TransactionKind::Cashback,
        TransactionKind::Reward,
        TransactionKind::Fee,
        TransactionKind::FeeRefund,
        TransactionKind::Tax,
        TransactionKind::TaxRefund,
        TransactionKind::Deposit,
        TransactionKind::Withdrawal,
        TransactionKind::TransferIn,
        TransactionKind::TransferOut,
    ];

    pub fn affects_quantity(self) -> bool {
        self.quantity_sign() != 0
    }

    /// Sign comes from the kind, not from `quantity`'s own sign — so "sell minus five shares" can't happen.
    pub fn quantity_sign(self) -> i8 {
        match self {
            TransactionKind::Buy | TransactionKind::DeliveryInbound | TransactionKind::SecurityTransferIn => {
                1
            }
            TransactionKind::Sell
            | TransactionKind::DeliveryOutbound
            | TransactionKind::SecurityTransferOut => -1,
            _ => 0,
        }
    }

    pub fn requires_security(self) -> bool {
        self.affects_quantity() || self == TransactionKind::Dividend
    }

    /// Breaks the period for TWR. `Delivery*` counts too — shares crossing the portfolio boundary are a flow even without cash.
    pub fn is_external_flow(self) -> bool {
        matches!(
            self,
            TransactionKind::Deposit
                | TransactionKind::Withdrawal
                | TransactionKind::DeliveryInbound
                | TransactionKind::DeliveryOutbound
        )
    }

    /// Same sign as [`Transaction::cash_delta`], known before the transaction exists — used by import's amount-sign check.
    pub fn cash_sign(self) -> i8 {
        match self {
            TransactionKind::Buy
            | TransactionKind::Fee
            | TransactionKind::Tax
            | TransactionKind::InterestCharge
            | TransactionKind::Withdrawal
            | TransactionKind::TransferOut => -1,

            TransactionKind::Sell
            | TransactionKind::Dividend
            | TransactionKind::Interest
            | TransactionKind::FeeRefund
            | TransactionKind::TaxRefund
            | TransactionKind::Cashback
            | TransactionKind::Reward
            | TransactionKind::Deposit
            | TransactionKind::TransferIn => 1,

            TransactionKind::DeliveryInbound
            | TransactionKind::DeliveryOutbound
            | TransactionKind::SecurityTransferIn
            | TransactionKind::SecurityTransferOut => 0,
        }
    }

    /// How charges enter this operation's total: they add to what an acquisition cost and come
    /// off what a disposal or a payment yielded. Zero where the amount is already the whole of
    /// it — a deposit, a standalone fee — so fees recorded there change no total.
    pub fn charge_sign(self) -> i8 {
        match self {
            TransactionKind::Buy | TransactionKind::DeliveryInbound => 1,
            TransactionKind::Sell
            | TransactionKind::DeliveryOutbound
            | TransactionKind::Dividend
            | TransactionKind::Interest => -1,
            _ => 0,
        }
    }

    /// Opposite-direction counterpart, used when import flips a row by amount sign. `Buy`/`Sell` never pair — see CLAUDE.md import invariants.
    pub fn reversed(self) -> Option<TransactionKind> {
        match self {
            TransactionKind::Deposit => Some(TransactionKind::Withdrawal),
            TransactionKind::Withdrawal => Some(TransactionKind::Deposit),
            TransactionKind::TransferIn => Some(TransactionKind::TransferOut),
            TransactionKind::TransferOut => Some(TransactionKind::TransferIn),
            TransactionKind::Fee => Some(TransactionKind::FeeRefund),
            TransactionKind::FeeRefund => Some(TransactionKind::Fee),
            TransactionKind::Tax => Some(TransactionKind::TaxRefund),
            TransactionKind::TaxRefund => Some(TransactionKind::Tax),
            TransactionKind::Interest => Some(TransactionKind::InterestCharge),
            TransactionKind::InterestCharge => Some(TransactionKind::Interest),
            TransactionKind::DeliveryInbound => Some(TransactionKind::DeliveryOutbound),
            TransactionKind::DeliveryOutbound => Some(TransactionKind::DeliveryInbound),
            TransactionKind::SecurityTransferIn => Some(TransactionKind::SecurityTransferOut),
            TransactionKind::SecurityTransferOut => Some(TransactionKind::SecurityTransferIn),
            TransactionKind::Buy
            | TransactionKind::Sell
            | TransactionKind::Dividend
            | TransactionKind::Cashback
            | TransactionKind::Reward => None,
        }
    }

    pub fn is_linked_side(self) -> bool {
        matches!(
            self,
            TransactionKind::TransferIn
                | TransactionKind::TransferOut
                | TransactionKind::SecurityTransferIn
                | TransactionKind::SecurityTransferOut
        )
    }
}

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
    /// Currency of `fees`, `None` when it is the transaction's own — a commission is charged on
    /// the venue and withholding is deducted where the issuer sits, so neither has to follow the
    /// currency the operation settles in.
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
    /// The broker's own identifier for this operation, when the file carried one. It is what
    /// makes a re-import an update rather than a second row: the same id is the same
    /// operation even when its numbers have been restated.
    #[serde(default)]
    pub external_id: Option<String>,
    pub note: Option<String>,
    /// The kind this row had before `calc::scope` rewrote it into a delivery — set only where the
    /// shares are in scope and the cash that paid for them is not. A lens changes the point of
    /// view, not the fact that the portfolio traded, so turnover still counts it and the
    /// commission it carries still has a trade to belong to. Never stored, never on the wire.
    #[serde(skip)]
    pub scoped_from: Option<TransactionKind>,
}

impl Transaction {
    fn blank(account_id: &str, kind: TransactionKind, date: NaiveDate, currency: &str) -> Self {
        Transaction {
            id: super::new_id(),
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
        let link = super::new_id();
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
        let link = super::new_id();
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
        let link = super::new_id();
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

    /// Always non-negative; direction comes from [`Self::cash_delta`]. Fees/taxes add to cost on acquisition, subtract from proceeds on disposal.
    ///
    /// A charge in a currency of its own is **not** in this total: it is money taken from
    /// another balance, and adding it here would sum two currencies into one number. It comes
    /// back as its own leg in [`Self::foreign_charge_legs`].
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

    /// Charges paid in a currency other than the transaction's, as cash movements. A commission
    /// leaves money whichever side of the trade it sits on, so both legs are negative; a
    /// delivery moves no cash at all and therefore has no legs here.
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
        let gross = self.gross_in_transaction_currency();
        match self.kind {
            TransactionKind::Buy
            | TransactionKind::Fee
            | TransactionKind::Tax
            | TransactionKind::InterestCharge
            | TransactionKind::Withdrawal
            | TransactionKind::TransferOut => -gross,

            TransactionKind::Sell
            | TransactionKind::Dividend
            | TransactionKind::Interest
            | TransactionKind::FeeRefund
            | TransactionKind::TaxRefund
            | TransactionKind::Cashback
            | TransactionKind::Reward
            | TransactionKind::Deposit
            | TransactionKind::TransferIn => gross,

            // Security delivery moves no money — that's the point of it.
            TransactionKind::DeliveryInbound
            | TransactionKind::DeliveryOutbound
            | TransactionKind::SecurityTransferIn
            | TransactionKind::SecurityTransferOut => Decimal::ZERO,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.quantity.is_sign_negative() {
            return Err(Error::Invalid("quantity must not be negative".into()));
        }
        if self.price.is_sign_negative() || self.amount.is_sign_negative() {
            return Err(Error::Invalid("price/amount must not be negative".into()));
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

/// A charge currency is only recorded when it differs from the transaction's, so one row has
/// exactly one spelling of "the same currency".
fn charge_currency(transaction: &Currency, charge: &str) -> Option<Currency> {
    let charge = normalize_currency(charge);
    (charge != *transaction).then_some(charge)
}

impl TransactionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TransactionKind::Buy => "BUY",
            TransactionKind::Sell => "SELL",
            TransactionKind::DeliveryInbound => "DELIVERY_INBOUND",
            TransactionKind::DeliveryOutbound => "DELIVERY_OUTBOUND",
            TransactionKind::SecurityTransferIn => "SECURITY_TRANSFER_IN",
            TransactionKind::SecurityTransferOut => "SECURITY_TRANSFER_OUT",
            TransactionKind::Dividend => "DIVIDEND",
            TransactionKind::Interest => "INTEREST",
            TransactionKind::InterestCharge => "INTEREST_CHARGE",
            TransactionKind::Cashback => "CASHBACK",
            TransactionKind::Reward => "REWARD",
            TransactionKind::FeeRefund => "FEE_REFUND",
            TransactionKind::TaxRefund => "TAX_REFUND",
            TransactionKind::Fee => "FEE",
            TransactionKind::Tax => "TAX",
            TransactionKind::Deposit => "DEPOSIT",
            TransactionKind::Withdrawal => "WITHDRAWAL",
            TransactionKind::TransferIn => "TRANSFER_IN",
            TransactionKind::TransferOut => "TRANSFER_OUT",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "BUY" => TransactionKind::Buy,
            "SELL" => TransactionKind::Sell,
            "DELIVERY_INBOUND" => TransactionKind::DeliveryInbound,
            "DELIVERY_OUTBOUND" => TransactionKind::DeliveryOutbound,
            "SECURITY_TRANSFER_IN" => TransactionKind::SecurityTransferIn,
            "SECURITY_TRANSFER_OUT" => TransactionKind::SecurityTransferOut,
            "DIVIDEND" => TransactionKind::Dividend,
            "INTEREST" => TransactionKind::Interest,
            "INTEREST_CHARGE" => TransactionKind::InterestCharge,
            "CASHBACK" => TransactionKind::Cashback,
            "REWARD" => TransactionKind::Reward,
            "FEE_REFUND" => TransactionKind::FeeRefund,
            "TAX_REFUND" => TransactionKind::TaxRefund,
            "FEE" => TransactionKind::Fee,
            "TAX" => TransactionKind::Tax,
            "DEPOSIT" => TransactionKind::Deposit,
            "WITHDRAWAL" => TransactionKind::Withdrawal,
            "TRANSFER_IN" => TransactionKind::TransferIn,
            "TRANSFER_OUT" => TransactionKind::TransferOut,
            other => return Err(Error::Invalid(format!("unknown transaction kind {other:?}"))),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// Manual list, but safe: `cash_sign`/`reversed` match without `_`, so a new kind breaks the build first.
    const ALL: &[TransactionKind] = &[
        TransactionKind::Buy,
        TransactionKind::Sell,
        TransactionKind::DeliveryInbound,
        TransactionKind::DeliveryOutbound,
        TransactionKind::SecurityTransferIn,
        TransactionKind::SecurityTransferOut,
        TransactionKind::Dividend,
        TransactionKind::Interest,
        TransactionKind::InterestCharge,
        TransactionKind::Fee,
        TransactionKind::FeeRefund,
        TransactionKind::Tax,
        TransactionKind::TaxRefund,
        TransactionKind::Deposit,
        TransactionKind::Withdrawal,
        TransactionKind::TransferIn,
        TransactionKind::TransferOut,
    ];

    #[test]
    fn cash_sign_matches_cash_delta() {
        for &kind in ALL {
            let t = Transaction::cash("acc", kind, "2024-01-02".parse().unwrap(), dec!(100), "EUR");
            let delta = t.cash_delta();
            let expected = match kind.cash_sign() {
                1 => dec!(100),
                -1 => dec!(-100),
                _ => Decimal::ZERO,
            };
            assert_eq!(delta, expected, "{kind:?}");
        }
    }

    #[test]
    fn reversed_is_an_involution_with_the_opposite_sign() {
        for &kind in ALL {
            let Some(back) = kind.reversed() else {
                assert!(matches!(
                    kind,
                    TransactionKind::Buy | TransactionKind::Sell | TransactionKind::Dividend
                ));
                continue;
            };
            assert_eq!(back.reversed(), Some(kind), "{kind:?}");
            if kind.cash_sign() != 0 {
                assert_eq!(back.cash_sign(), -kind.cash_sign(), "{kind:?}");
            }
        }
    }
}
