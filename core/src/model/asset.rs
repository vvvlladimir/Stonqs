use crate::error::{Error, Result};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Which way the amount of an [`Asset`] points in net worth. Both sides are stored positive
/// (ADR-0092), so this is what tells a debt from a thing owned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssetSide {
    Owned,
    Owed,
}

/// What an [`Asset`] is. Only things with no price series live here: something with a quote is a
/// `Security`, and something the owner records operations against is an `Account` (ADR-0092).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AssetKind {
    /// A home, a rental, land.
    Property,
    Vehicle,
    /// Art, wine, a watch — anything valued by opinion.
    Collectible,
    /// Money held somewhere the app has no ledger for, restated as one figure.
    Cash,
    /// A share in a company that is not listed.
    Private,
    /// Money lent out and expected back.
    Receivable,
    Other,

    Mortgage,
    Loan,
    CreditCard,
    /// A drawn credit line or overdraft, including a HELOC.
    CreditLine,
    /// Tax assessed and not yet paid.
    TaxDue,
}

impl AssetKind {
    pub fn side(self) -> AssetSide {
        match self {
            AssetKind::Property
            | AssetKind::Vehicle
            | AssetKind::Collectible
            | AssetKind::Cash
            | AssetKind::Private
            | AssetKind::Receivable
            | AssetKind::Other => AssetSide::Owned,
            AssetKind::Mortgage
            | AssetKind::Loan
            | AssetKind::CreditCard
            | AssetKind::CreditLine
            | AssetKind::TaxDue => AssetSide::Owed,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            AssetKind::Property => "PROPERTY",
            AssetKind::Vehicle => "VEHICLE",
            AssetKind::Collectible => "COLLECTIBLE",
            AssetKind::Cash => "CASH",
            AssetKind::Private => "PRIVATE",
            AssetKind::Receivable => "RECEIVABLE",
            AssetKind::Other => "OTHER",
            AssetKind::Mortgage => "MORTGAGE",
            AssetKind::Loan => "LOAN",
            AssetKind::CreditCard => "CREDIT_CARD",
            AssetKind::CreditLine => "CREDIT_LINE",
            AssetKind::TaxDue => "TAX_DUE",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        Ok(match s {
            "PROPERTY" => AssetKind::Property,
            "VEHICLE" => AssetKind::Vehicle,
            "COLLECTIBLE" => AssetKind::Collectible,
            "CASH" => AssetKind::Cash,
            "PRIVATE" => AssetKind::Private,
            "RECEIVABLE" => AssetKind::Receivable,
            "OTHER" => AssetKind::Other,
            "MORTGAGE" => AssetKind::Mortgage,
            "LOAN" => AssetKind::Loan,
            "CREDIT_CARD" => AssetKind::CreditCard,
            "CREDIT_LINE" => AssetKind::CreditLine,
            "TAX_DUE" => AssetKind::TaxDue,
            other => return Err(Error::Invalid(format!("unknown asset kind {other:?}"))),
        })
    }

    /// Every kind, owned first, in the order a form offers them.
    pub const ALL: &'static [AssetKind] = &[
        AssetKind::Property,
        AssetKind::Vehicle,
        AssetKind::Collectible,
        AssetKind::Cash,
        AssetKind::Private,
        AssetKind::Receivable,
        AssetKind::Other,
        AssetKind::Mortgage,
        AssetKind::Loan,
        AssetKind::CreditCard,
        AssetKind::CreditLine,
        AssetKind::TaxDue,
    ];
}

/// What a debt costs and when it ends. Read forwards only: the balance itself is whatever the
/// owner last valued it at, never a figure this schedule produced (ADR-0092).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Amortization {
    /// Yearly nominal rate as a fraction; `0.0345` is 3.45%.
    #[serde(with = "rust_decimal::serde::str")]
    pub rate: Decimal,
    /// What is paid every month, in the asset's currency.
    #[serde(with = "rust_decimal::serde::str")]
    pub monthly_payment: Decimal,
    /// The day the debt is meant to be gone; `None` leaves the end to be worked out.
    pub ends_on: Option<NaiveDate>,
}

/// Something owned or owed that has no market to price it: a house, a car, a mortgage. It carries
/// a series of [`AssetValue`] rows and enters net worth only — never a return (ADR-0092).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub kind: AssetKind,
    /// The currency every valuation of it is written in.
    pub currency: Currency,
    /// The liability this asset secures, or the asset a liability is secured by. Shows a
    /// relationship and moves no figure of its own.
    #[serde(default)]
    pub secured_by: Option<String>,
    /// Only a liability has one; it answers when the debt ends, not what it is worth today.
    #[serde(default)]
    pub schedule: Option<Amortization>,
    #[serde(default)]
    pub note: Option<String>,
    /// Sold, repaid or written off. From this day on it is absent from net worth, and its
    /// valuations stay where they are.
    #[serde(default)]
    pub closed_on: Option<NaiveDate>,
}

impl Asset {
    pub fn new(name: impl Into<String>, kind: AssetKind, currency: &str) -> Self {
        Asset {
            id: super::new_id(),
            name: name.into(),
            kind,
            currency: normalize_currency(currency),
            secured_by: None,
            schedule: None,
            note: None,
            closed_on: None,
        }
    }

    pub fn side(&self) -> AssetSide {
        self.kind.side()
    }

    /// Lives on the model, not the host, so the CLI and the commands share one rule.
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(Error::Invalid("an asset needs a name".into()));
        }
        if self.currency.trim().is_empty() {
            return Err(Error::Invalid(format!("asset {:?} needs a currency", self.name)));
        }
        if self.secured_by.as_deref() == Some(self.id.as_str()) {
            return Err(Error::Invalid(format!("asset {:?} secures itself", self.name)));
        }
        if self.schedule.is_some() && self.side() == AssetSide::Owned {
            return Err(Error::Invalid(format!(
                "asset {:?} is not a debt, so it has no payment schedule",
                self.name
            )));
        }
        if let Some(schedule) = &self.schedule
            && (schedule.rate < Decimal::ZERO || schedule.monthly_payment < Decimal::ZERO)
        {
            return Err(Error::Invalid(format!(
                "the schedule of {:?} has a negative rate or payment",
                self.name
            )));
        }
        Ok(())
    }
}

/// What an asset was worth on one day, in the asset's currency, because somebody said so. Always
/// positive: a debt of 200 000 is `200000`, and the side of the kind does the subtracting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetValue {
    pub asset_id: String,
    pub date: NaiveDate,
    #[serde(with = "rust_decimal::serde::str")]
    pub amount: Decimal,
    #[serde(default)]
    pub note: Option<String>,
}

impl AssetValue {
    pub fn new(asset_id: &str, date: NaiveDate, amount: Decimal) -> Self {
        AssetValue {
            asset_id: asset_id.to_string(),
            date,
            amount,
            note: None,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.amount < Decimal::ZERO {
            return Err(Error::Invalid(
                "a valuation is what a thing is worth or what is owed, so it is never negative".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn every_kind_has_a_side_and_survives_a_round_trip() {
        for kind in AssetKind::ALL {
            assert_eq!(AssetKind::parse(kind.as_str()).unwrap(), *kind);
        }
        assert_eq!(AssetKind::Property.side(), AssetSide::Owned);
        assert_eq!(AssetKind::Mortgage.side(), AssetSide::Owed);
    }

    /// A schedule answers when a debt ends; on a house it would answer nothing at all.
    #[test]
    fn only_a_debt_carries_a_schedule() {
        let mut house = Asset::new("Flat", AssetKind::Property, "eur");
        assert_eq!(house.currency, "EUR");
        house.schedule = Some(Amortization {
            rate: dec!(0.03),
            monthly_payment: dec!(900),
            ends_on: None,
        });
        assert!(house.validate().is_err());

        let mut mortgage = Asset::new("Mortgage", AssetKind::Mortgage, "EUR");
        mortgage.schedule = house.schedule.clone();
        assert!(mortgage.validate().is_ok());
    }

    #[test]
    fn an_asset_cannot_secure_itself() {
        let mut house = Asset::new("Flat", AssetKind::Property, "EUR");
        house.secured_by = Some(house.id.clone());
        assert!(house.validate().is_err());
    }

    /// The form asks "what is still owed", so a minus sign here is a typo, not a direction.
    #[test]
    fn a_valuation_is_never_negative() {
        let day = NaiveDate::from_ymd_opt(2026, 1, 31).unwrap();
        assert!(AssetValue::new("a", day, dec!(-1)).validate().is_err());
        assert!(AssetValue::new("a", day, dec!(0)).validate().is_ok());
    }
}
