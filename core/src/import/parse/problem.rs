//! What parsing and every later stage of the import has to say about a file or a row.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Whether a problem blocks importing a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    /// Blocks the affected row or file from being imported.
    Error,

    /// Keeps the row importable while surfacing a plausibility concern.
    Warning,
}

/// Stable category for an import problem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProblemCode {
    Encoding,

    MalformedRow,

    MissingColumn,

    NotANumber,

    BadDate,

    MissingValue,

    UnknownKind,

    UnknownAccount,

    WrongAccountKind,

    TransferWithSecurity,

    InvalidTransaction,

    DuplicateInStore,

    DuplicateInFile,

    RestatedInStore,

    SecurityWithoutSource,

    UnknownSecurity,

    DirectionFromSign,

    DirectionConflict,

    AmountSignAmbiguous,

    AmountBasisAmbiguous,

    AmountVsQuantityPrice,

    FeeExceedsAmount,

    FxRateOnBaseCurrency,

    SingleKindValue,

    FutureDate,

    ImplausibleDateSpan,

    ZeroAmount,

    SuspiciousCurrency,

    /// Shares arrived or left with no money named: the row moves a quantity at a price of zero,
    /// so the lot enters the portfolio with no cost basis and shows the whole holding as profit.
    DeliveryWithoutCost,

    /// The row is denominated in a currency the account it lands on does not keep.
    AccountCurrencyMismatch,

    /// The file's ticker names an instrument already stored under a different ISIN: one ticker,
    /// two instruments.
    TickerIsinConflict,

    /// A stored operation of the same day, account, instrument and quantity, differing only in
    /// what it is worth — what editing an imported row by hand leaves behind.
    SimilarInStore,

    /// One instrument's prices in the file step by a whole factor: a split the broker applied
    /// part-way through the statement.
    PossibleSplit,

    /// A sale or outgoing delivery takes more than the stored ledger and this file hold of the
    /// instrument at that date: the purchase or incoming transfer is missing.
    SaleExceedsHoldings,
}

/// File- or row-level diagnostic collected during parsing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportProblem {
    pub row: Option<usize>,

    pub column: Option<String>,
    pub severity: Severity,
    pub code: ProblemCode,
    /// English wording, used as a fallback when the UI has no sentence for this code.
    pub message: String,
    /// Values the UI substitutes into its own wording, so the sentence can be translated.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, String>,
}

impl ImportProblem {
    pub fn file(code: ProblemCode, message: impl Into<String>) -> Self {
        ImportProblem {
            row: None,
            column: None,
            severity: Severity::Error,
            code,
            message: message.into(),
            params: BTreeMap::new(),
        }
    }

    pub fn row(code: ProblemCode, row: usize, message: impl Into<String>) -> Self {
        ImportProblem {
            row: Some(row),
            column: None,
            severity: Severity::Error,
            code,
            message: message.into(),
            params: BTreeMap::new(),
        }
    }

    pub fn cell(
        code: ProblemCode,
        row: usize,
        column: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        ImportProblem {
            row: Some(row),
            column: Some(column.into()),
            severity: Severity::Error,
            code,
            message: message.into(),
            params: BTreeMap::new(),
        }
    }

    /// Adds one value the UI can put into its own sentence for this code.
    pub fn with(mut self, key: &str, value: impl ToString) -> Self {
        self.params.insert(key.to_string(), value.to_string());
        self
    }

    pub fn warn(mut self) -> Self {
        self.severity = Severity::Warning;
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}
