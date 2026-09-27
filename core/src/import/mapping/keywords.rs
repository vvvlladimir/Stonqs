//! Operation wordings per kind. Canonical-first, and sell wordings precede buy ones ("Verkoop"
//! contains "Koop").

use super::normalize_alias;
use crate::model::TransactionKind;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A plugin's wordings, read only *after* the shipped keywords, so they fill gaps and never
/// re-answer a wording the app reads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "KindWordsFile", into = "KindWordsFile")]
pub struct KindWords(Vec<(String, TransactionKind)>);

/// The file shape: an ordered list, because order decides which of two nested words wins.
#[derive(Serialize, Deserialize)]
struct KindWordsFile {
    words: Vec<KindWord>,
}

#[derive(Serialize, Deserialize)]
struct KindWord {
    word: String,
    kind: TransactionKind,
}

impl From<KindWordsFile> for KindWords {
    fn from(file: KindWordsFile) -> Self {
        KindWords::new(file.words.into_iter().map(|w| (w.word, w.kind)))
    }
}

impl From<KindWords> for KindWordsFile {
    fn from(words: KindWords) -> Self {
        KindWordsFile {
            words: words
                .0
                .into_iter()
                .map(|(word, kind)| KindWord { word, kind })
                .collect(),
        }
    }
}

impl KindWords {
    pub const fn empty() -> Self {
        KindWords(Vec::new())
    }

    /// Normalised on the way in, the way every wording is before it is compared. A word that
    /// normalises to nothing would match every wording, so it is dropped.
    pub fn new<S: AsRef<str>>(words: impl IntoIterator<Item = (S, TransactionKind)>) -> Self {
        KindWords(
            words
                .into_iter()
                .map(|(word, kind)| (normalize_alias(word.as_ref()), kind))
                .filter(|(word, _)| !word.is_empty())
                .collect(),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Several dictionaries read as one, in the order given.
    pub fn extend(&mut self, other: KindWords) {
        self.0.extend(other.0);
    }

    pub(crate) fn kind_of(&self, normalized: &str) -> Option<TransactionKind> {
        self.0
            .iter()
            .filter(|(word, _)| long_enough(word))
            .find(|(word, _)| normalized.contains(word.as_str()))
            .map(|(_, kind)| *kind)
    }

    /// Words too short to be read: matched by containment, "E" or "TO" would answer nearly every
    /// wording nobody else did. Refused at install, and skipped by `kind_of` all the same.
    pub fn too_short(&self) -> Vec<&str> {
        self.0
            .iter()
            .filter(|(word, _)| !long_enough(word))
            .map(|(word, _)| word.as_str())
            .collect()
    }

    /// Words the shipped keywords already read as another kind: never reached, so refused at install.
    pub fn shadowed(&self) -> Vec<(&str, TransactionKind)> {
        self.0
            .iter()
            .filter_map(|(word, kind)| match kind_from_keywords(word) {
                Some(shipped) if shipped != *kind => Some((word.as_str(), shipped)),
                _ => None,
            })
            .collect()
    }
}

/// Three characters of an alphabet; two of a script whose one character is a syllable or a word,
/// where "买入" (buy) is already the whole word.
fn long_enough(word: &str) -> bool {
    let count = word.chars().count();
    count >= 3 || (count == 2 && word.chars().all(|c| c as u32 >= 0x2E80))
}

pub fn default_kind_aliases() -> BTreeMap<String, TransactionKind> {
    use TransactionKind::*;
    let table: &[(&str, TransactionKind)] = &[
        ("BUY", Buy),
        ("PURCHASE", Buy),
        ("KAUF", Buy),
        ("KAUFEN", Buy),
        ("ПОКУПКА", Buy),
        ("SELL", Sell),
        ("SALE", Sell),
        ("VERKAUF", Sell),
        ("ПРОДАЖА", Sell),
        ("DIVIDEND", Dividend),
        ("DIVIDENDE", Dividend),
        ("DIV", Dividend),
        ("DISTRIBUTION", Dividend),
        ("ДИВИДЕНДЫ", Dividend),
        ("INTEREST", Interest),
        ("ZINSEN", Interest),
        ("COUPON", Interest),
        ("КУПОН", Interest),
        ("ПРОЦЕНТЫ", Interest),
        ("INTERESTCHARGE", InterestCharge),
        ("SOLLZINSEN", InterestCharge),
        ("FEE", Fee),
        ("COMMISSION", Fee),
        ("GEBÜHR", Fee),
        ("GEBUEHR", Fee),
        ("КОМИССИЯ", Fee),
        ("FEEREFUND", FeeRefund),
        ("GEBÜHRENERSTATTUNG", FeeRefund),
        ("TAX", Tax),
        ("WITHHOLDING", Tax),
        ("STEUER", Tax),
        ("НАЛОГ", Tax),
        ("TAXREFUND", TaxRefund),
        ("STEUERERSTATTUNG", TaxRefund),
        ("DEPOSIT", Deposit),
        ("CONTRIBUTION", Deposit),
        ("EINZAHLUNG", Deposit),
        ("ПОПОЛНЕНИЕ", Deposit),
        ("WITHDRAWAL", Withdrawal),
        ("AUSZAHLUNG", Withdrawal),
        ("ВЫВОД", Withdrawal),
        ("TRANSFERIN", TransferIn),
        ("TRANSFEROUT", TransferOut),
        ("TRANSFERINBOUND", TransferIn),
        ("TRANSFERINSTANTINBOUND", TransferIn),
        ("TRANSFEROUTBOUND", TransferOut),
        ("TRANSFERINSTANTOUTBOUND", TransferOut),
        ("INTERESTPAYMENT", Interest),
        ("DIVIDENDPAYMENT", Dividend),
        ("DELIVERYINBOUND", DeliveryInbound),
        ("EINLIEFERUNG", DeliveryInbound),
        ("DELIVERYOUTBOUND", DeliveryOutbound),
        ("AUSLIEFERUNG", DeliveryOutbound),
        ("SECURITYTRANSFERIN", SecurityTransferIn),
        ("SECURITYTRANSFEROUT", SecurityTransferOut),
    ];
    table.iter().map(|(k, v)| (normalize_alias(k), *v)).collect()
}

/// Operation wording by keyword; the first hit wins, so order matters.
pub(crate) fn kind_from_keywords(normalized: &str) -> Option<TransactionKind> {
    use TransactionKind::*;
    const KEYWORDS: &[(&str, TransactionKind)] = &[
        ("WITHHOLDING", Tax),
        ("STAMPDUTY", Tax),
        ("SDRT", Tax),
        ("TAX", Tax),
        ("STEUER", Tax),
        ("BELASTING", Tax),
        ("IMPOSTA", Tax),
        ("IMPUESTO", Tax),
        ("PODATEK", Tax),
        ("KÄLLSKATT", Tax),
        ("RITENUTA", Tax),
        ("НАЛОГ", Tax),
        ("FEE", Fee),
        ("COMMISSION", Fee),
        ("COURTAGE", Fee),
        ("GEBÜHR", Fee),
        ("GEBUEHR", Fee),
        ("KOSTEN", Fee),
        ("PROVISION", Fee),
        ("FRAIS", Fee),
        ("CHARGE", Fee),
        ("PROWIZJA", Fee),
        ("TARIEVEN", Fee),
        ("КОМИССИЯ", Fee),
        ("CASHBACK", Cashback),
        ("REBATE", Cashback),
        ("REFERRAL", Cashback),
        ("КЕШБЭК", Cashback),
        ("КЭШБЭК", Cashback),
        ("REWARD", Reward),
        ("STAKINGINCOME", Reward),
        ("AIRDROP", Reward),
        ("НАГРАД", Reward),
        ("ВОЗНАГРАЖД", Reward),
        ("INTEREST", Interest),
        ("INTERESSI", Interest),
        ("RÄNTA", Interest),
        ("COUPON", Interest),
        ("CEDOLA", Interest),
        ("INTÉR", Interest),
        ("ZINSEN", Interest),
        ("RENTE", Interest),
        ("ПРОЦЕНТ", Interest),
        ("DIVIDEND", Dividend),
        ("DIVIDENDO", Dividend),
        ("UTDELNING", Dividend),
        ("PROVENTO", Dividend),
        ("ДИВИДЕНД", Dividend),
        ("VERKAUF", Sell),
        ("VERKOOP", Sell),
        ("SELL", Sell),
        ("SALE", Sell),
        ("VEND", Sell),
        ("VENTE", Sell),
        ("SPRZEDA", Sell),
        ("SÄLJ", Sell),
        ("ПРОДАЖА", Sell),
        ("AANKOOP", Buy),
        ("KOOP", Buy),
        ("BUY", Buy),
        ("PURCHASE", Buy),
        ("KAUF", Buy),
        ("ACHAT", Buy),
        ("COMPRA", Buy),
        ("ACQUIST", Buy),
        ("ZAKUP", Buy),
        ("KÖP", Buy),
        ("ПОКУПКА", Buy),
        ("WITHDRAW", Withdrawal),
        ("AUSZAHLUNG", Withdrawal),
        ("ONTTREKKING", Withdrawal),
        ("OPNAME", Withdrawal),
        ("UTTAG", Withdrawal),
        ("RETIRADA", Withdrawal),
        ("RETRAIT", Withdrawal),
        ("PRELIEVO", Withdrawal),
        ("WYPLATA", Withdrawal),
        ("СНЯТИЕ", Withdrawal),
        ("DEPOSIT", Deposit),
        ("TOPUP", Deposit),
        ("EINZAHLUNG", Deposit),
        ("STORTING", Deposit),
        ("INSÄTTNING", Deposit),
        ("VERSEMENT", Deposit),
        ("INGRESO", Deposit),
        ("WPLATA", Deposit),
        ("COTISATION", Deposit),
        ("SUBVENTION", Deposit),
        ("ПОПОЛНЕНИЕ", Deposit),
        // Value moving inside the portfolio: one keyword for both legs, direction from the sign.
        (">", TransferIn),
        ("CONVERSION", TransferIn),
        ("CONVERT", TransferIn),
        ("КОНВЕРТ", TransferIn),
        ("EXCHANGE", TransferIn),
        ("FOREX", TransferIn),
        ("SWAP", TransferIn),
        ("STAK", TransferIn),
        ("RECEIVE", TransferIn),
        ("SEND", TransferOut),
        ("LOCKUP", TransferIn),
        ("TRANSFER", TransferIn),
        ("ÜBERTRAG", TransferIn),
        ("UEBERTRAG", TransferIn),
        ("OVERBOEKING", TransferIn),
        ("TRASFERIMENTO", TransferIn),
        ("TRANSFERENCIA", TransferIn),
        ("ПЕРЕВОД", TransferIn),
    ];
    KEYWORDS
        .iter()
        .find(|(word, _)| normalized.contains(word))
        .map(|(_, kind)| *kind)
}
