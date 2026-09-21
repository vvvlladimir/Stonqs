//! Interactive Brokers Flex Query statements through the whole import: XML in, drafts out.
//!
//! The fixture is one statement carrying every shape the reader has to tell apart — an order and
//! the two fills behind it, a dividend with its withholding tax, a currency conversion, a split
//! and a cash movement that is a withdrawal only because of its sign.

use super::*;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sq_core::import::is_flex;

/// One account, one month, every section a Flex Query for this app is asked to select.
/// Values are the ones Interactive Brokers prints: `proceeds` is negative for a purchase,
/// `ibCommission` is negative, a withdrawal is a negative `Deposits/Withdrawals`.
const STATEMENT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<FlexQueryResponse queryName="Stonqs" type="AF">
 <FlexStatements count="1">
  <FlexStatement accountId="U1234567" fromDate="20240601" toDate="20240630">
   <AccountInformation accountId="U1234567" acctAlias="IB Main" currency="EUR" />
   <SecuritiesInfo>
    <SecurityInfo conid="265598" symbol="AAPL" isin="US0378331005" description="APPLE INC" />
    <SecurityInfo conid="756733" symbol="SPY" description="SPDR S&amp;P 500 ETF TRUST" />
   </SecuritiesInfo>
   <Trades>
    <Trade accountId="U1234567" acctAlias="IB Main" conid="265598" symbol="AAPL"
      description="APPLE INC" assetCategory="STK" isin="US0378331005" currency="USD"
      fxRateToBase="0.92" tradeDate="20240603" quantity="4" tradePrice="190" proceeds="-760"
      ibCommission="-0.4" taxes="0" multiplier="1" buySell="BUY" levelOfDetail="EXECUTION"
      tradeID="111" />
    <Trade accountId="U1234567" acctAlias="IB Main" conid="265598" symbol="AAPL"
      description="APPLE INC" assetCategory="STK" isin="US0378331005" currency="USD"
      fxRateToBase="0.92" tradeDate="20240603" quantity="6" tradePrice="190" proceeds="-1140"
      ibCommission="-0.6" taxes="0" multiplier="1" buySell="BUY" levelOfDetail="EXECUTION"
      tradeID="112" />
    <Trade accountId="U1234567" acctAlias="IB Main" conid="265598" symbol="AAPL"
      description="APPLE INC" assetCategory="STK" isin="US0378331005" currency="USD"
      fxRateToBase="0.92" tradeDate="20240603" quantity="10" tradePrice="190" proceeds="-1900"
      ibCommission="-1" taxes="0" multiplier="1" buySell="BUY" levelOfDetail="ORDER"
      tradeID="110" />
    <Trade accountId="U1234567" acctAlias="IB Main" conid="756733" symbol="SPY"
      description="SPDR S&amp;P 500 ETF TRUST" assetCategory="STK" currency="USD"
      fxRateToBase="0.92" tradeDate="20240610" quantity="-2" tradePrice="540" proceeds="1080"
      ibCommission="-0.5" taxes="0" multiplier="1" buySell="SELL" levelOfDetail="ORDER"
      tradeID="120" />
    <Trade accountId="U1234567" acctAlias="IB Main" symbol="EUR.USD" description="EUR.USD"
      assetCategory="CASH" currency="USD" ibCommissionCurrency="USD" tradeDate="20240605"
      quantity="1000" tradePrice="1.08" proceeds="-1080" ibCommission="-2"
      levelOfDetail="ORDER" tradeID="130" />
   </Trades>
   <CashTransactions>
    <CashTransaction accountId="U1234567" acctAlias="IB Main" conid="265598" symbol="AAPL"
      isin="US0378331005" description="AAPL(US0378331005) CASH DIVIDEND USD 0.25 PER SHARE"
      dateTime="20240620" amount="2.5" currency="USD" fxRateToBase="0.92" type="Dividends" />
    <CashTransaction accountId="U1234567" acctAlias="IB Main" conid="265598" symbol="AAPL"
      isin="US0378331005" description="AAPL(US0378331005) CASH DIVIDEND - US TAX"
      dateTime="20240620" amount="-0.38" currency="USD" fxRateToBase="0.92"
      type="Withholding Tax" />
    <CashTransaction accountId="U1234567" acctAlias="IB Main" description="CASH RECEIPTS"
      dateTime="20240601" amount="5000" currency="EUR" fxRateToBase="1"
      type="Deposits/Withdrawals" />
    <CashTransaction accountId="U1234567" acctAlias="IB Main" description="DISBURSEMENT"
      dateTime="20240628" amount="-750" currency="EUR" fxRateToBase="1"
      type="Deposits/Withdrawals" />
    <CashTransaction accountId="U1234567" acctAlias="IB Main" description="BALANCE INTEREST"
      dateTime="20240630" amount="-1.2" currency="EUR" fxRateToBase="1"
      type="Broker Interest Paid" />
   </CashTransactions>
   <CorporateActions>
    <CorporateAction accountId="U1234567" acctAlias="IB Main" conid="756733" symbol="SPY"
      description="SPY(US78462F1030) SPLIT 2 FOR 1" dateTime="20240625" quantity="8"
      proceeds="0" currency="USD" type="FS" />
   </CorporateActions>
  </FlexStatement>
 </FlexStatements>
</FlexQueryResponse>"#;

fn ib_account(store: &Store) -> Account {
    depot(store, "IB Main", "EUR")
}

/// The layout the service picks by itself, plus the one choice the wizard still asks for: which
/// account "IB Main" is. Reaching it through a preview is how the screen reaches it too.
fn ib_mapping(account: &Account) -> ImportMapping {
    let empty = Store::open_in_memory().unwrap();
    let mut mapping = ImportService::new(&empty)
        .preview(STATEMENT.as_bytes(), &ParseConfig::default(), None, &[])
        .unwrap()
        .mapping;
    mapping
        .account_aliases
        .insert("IBMAIN".to_string(), account.id.clone());
    mapping
}

fn preview_of(store: &Store, mapping: &ImportMapping) -> sq_core::import::ImportPreview {
    ImportService::new(store)
        .with_base_currency("EUR")
        .preview(STATEMENT.as_bytes(), &ParseConfig::default(), Some(mapping), &[])
        .unwrap()
}

/// An XML statement needs no parse settings and no column mapping: the file is recognised and
/// laid out by the reader, so the wizard opens on the account question alone.
#[test]
fn a_flex_statement_lays_itself_out() {
    assert!(is_flex(STATEMENT.as_bytes()));

    let store = Store::open_in_memory().unwrap();
    let preview = ImportService::new(&store)
        .preview(STATEMENT.as_bytes(), &ParseConfig::default(), None, &[])
        .unwrap();

    assert!(
        preview.unknown_kinds().is_empty(),
        "{:?}",
        preview.unknown_kinds()
    );
    assert_eq!(preview.mapping.column(ImportField::Date), Some("Date"));
    assert_eq!(preview.config.date_format.as_deref(), Some("%Y-%m-%d"));
    assert_eq!(preview.amount_sign, AmountSign::Signed);

    // The alias is what the user named the account, not the broker's identifier.
    assert_eq!(preview.accounts.len(), 1);
    assert_eq!(preview.accounts[0].value, "IB Main");
}

/// Asking a Flex Query for executions *and* orders prints one purchase twice. Ten shares were
/// bought as 4 + 6, and the order line already says 10: keeping all three would import 20.
#[test]
fn an_order_replaces_the_fills_it_was_made_of() {
    let store = Store::open_in_memory().unwrap();
    let account = ib_account(&store);
    let preview = preview_of(&store, &ib_mapping(&account));

    let apple: Vec<&sq_core::import::TransactionDraft> = preview
        .rows
        .iter()
        .filter_map(|r| r.draft.as_ref())
        .filter(|d| d.symbol.as_deref() == Some("AAPL") && d.kind == TransactionKind::Buy)
        .collect();
    assert_eq!(apple.len(), 1, "4 + 6 and 10 are the same purchase");
    assert_eq!(apple[0].quantity, dec!(10));

    // 10 × 190 = 1900 paid, commission 1 on top, fx 0.92 recorded as the trade's own rate.
    assert_eq!(apple[0].amount, dec!(1900));
    assert_eq!(apple[0].fees, dec!(1));
    assert_eq!(apple[0].fx_rate_to_base, Some(dec!(0.92)));
    assert_eq!(apple[0].currency, "USD");
    assert_eq!(apple[0].isin.as_deref(), Some("US0378331005"));
}

/// Interactive Brokers signs the number, not the wording: one `Deposits/Withdrawals` value
/// covers both directions, and one `quantity` covers a sale.
#[test]
fn the_sign_of_the_number_gives_the_direction() {
    let store = Store::open_in_memory().unwrap();
    let account = ib_account(&store);
    let preview = preview_of(&store, &ib_mapping(&account));

    let kinds: Vec<(TransactionKind, Decimal)> = preview
        .rows
        .iter()
        .filter_map(|r| r.draft.as_ref())
        .map(|d| (d.kind, d.amount))
        .collect();

    assert!(
        kinds.contains(&(TransactionKind::Deposit, dec!(5000))),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&(TransactionKind::Withdrawal, dec!(750))),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&(TransactionKind::InterestCharge, dec!(1.2))),
        "{kinds:?}"
    );

    // -2 shares at 540 is a sale of two: 2 × 540 = 1080 received.
    let sell = preview
        .rows
        .iter()
        .filter_map(|r| r.draft.as_ref())
        .find(|d| d.symbol.as_deref() == Some("SPY") && d.kind == TransactionKind::Sell)
        .expect("the SPY sale");
    assert_eq!(sell.quantity, dec!(2), "a draft quantity is never negative");
    assert_eq!(sell.amount, dec!(1080));
    // The ISIN is absent from the trade and read off the instrument section by conid.
    assert_eq!(sell.isin, None, "SPY's own section carries no ISIN either");
}

/// A dividend and the tax withheld from it are two rows and stay two operations: netting them
/// would hide the gross payment every yield figure is computed from.
#[test]
fn withholding_tax_stays_beside_the_dividend_it_came_from() {
    let store = Store::open_in_memory().unwrap();
    let account = ib_account(&store);
    let preview = preview_of(&store, &ib_mapping(&account));

    let dividend = preview
        .rows
        .iter()
        .filter_map(|r| r.draft.as_ref())
        .find(|d| d.kind == TransactionKind::Dividend)
        .expect("the dividend");
    let tax = preview
        .rows
        .iter()
        .filter_map(|r| r.draft.as_ref())
        .find(|d| d.kind == TransactionKind::Tax)
        .expect("the withholding tax");

    // 0.25 per share on 10 shares = 2.50 gross; 0.38 withheld is 15% of it, kept apart.
    assert_eq!(dividend.amount, dec!(2.5));
    assert_eq!(tax.amount, dec!(0.38));
    assert_eq!(dividend.date, "2024-06-20".parse::<NaiveDate>().unwrap());
    assert_eq!(tax.date, dividend.date);
    assert_eq!(
        tax.symbol.as_deref(),
        Some("AAPL"),
        "the tax names its instrument"
    );
    // Both settle in cash, so both land on the deposit account behind the depot.
    assert_eq!(tax.account_id, account.settlement_account_id());
}

/// A currency trade moves value *inside* the portfolio. Read as two unlinked rows it would be
/// a withdrawal followed by a deposit and would break the return; the legs are paired here.
#[test]
fn a_currency_trade_becomes_one_linked_pair() {
    let store = Store::open_in_memory().unwrap();
    let account = ib_account(&store);
    let preview = preview_of(&store, &ib_mapping(&account));

    let legs: Vec<&sq_core::import::TransactionDraft> = preview
        .rows
        .iter()
        .filter_map(|r| r.draft.as_ref())
        .filter(|d| d.kind.is_linked_side())
        .collect();
    assert_eq!(legs.len(), 2, "{legs:?}");

    // 1000 EUR bought at 1.08 costs 1080 USD: the euros come in, the dollars go out.
    let euros = legs.iter().find(|d| d.currency == "EUR").unwrap();
    let dollars = legs.iter().find(|d| d.currency == "USD").unwrap();
    assert_eq!(euros.kind, TransactionKind::TransferIn);
    assert_eq!(euros.amount, dec!(1000));
    assert_eq!(dollars.kind, TransactionKind::TransferOut);
    assert_eq!(dollars.amount, dec!(1080));

    assert_eq!(euros.link_id, dollars.link_id, "the pair must carry one link");
    assert!(euros.link_id.is_some());
    // The commission was charged in dollars, so only that leg carries it.
    assert_eq!(dollars.fees, dec!(2));
    assert_eq!(euros.fees, Decimal::ZERO);
    assert!(legs.iter().all(|d| d.security_id.is_none() && d.symbol.is_none()));
}

/// A split moves no money and no quantity the model would take from the broker — lots are
/// adjusted by a corporate action, not by a delivery. The row is skipped, but visibly.
#[test]
fn a_corporate_action_is_counted_and_skipped_rather_than_guessed_at() {
    let store = Store::open_in_memory().unwrap();
    let account = ib_account(&store);
    let preview = preview_of(&store, &ib_mapping(&account));

    let skipped = preview
        .kinds
        .iter()
        .find(|k| k.ignored)
        .expect("the corporate action stays in the kind list");
    assert_eq!(skipped.count, 1);
    assert_eq!(preview.summary.ignored, 1);
    assert!(preview.unknown_kinds().is_empty());
}

/// Two exports of overlapping months are the normal way to use a Flex Query, so the second one
/// has to be a no-op — the same guarantee the CSV import gives.
#[test]
fn importing_the_same_statement_twice_writes_nothing_the_second_time() {
    let store = Store::open_in_memory().unwrap();
    let account = ib_account(&store);
    let mapping = ib_mapping(&account);
    let service = ImportService::new(&store).with_base_currency("EUR");
    let options = ImportOptions::default();

    let first = service
        .preview(STATEMENT.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    let written = service.commit(&first, &options).unwrap();
    assert!(written.imported > 0, "{:?}", first.summary);

    let again = service
        .preview(STATEMENT.as_bytes(), &ParseConfig::default(), Some(&mapping), &[])
        .unwrap();
    assert_eq!(again.summary.ready, 0);
    assert_eq!(again.summary.duplicates, written.imported);
    assert_eq!(service.commit(&again, &options).unwrap().imported, 0);
}
