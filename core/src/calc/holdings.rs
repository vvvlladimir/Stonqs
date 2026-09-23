use crate::error::{Error, Result};
use crate::fx::RateLookup;
use crate::model::{CorporateAction, CostBasisMethod, Lot, Position, Transaction, TransactionKind};
use crate::money::{Currency, normalize_currency};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

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
    /// Purchase lots this disposal consumed, oldest first. A trade is built from these:
    /// without their dates there is no holding period and no IRR — see ADR-0027.
    #[serde(default)]
    pub lots: Vec<Lot>,
}

impl RealizedGain {
    /// Result against this disposal's own cost basis; see [`super::return_on_cost`].
    pub fn return_on_cost(&self) -> Option<Decimal> {
        super::return_on_cost(self.gain_base, self.cost_base)
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

/// Position-building options kept together and cheap to copy.
#[derive(Debug, Clone, Copy, Default)]
pub struct HoldingsOptions<'a> {
    pub cost_basis: CostBasisMethod,
    /// Splits and other security events; an empty slice means none.
    pub corporate_actions: &'a [CorporateAction],
}

impl<'a> HoldingsOptions<'a> {
    pub fn with_cost_basis(mut self, method: CostBasisMethod) -> Self {
        self.cost_basis = method;
        self
    }

    pub fn with_corporate_actions(mut self, actions: &'a [CorporateAction]) -> Self {
        self.corporate_actions = actions;
        self
    }
}

/// Portfolio state after applying transactions through a date; it contains no market prices.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Holdings {
    /// Positions keyed by security ID; `BTreeMap` keeps output deterministic.
    pub positions: BTreeMap<String, Position>,
    /// Cash by currency; simultaneous positive and negative balances are valid.
    pub cash: BTreeMap<Currency, Decimal>,
    /// Chronological external flows in base currency.
    pub external_flows: Vec<CashFlow>,
    /// Disposals with results for realized-gain reports.
    #[serde(default)]
    pub realized: Vec<RealizedGain>,
    /// Income events for dividend and interest reports.
    #[serde(default)]
    pub income: Vec<IncomeRecord>,
    /// Standalone fees and taxes for expense reports.
    #[serde(default)]
    pub charges: Vec<ChargeRecord>,
    #[serde(with = "rust_decimal::serde::str")]
    pub realized_pnl_base: Decimal,
    /// The share of `realized_pnl_base` that came from the exchange rate.
    #[serde(default, with = "rust_decimal::serde::str")]
    pub realized_currency_gain_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub dividends_base: Decimal,
    /// Net interest: received minus paid.
    #[serde(with = "rust_decimal::serde::str")]
    pub interest_base: Decimal,
    /// Standalone fees; trade fees are already in cost or proceeds.
    #[serde(with = "rust_decimal::serde::str")]
    pub fees_base: Decimal,
    #[serde(with = "rust_decimal::serde::str")]
    pub taxes_base: Decimal,
}

impl Holdings {
    /// Converts all non-zero cash balances to base currency at `date`.
    pub fn cash_in_base(&self, base: &str, date: NaiveDate, rates: &dyn RateLookup) -> Result<Decimal> {
        let mut total = Decimal::ZERO;
        for (currency, amount) in &self.cash {
            if amount.is_zero() {
                continue;
            }
            total += rates.convert(*amount, currency, base, date)?;
        }
        Ok(total)
    }

    /// Open positions with non-zero quantity.
    pub fn open_positions(&self) -> impl Iterator<Item = (&String, &Position)> {
        self.positions.iter().filter(|(_, p)| !p.is_closed())
    }
}

/// Builds FIFO holdings without corporate actions; convenience wrapper for simple callers.
pub fn build_holdings(transactions: &[Transaction], base: &str, rates: &dyn RateLookup) -> Result<Holdings> {
    build_holdings_with(transactions, base, rates, HoldingsOptions::default())
}

/// Builds holdings with a cost-basis method and corporate actions.
/// A transaction's recorded FX rate takes precedence over the lookup.
pub fn build_holdings_with(
    transactions: &[Transaction],
    base: &str,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
) -> Result<Holdings> {
    let mut builder = HoldingsBuilder::new(transactions, base, rates, options);
    for event in ordered_events(transactions, options.corporate_actions) {
        builder.apply(event)?;
    }
    builder.finish()
}

/// Link ids that really do join two legs of one move.
///
/// A `link_id` is a claim, not a fact: brokers print a per-row identifier under the very words
/// a layout maps to this field ("Reference", "Transaction ID"), and one id per row makes every
/// transfer look internal — which silently removes every deposit and every withdrawal from the
/// portfolio's external flows, and with them TWR, XIRR and the capital every rate divides by.
/// A leg whose partner is nowhere in the same set is therefore money crossing the portfolio
/// boundary, which is the rule [`super::scoped_transactions`] has always applied to a scope.
fn paired_links(transactions: &[Transaction]) -> HashSet<&str> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    for t in transactions {
        if let Some(link) = t.link_id.as_deref() {
            *seen.entry(link).or_insert(0) += 1;
        }
    }
    seen.into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(link, _)| link)
        .collect()
}

/// Incremental holdings builder; applies events once so daily valuation stays linear.
pub(crate) struct HoldingsBuilder<'a> {
    holdings: Holdings,
    base: Currency,
    rates: &'a dyn RateLookup,
    options: HoldingsOptions<'a>,
    /// Lots removed by a transfer-out and waiting for the linked transfer-in.
    in_transit: HashMap<String, Vec<Lot>>,
    /// Links with two legs present, resolved once over the whole set — a single event cannot
    /// see whether its counterpart exists. See [`paired_links`].
    paired: HashSet<String>,
}

impl<'a> HoldingsBuilder<'a> {
    /// The transaction set is taken whole because a `link_id` only means something against it.
    pub(crate) fn new(
        transactions: &[Transaction],
        base: &str,
        rates: &'a dyn RateLookup,
        options: HoldingsOptions<'a>,
    ) -> Self {
        HoldingsBuilder {
            holdings: Holdings::default(),
            base: normalize_currency(base),
            rates,
            options,
            in_transit: HashMap::new(),
            paired: paired_links(transactions).into_iter().map(String::from).collect(),
        }
    }

    pub(crate) fn apply(&mut self, event: Event<'_>) -> Result<()> {
        match event {
            Event::Action(action) => apply_corporate_action(&mut self.holdings, action),
            Event::Tx(t) => apply_transaction(
                &mut self.holdings,
                t,
                &self.base,
                self.rates,
                self.options,
                &mut self.in_transit,
                &self.paired,
            ),
        }
    }

    /// Holdings snapshot at the current event.
    pub(crate) fn holdings(&self) -> &Holdings {
        &self.holdings
    }

    pub(crate) fn finish(self) -> Result<Holdings> {
        if let Some(link) = self.in_transit.keys().next() {
            return Err(Error::Invalid(format!(
                "security transfer {link} has an outgoing side but no incoming one"
            )));
        }
        Ok(self.holdings)
    }
}

// --- Event ordering ---------------------------------------------------------

#[derive(Clone, Copy)]
pub(crate) enum Event<'a> {
    Action(&'a CorporateAction),
    Tx(&'a Transaction),
}

impl Event<'_> {
    pub(crate) fn date(&self) -> NaiveDate {
        match self {
            Event::Action(a) => a.date,
            Event::Tx(t) => t.date,
        }
    }
}

/// Merges transactions and corporate actions chronologically.
/// Same-day order is action, transfer-out, transfer-in, then original transaction order.
pub(crate) fn ordered_events<'a>(
    transactions: &'a [Transaction],
    actions: &'a [CorporateAction],
) -> Vec<Event<'a>> {
    fn phase(kind: TransactionKind) -> u8 {
        match kind {
            TransactionKind::SecurityTransferOut => 1,
            TransactionKind::SecurityTransferIn => 2,
            _ => 3,
        }
    }

    let mut events: Vec<(NaiveDate, u8, usize, Event<'a>)> =
        Vec::with_capacity(transactions.len() + actions.len());
    for (i, a) in actions.iter().enumerate() {
        events.push((a.date, 0, i, Event::Action(a)));
    }
    for (i, t) in transactions.iter().enumerate() {
        events.push((t.date, phase(t.kind), i, Event::Tx(t)));
    }
    // Original index preserves storage order within the same date and phase.
    events.sort_by_key(|(date, phase, index, _)| (*date, *phase, *index));
    events.into_iter().map(|(_, _, _, e)| e).collect()
}

// --- Corporate actions ------------------------------------------------------

/// Applies a split: quantity is multiplied and per-unit cost divided, preserving total cost.
fn apply_corporate_action(h: &mut Holdings, action: &CorporateAction) -> Result<()> {
    let Some(position) = h.positions.get_mut(&action.security_id) else {
        // No position existed on the split date.
        return Ok(());
    };
    let factor = action.quantity_factor()?;
    if factor.is_zero() {
        return Err(Error::Invalid(format!("split {} has zero factor", action.id)));
    }
    for lot in &mut position.lots {
        lot.quantity *= factor;
        lot.cost_per_unit /= factor;
        lot.cost_per_unit_base /= factor;
    }
    position.quantity *= factor;
    // Account quantities change by the same factor; ownership does not move.
    for quantity in position.accounts.values_mut() {
        *quantity *= factor;
    }
    Ok(())
}

// --- Transactions -----------------------------------------------------------

fn apply_transaction(
    h: &mut Holdings,
    t: &Transaction,
    base: &str,
    rates: &dyn RateLookup,
    options: HoldingsOptions<'_>,
    in_transit: &mut HashMap<String, Vec<Lot>>,
    paired: &HashSet<String>,
) -> Result<()> {
    t.validate()?;
    let rate = resolve_rate(t, base, rates)?;
    let charges = Charges::of(t, rate, base, rates)?;

    // Keep cash in transaction currency; FX conversion belongs to valuation.
    let delta = t.cash_delta();
    if !delta.is_zero() {
        *h.cash.entry(t.currency.clone()).or_insert(Decimal::ZERO) += delta;
    }
    // A charge billed in another currency leaves that currency's balance, not this one's.
    for (currency, amount) in t.foreign_charge_legs() {
        *h.cash.entry(currency).or_insert(Decimal::ZERO) += amount;
    }

    // Any non-zero quantity reveals the broker's effective trading step.
    let observed = t.security_id.clone().filter(|_| !t.quantity.is_zero());

    match t.kind {
        TransactionKind::Buy => acquire(h, t, &charges, options.cost_basis),
        TransactionKind::Sell => {
            let realized = dispose(h, t, rate, &charges)?;
            h.realized_pnl_base += realized;
        }

        // Inbound delivery is an external flow at gross value; cost includes fees and taxes.
        TransactionKind::DeliveryInbound => {
            acquire(h, t, &charges, options.cost_basis);
            h.external_flows.push(CashFlow {
                date: t.date,
                amount_base: charges.gross_base,
            });
        }
        // Outbound delivery records P/L and a matching negative gross external flow.
        TransactionKind::DeliveryOutbound => {
            let realized = dispose(h, t, rate, &charges)?;
            h.realized_pnl_base += realized;
            h.external_flows.push(CashFlow {
                date: t.date,
                amount_base: -charges.gross_base,
            });
        }

        TransactionKind::SecurityTransferOut => {
            let link = t.link_id.clone().expect("validated: linked side has link_id");
            let lots = take_lots(h, t)?;
            in_transit.insert(link, lots);
        }
        TransactionKind::SecurityTransferIn => {
            let link = t.link_id.as_deref().expect("validated: linked side has link_id");
            let lots = in_transit.remove(link).ok_or_else(|| {
                Error::Invalid(format!(
                    "security transfer {link} has no outgoing side in this scope — \
                     use DELIVERY_INBOUND if the shares came from outside the portfolio"
                ))
            })?;
            let sid = t.security_id.clone().expect("validated: transfer has security");
            let position = h
                .positions
                .entry(sid.clone())
                .or_insert_with(|| Position::new(&sid, &t.currency));
            for lot in lots {
                position.quantity += lot.quantity;
                position.move_on_account(&t.account_id, lot.quantity);
                position.cost_basis += lot.quantity * lot.cost_per_unit;
                position.cost_basis_base += lot.quantity * lot.cost_per_unit_base;
                add_lot(position, lot, options.cost_basis);
            }
        }

        TransactionKind::Dividend => {
            let net = charges.gross_base;
            h.dividends_base += net;
            // Record every dividend; the security ID is optional.
            h.income.push(income_record(t, &charges, t.amount * rate, net));
            if let Some(p) = t.security_id.as_ref().and_then(|sid| h.positions.get_mut(sid)) {
                p.dividends_base += net;
            }
        }
        TransactionKind::Interest => {
            let net = charges.gross_base;
            h.interest_base += net;
            h.income.push(income_record(t, &charges, t.amount * rate, net));
        }
        // Cashback and rewards are income too; they carry no security, so they land in the
        // income report by kind and nowhere near a position's dividend total.
        TransactionKind::Cashback | TransactionKind::Reward => {
            let net = charges.gross_base;
            h.income.push(income_record(t, &charges, t.amount * rate, net));
        }
        // Interest charges are negative income events; their components are not split here.
        TransactionKind::InterestCharge => {
            let net = -(t.amount * rate);
            h.interest_base += net;
            h.income.push(income_record(t, &charges, net, net));
        }

        // Refunds reduce accumulated expenses instead of becoming income.
        TransactionKind::Fee | TransactionKind::FeeRefund => {
            let signed = charge(h, t, rate);
            h.fees_base += signed;
        }
        TransactionKind::Tax | TransactionKind::TaxRefund => {
            let signed = charge(h, t, rate);
            h.taxes_base += signed;
        }

        TransactionKind::Deposit => h.external_flows.push(CashFlow {
            date: t.date,
            amount_base: t.amount * rate,
        }),
        TransactionKind::Withdrawal => h.external_flows.push(CashFlow {
            date: t.date,
            amount_base: -t.amount * rate,
        }),
        // A paired transfer moves money inside the portfolio and is no flow. A leg with no
        // counterpart — no `link_id` at all, or one no other row carries — is money that
        // crossed the portfolio boundary: an incoming bank transfer or a card payment.
        // Treating it as internal would make spending look like a loss and hide every
        // contribution from TWR/XIRR.
        TransactionKind::TransferIn | TransactionKind::TransferOut => {
            if !t.link_id.as_deref().is_some_and(|link| paired.contains(link)) {
                h.external_flows.push(CashFlow {
                    date: t.date,
                    amount_base: delta * rate,
                });
            }
        }
    }

    if let Some(position) = observed.and_then(|sid| h.positions.get_mut(&sid)) {
        position.observe_quantity(t.quantity);
    }
    Ok(())
}

/// Builds an income event from precomputed gross and net base amounts.
fn income_record(t: &Transaction, charges: &Charges, gross_base: Decimal, net_base: Decimal) -> IncomeRecord {
    IncomeRecord {
        date: t.date,
        account_id: t.account_id.clone(),
        security_id: t.security_id.clone(),
        kind: t.kind,
        gross_base,
        taxes_base: charges.taxes_base,
        fees_base: charges.fees_base,
        net_base,
        currency: t.currency.clone(),
        // Keep the original-currency sign aligned with the base-currency amount.
        gross_in_currency: if gross_base.is_sign_negative() {
            -t.amount
        } else {
            t.amount
        },
    }
}

/// Records a signed fee or tax event and returns its signed base amount.
fn charge(h: &mut Holdings, t: &Transaction, rate: Decimal) -> Decimal {
    let refund = matches!(t.kind, TransactionKind::FeeRefund | TransactionKind::TaxRefund);
    let amount_base = if refund {
        -(t.amount * rate)
    } else {
        t.amount * rate
    };
    h.charges.push(ChargeRecord {
        date: t.date,
        account_id: t.account_id.clone(),
        security_id: t.security_id.clone(),
        kind: t.kind,
        amount_base,
        currency: t.currency.clone(),
        amount_in_currency: if refund { -t.amount } else { t.amount },
    });
    amount_base
}

/// Resolves an operation FX rate: base currency is 1, then transaction rate, then lookup.
pub(crate) fn resolve_rate(t: &Transaction, base: &str, rates: &dyn RateLookup) -> Result<Decimal> {
    if t.currency == base {
        return Ok(Decimal::ONE);
    }
    if let Some(r) = t.fx_rate_to_base {
        return Ok(r);
    }
    rates
        .rate_as_of(&t.currency, base, t.date)?
        .ok_or_else(|| Error::MissingMarketData {
            kind: "fx rate",
            key: format!("{}/{}", t.currency, base),
            date: t.date,
        })
}

/// The rate one charge is converted at. `fx_rate_to_base` is the rate of the *transaction's*
/// currency, so a commission or a withholding billed in another one is converted at its own
/// pair on the same day instead of inheriting a rate that belongs elsewhere.
pub(crate) fn charge_rate(
    t: &Transaction,
    currency: &str,
    base: &str,
    rates: &dyn RateLookup,
) -> Result<Decimal> {
    if currency == t.currency {
        return resolve_rate(t, base, rates);
    }
    if currency == base {
        return Ok(Decimal::ONE);
    }
    rates
        .rate_as_of(currency, base, t.date)?
        .ok_or_else(|| Error::MissingMarketData {
            kind: "fx rate",
            key: format!("{currency}/{base}"),
            date: t.date,
        })
}

/// One transaction's charges and total, each converted the way it was actually paid.
pub(crate) struct Charges {
    pub fees_base: Decimal,
    pub taxes_base: Decimal,
    /// The total in the transaction's own currency. A charge billed elsewhere is carried back
    /// into it through the base currency — both rates are of the same day, so this is
    /// arithmetic over what was paid rather than a market cross rate.
    pub gross_in_currency: Decimal,
    pub gross_base: Decimal,
}

impl Charges {
    pub fn of(t: &Transaction, rate: Decimal, base: &str, rates: &dyn RateLookup) -> Result<Self> {
        let fees_base = t.fees * charge_rate(t, t.fees_in(), base, rates)?;
        let taxes_base = t.taxes * charge_rate(t, t.taxes_in(), base, rates)?;
        let foreign = t.fee_currency.as_ref().map_or(Decimal::ZERO, |_| fees_base)
            + t.tax_currency.as_ref().map_or(Decimal::ZERO, |_| taxes_base);

        let sign = Decimal::from(t.kind.charge_sign());
        let mut gross_in_currency = t.gross_in_transaction_currency();
        let mut gross_base = gross_in_currency * rate;
        if !foreign.is_zero() {
            gross_base += sign * foreign;
            if !rate.is_zero() {
                gross_in_currency += sign * foreign / rate;
            }
        }
        Ok(Charges {
            fees_base,
            taxes_base,
            gross_in_currency,
            gross_base,
        })
    }
}

// --- Lots -------------------------------------------------------------------

/// Adds a lot using FIFO or a single weighted-average lot.
fn add_lot(position: &mut Position, lot: Lot, method: CostBasisMethod) {
    match method {
        CostBasisMethod::Fifo => position.lots.push(lot),
        CostBasisMethod::AverageCost => match position.lots.first_mut() {
            None => position.lots.push(lot),
            Some(existing) => {
                let total = existing.quantity + lot.quantity;
                if total.is_zero() {
                    return;
                }
                existing.cost_per_unit =
                    (existing.quantity * existing.cost_per_unit + lot.quantity * lot.cost_per_unit) / total;
                existing.cost_per_unit_base = (existing.quantity * existing.cost_per_unit_base
                    + lot.quantity * lot.cost_per_unit_base)
                    / total;
                existing.quantity = total;
                existing.acquired_at = existing.acquired_at.min(lot.acquired_at);
            }
        },
    }
}

/// Acquires securities with known cost (purchase or inbound delivery).
fn acquire(h: &mut Holdings, t: &Transaction, charges: &Charges, method: CostBasisMethod) {
    let sid = t
        .security_id
        .clone()
        .expect("validated: acquisition has security");
    let position = h
        .positions
        .entry(sid.clone())
        .or_insert_with(|| Position::new(&sid, &t.currency));

    // Total cost is trade amount plus fees and taxes, whichever currency each was paid in.
    let cost = charges.gross_in_currency;
    let cost_base = charges.gross_base;

    position.quantity += t.quantity;
    position.move_on_account(&t.account_id, t.quantity);
    position.cost_basis += cost;
    position.cost_basis_base += cost_base;
    add_lot(
        position,
        Lot {
            acquired_at: t.date,
            quantity: t.quantity,
            cost_per_unit: cost / t.quantity,
            // Capture the transaction FX rate; lot cost never revalues.
            cost_per_unit_base: cost_base / t.quantity,
        },
        method,
    );
}

/// Removes `quantity` from a position and returns the consumed lots.
fn take_lots(h: &mut Holdings, t: &Transaction) -> Result<Vec<Lot>> {
    let sid = t.security_id.clone().expect("validated: disposal has security");
    let position = h
        .positions
        .get_mut(&sid)
        .ok_or_else(|| Error::Invalid(format!("disposal of {sid} with no open position")))?;

    if t.quantity > position.quantity {
        return Err(Error::Invalid(format!(
            "disposal of {} exceeds position {} for {sid} on {}",
            t.quantity, position.quantity, t.date
        )));
    }

    // Consume from the queue head; FIFO uses oldest lots, average cost has one lot.
    let mut remaining = t.quantity;
    let mut taken = Vec::new();
    while remaining > Decimal::ZERO {
        let lot = position.lots.first_mut().expect("quantity checked above");
        let take = remaining.min(lot.quantity);
        taken.push(Lot {
            acquired_at: lot.acquired_at,
            quantity: take,
            cost_per_unit: lot.cost_per_unit,
            cost_per_unit_base: lot.cost_per_unit_base,
        });
        lot.quantity -= take;
        remaining -= take;
        if lot.quantity.is_zero() {
            position.lots.remove(0);
        }
    }

    let removed_cost: Decimal = taken.iter().map(|l| l.quantity * l.cost_per_unit).sum();
    let removed_cost_base: Decimal = taken.iter().map(|l| l.quantity * l.cost_per_unit_base).sum();
    position.quantity -= t.quantity;
    // Attribute the disposal to its transaction account; account totals remain auditable.
    position.move_on_account(&t.account_id, -t.quantity);
    position.cost_basis -= removed_cost;
    position.cost_basis_base -= removed_cost_base;
    Ok(taken)
}

/// Disposes securities and records realized P/L in base currency.
fn dispose(h: &mut Holdings, t: &Transaction, rate: Decimal, charges: &Charges) -> Result<Decimal> {
    let sid = t.security_id.clone().expect("validated: disposal has security");
    let cost_currency = h.positions.get(&sid).map(|p| p.cost_currency.clone());
    let taken = take_lots(h, t)?;
    let removed_cost_base: Decimal = taken.iter().map(|l| l.quantity * l.cost_per_unit_base).sum();
    let removed_cost: Decimal = taken.iter().map(|l| l.quantity * l.cost_per_unit).sum();

    let proceeds_base = charges.gross_base;
    // Realized P/L is base-currency proceeds minus historical base cost.
    let realized = proceeds_base - removed_cost_base;

    // What the money put in is worth today minus what it was worth then: the rate's share of
    // the result. A sale settled in another currency than the purchase has no such pair, so
    // there is nothing to split and the whole result stays with the instrument.
    let currency_gain_base = match cost_currency {
        Some(currency) if currency == t.currency => removed_cost * rate - removed_cost_base,
        _ => Decimal::ZERO,
    };

    h.realized.push(RealizedGain {
        date: t.date,
        security_id: sid.clone(),
        kind: t.kind,
        quantity: t.quantity,
        // Keep gross proceeds, fees, and taxes separate for reporting.
        proceeds_base: t.amount * rate,
        fees_base: charges.fees_base,
        taxes_base: charges.taxes_base,
        cost_base: removed_cost_base,
        cost_in_currency: removed_cost,
        gain_base: realized,
        currency_gain_base,
        lots: taken,
    });
    h.realized_currency_gain_base += currency_gain_base;
    if let Some(p) = h.positions.get_mut(&sid) {
        p.realized_pnl_base += realized;
    }
    Ok(realized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TransactionKind;
    use chrono::NaiveDate;
    use rust_decimal_macros::dec;

    /// Missing lookup: the transaction must still supply a usable rate.
    struct NoRates;
    impl RateLookup for NoRates {
        fn rate_as_of(&self, _: &str, _: &str, _: NaiveDate) -> Result<Option<Decimal>> {
            Ok(None)
        }
    }

    /// A base-currency dividend must ignore a misleading broker FX column.
    #[test]
    fn a_transaction_in_the_base_currency_is_never_converted() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut t = Transaction::cash("acc", TransactionKind::Dividend, date, dec!(0.22), "EUR");
        t.fx_rate_to_base = Some(dec!(0.853898));

        assert_eq!(resolve_rate(&t, "EUR", &NoRates).unwrap(), Decimal::ONE);
    }

    /// For foreign currency, the transaction's recorded rate overrides the lookup.
    #[test]
    fn the_rate_recorded_in_the_trade_still_wins_for_other_currencies() {
        let date = NaiveDate::from_ymd_opt(2025, 8, 14).unwrap();
        let mut t = Transaction::cash("acc", TransactionKind::Dividend, date, dec!(0.26), "USD");
        t.fx_rate_to_base = Some(dec!(0.853898));

        assert_eq!(resolve_rate(&t, "EUR", &NoRates).unwrap(), dec!(0.853898));
    }
}
