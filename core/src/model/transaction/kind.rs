//! What an operation is, and every sign that follows from that alone.

use crate::error::{Error, Result};
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
    /// Money handed back for spending: cashback, rebates, promotions.
    Cashback,
    /// Income for holding: staking, lock-up and loyalty rewards, often paid in the asset itself.
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
    /// Every kind in declaration order (the order reports group by); `parse` reads through it.
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

    /// Charges add to an acquisition's cost and come off a disposal's or a payment's yield; zero
    /// where the amount is already the whole of it.
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

    /// Opposite-direction counterpart, used when import flips a row by amount sign. `Buy`/`Sell` never pair — see `.claude/rules/import.md`.
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
        Self::ALL
            .iter()
            .copied()
            .find(|kind| kind.as_str() == s)
            .ok_or_else(|| Error::Invalid(format!("unknown transaction kind {s:?}")))
    }
}
