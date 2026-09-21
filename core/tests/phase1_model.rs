//! FX exchange, security transfers, splits, cost-basis methods, and new operations.

mod support;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sq_core::calc::{HoldingsOptions, build_holdings, build_holdings_with, value_holdings};
use sq_core::model::{CorporateAction, CostBasisMethod, Transaction, TransactionKind};
use support::{FakePrices, FakeRates, d};

const ACC: &str = "acc-1";
const ACC2: &str = "acc-2";
const AAPL: &str = "sec-aapl";

/// FX exchange prevents a negative purchased-currency balance:
/// +1000 EUR; exchange 920 EUR → 1000 USD; buy 975 USD; balances EUR 80, USD 25.
#[test]
fn currency_exchange_keeps_both_balances_positive() {
    let (out, inc) =
        Transaction::currency_exchange(ACC, ACC, d(2024, 6, 2), dec!(920), "EUR", dec!(1000), "USD");

    // The rate is derived from the exchange pair.
    assert_eq!(
        out.implied_exchange_rate(&inc).unwrap().round_dp(6),
        dec!(1.086957)
    );

    let txs = vec![
        Transaction::cash(ACC, TransactionKind::Deposit, d(2024, 6, 1), dec!(1000), "EUR"),
        out,
        inc,
        Transaction::buy(ACC, AAPL, d(2024, 6, 5), dec!(5), dec!(195), "USD").with_fx_rate(dec!(0.92)),
    ];

    let rates = FakeRates::new().with("USD", "EUR", d(2024, 6, 1), dec!(0.92));
    let h = build_holdings(&txs, "EUR", &rates).unwrap();

    assert_eq!(h.cash["EUR"], dec!(80));
    assert_eq!(h.cash["USD"], dec!(25));
    // Internal exchange creates no external flow; only the 1000 EUR deposit does.
    assert_eq!(h.external_flows.len(), 1);
    assert_eq!(h.external_flows[0].amount_base, dec!(1000));
}

/// A transfer leg without a counterpart in the portfolio is money crossing its boundary —
/// broker exports print bank transfers and card payments that way:
/// +1000 in, −250 out, both unlinked; flows = +1000 and −250; cash = 750.
#[test]
fn unlinked_cash_transfer_counts_as_external_flow() {
    let txs = vec![
        Transaction::cash(ACC, TransactionKind::TransferIn, d(2024, 4, 1), dec!(1000), "EUR"),
        Transaction::cash(ACC, TransactionKind::TransferOut, d(2024, 4, 8), dec!(250), "EUR"),
    ];

    let h = build_holdings(&txs, "EUR", &FakeRates::new()).unwrap();

    assert_eq!(h.cash["EUR"], dec!(750));
    assert_eq!(
        h.external_flows.iter().map(|f| f.amount_base).collect::<Vec<_>>(),
        vec![dec!(1000), dec!(-250)]
    );

    // The same pair, linked, is one move between own accounts: no flow at all.
    let (out, inc) = Transaction::cash_transfer(ACC, ACC2, d(2024, 4, 8), dec!(250), "EUR");
    let linked = build_holdings(&[out, inc], "EUR", &FakeRates::new()).unwrap();
    assert!(linked.external_flows.is_empty());
}

/// A `link_id` no other row carries pairs nothing, so the money crossed the boundary after
/// all. Broker exports carry a per-row identifier under the same words a link column uses,
/// and one id per row would otherwise turn every deposit and card payment into a result:
/// +1000 in, −250 out, each with an id of its own; flows = +1000 and −250, exactly as if
/// neither had been linked at all.
#[test]
fn a_link_with_no_counterpart_is_still_an_external_flow() {
    let txs = vec![
        Transaction::cash(ACC, TransactionKind::TransferIn, d(2024, 4, 1), dec!(1000), "EUR")
            .with_link("broker-row-1"),
        Transaction::cash(ACC, TransactionKind::TransferOut, d(2024, 4, 8), dec!(250), "EUR")
            .with_link("broker-row-2"),
    ];

    let h = build_holdings(&txs, "EUR", &FakeRates::new()).unwrap();

    assert_eq!(h.cash["EUR"], dec!(750));
    assert_eq!(
        h.external_flows.iter().map(|f| f.amount_base).collect::<Vec<_>>(),
        vec![dec!(1000), dec!(-250)]
    );

    // Two rows really carrying one id are the internal move the column claims to describe.
    let paired = vec![
        Transaction::cash(ACC, TransactionKind::TransferOut, d(2024, 4, 8), dec!(250), "EUR")
            .with_link("move-7"),
        Transaction::cash(ACC2, TransactionKind::TransferIn, d(2024, 4, 8), dec!(250), "EUR")
            .with_link("move-7"),
    ];
    let linked = build_holdings(&paired, "EUR", &FakeRates::new()).unwrap();
    assert!(linked.external_flows.is_empty());
}

/// Moving securities between owned accounts creates no realized gain:
/// buy 10 × 100 = 1000 USD; move 10 shares; quantity 10, cost 1000, gain 0.
#[test]
fn security_transfer_preserves_lots() {
    let (out, inc) = Transaction::security_transfer(ACC, ACC2, AAPL, d(2024, 3, 1), dec!(10), "USD");
    let txs = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 10), dec!(10), dec!(100), "USD"),
        out,
        inc,
    ];

    let h = build_holdings(&txs, "USD", &FakeRates::new()).unwrap();
    let p = &h.positions[AAPL];

    assert_eq!(p.quantity, dec!(10));
    assert_eq!(p.cost_basis_base, dec!(1000));
    assert_eq!(p.realized_pnl_base, Decimal::ZERO);
    assert_eq!(p.lots.len(), 1);
    assert_eq!(
        p.lots[0].acquired_at,
        d(2024, 1, 10),
        "the buy date survived the move"
    );
    assert_eq!(h.realized_pnl_base, Decimal::ZERO);
}

/// Transfer-side order within one day must not change the result.
/// Both legs share a date; calculation orders outgoing before incoming.
#[test]
fn transfer_sides_may_arrive_in_any_order() {
    let (out, inc) = Transaction::security_transfer(ACC, ACC2, AAPL, d(2024, 3, 1), dec!(10), "USD");
    let buy = Transaction::buy(ACC, AAPL, d(2024, 1, 10), dec!(10), dec!(100), "USD");

    let forward = build_holdings(&[buy.clone(), out.clone(), inc.clone()], "USD", &FakeRates::new()).unwrap();
    let reversed = build_holdings(&[buy, inc, out], "USD", &FakeRates::new()).unwrap();

    assert_eq!(
        forward.positions[AAPL].cost_basis_base,
        reversed.positions[AAPL].cost_basis_base
    );
    assert_eq!(reversed.positions[AAPL].quantity, dec!(10));
}

/// An unmatched incoming transfer is an explicit error, not zero-cost inventory.
#[test]
fn incoming_transfer_without_counterpart_is_rejected() {
    let (_, inc) = Transaction::security_transfer(ACC, ACC2, AAPL, d(2024, 3, 1), dec!(10), "USD");
    let err = build_holdings(&[inc], "USD", &FakeRates::new()).unwrap_err();
    assert!(
        format!("{err}").contains("DELIVERY_INBOUND"),
        "hint about the disposal: {err}"
    );
}

/// A 2:1 split doubles quantity and halves unit cost:
/// 10 × 100 = 1000; 20 × 50 = 1000; 20 × 60 = 1200; unrealized = 200.
#[test]
fn split_doubles_quantity_and_halves_unit_cost() {
    let txs = vec![Transaction::buy(
        ACC,
        AAPL,
        d(2024, 1, 10),
        dec!(10),
        dec!(100),
        "USD",
    )];
    let splits = vec![CorporateAction::split(AAPL, d(2024, 6, 1), dec!(1), dec!(2))];

    let h = build_holdings_with(
        &txs,
        "USD",
        &FakeRates::new(),
        HoldingsOptions::default().with_corporate_actions(&splits),
    )
    .unwrap();

    let p = &h.positions[AAPL];
    assert_eq!(p.quantity, dec!(20));
    assert_eq!(
        p.cost_basis_base,
        dec!(1000),
        "a split neither creates nor destroys value"
    );
    assert_eq!(p.lots[0].cost_per_unit, dec!(50));

    let prices = FakePrices::new("USD").with(AAPL, d(2024, 7, 1), dec!(60));
    let v = value_holdings(&h, "USD", d(2024, 7, 1), &prices, &FakeRates::new()).unwrap();
    assert_eq!(v.securities_value_base, dec!(1200));
    assert_eq!(v.unrealized_pnl_base, dec!(200));
}

/// A split-day purchase uses the post-split price:
/// 10 × 100 = 1000; split → 20 shares; same-day 5 × 50 = 250; total cost 1250.
#[test]
fn trade_on_split_date_is_not_adjusted() {
    let txs = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 10), dec!(10), dec!(100), "USD"),
        Transaction::buy(ACC, AAPL, d(2024, 6, 1), dec!(5), dec!(50), "USD"),
    ];
    let splits = vec![CorporateAction::split(AAPL, d(2024, 6, 1), dec!(1), dec!(2))];

    let h = build_holdings_with(
        &txs,
        "USD",
        &FakeRates::new(),
        HoldingsOptions::default().with_corporate_actions(&splits),
    )
    .unwrap();

    assert_eq!(h.positions[AAPL].quantity, dec!(25));
    assert_eq!(h.positions[AAPL].cost_basis_base, dec!(1250));
}

/// A 10:1 reverse split uses the same mechanism in reverse:
/// buy 100 × 5 = 500 USD; consolidate → 10 × 50 USD, still 500.
#[test]
fn reverse_split_shrinks_quantity() {
    let txs = vec![Transaction::buy(
        ACC,
        AAPL,
        d(2024, 1, 10),
        dec!(100),
        dec!(5),
        "USD",
    )];
    let splits = vec![CorporateAction::split(AAPL, d(2024, 6, 1), dec!(10), dec!(1))];

    let h = build_holdings_with(
        &txs,
        "USD",
        &FakeRates::new(),
        HoldingsOptions::default().with_corporate_actions(&splits),
    )
    .unwrap();

    assert_eq!(h.positions[AAPL].quantity, dec!(10));
    assert_eq!(h.positions[AAPL].cost_basis_base, dec!(500));
    assert_eq!(h.positions[AAPL].lots[0].cost_per_unit, dec!(50));
}

/// FIFO and average cost differ: buys 1000 and 1200, then sell 10 × 150.
/// FIFO cost/gain = 1000/500; average cost/gain = 1100/400.
#[test]
fn fifo_and_average_cost_differ() {
    let txs = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 10), dec!(10), dec!(100), "USD"),
        Transaction::buy(ACC, AAPL, d(2024, 2, 10), dec!(10), dec!(120), "USD"),
        Transaction::sell(ACC, AAPL, d(2024, 3, 10), dec!(10), dec!(150), "USD"),
    ];

    let fifo = build_holdings_with(
        &txs,
        "USD",
        &FakeRates::new(),
        HoldingsOptions::default().with_cost_basis(CostBasisMethod::Fifo),
    )
    .unwrap();
    assert_eq!(fifo.realized_pnl_base, dec!(500));
    assert_eq!(fifo.positions[AAPL].cost_basis_base, dec!(1200));

    let average = build_holdings_with(
        &txs,
        "USD",
        &FakeRates::new(),
        HoldingsOptions::default().with_cost_basis(CostBasisMethod::AverageCost),
    )
    .unwrap();
    assert_eq!(average.realized_pnl_base, dec!(400));
    assert_eq!(average.positions[AAPL].cost_basis_base, dec!(1100));
    assert_eq!(
        average.positions[AAPL].lots.len(),
        1,
        "average cost always keeps a single lot"
    );
    assert_eq!(average.positions[AAPL].lots[0].cost_per_unit, dec!(110));
}

/// Externally received securities carry cost and external flow:
/// receive 10 for 1000 USD; value 1300 later; unrealized +300; flow +1000.
#[test]
fn delivery_inbound_sets_cost_and_counts_as_flow() {
    let txs = vec![Transaction::delivery_inbound(
        ACC,
        AAPL,
        d(2024, 2, 1),
        dec!(10),
        dec!(1000),
        "USD",
    )];

    let h = build_holdings(&txs, "USD", &FakeRates::new()).unwrap();
    assert_eq!(h.positions[AAPL].cost_basis_base, dec!(1000));
    assert_eq!(h.positions[AAPL].lots[0].cost_per_unit, dec!(100));
    assert_eq!(
        h.external_flows,
        vec![sq_core::calc::CashFlow {
            date: d(2024, 2, 1),
            amount_base: dec!(1000)
        }]
    );
    assert!(h.cash.get("USD").copied().unwrap_or_default().is_zero());

    let prices = FakePrices::new("USD").with(AAPL, d(2024, 7, 1), dec!(130));
    let v = value_holdings(&h, "USD", d(2024, 7, 1), &prices, &FakeRates::new()).unwrap();
    assert_eq!(v.unrealized_pnl_base, dec!(300));
}

/// Sending securities out realizes the result and removes their cost:
/// buy 10 × 100 = 1000; transfer at 1400; realized = 400; flow = −1400.
#[test]
fn delivery_outbound_realizes_and_removes_value() {
    let txs = vec![
        Transaction::buy(ACC, AAPL, d(2024, 1, 10), dec!(10), dec!(100), "USD"),
        Transaction::delivery_outbound(ACC, AAPL, d(2024, 3, 1), dec!(10), dec!(1400), "USD"),
    ];

    let h = build_holdings(&txs, "USD", &FakeRates::new()).unwrap();
    assert_eq!(h.realized_pnl_base, dec!(400));
    assert_eq!(h.positions[AAPL].quantity, Decimal::ZERO);
    assert_eq!(h.external_flows.last().unwrap().amount_base, dec!(-1400));
    // The transfer does not move cash; the purchase balance remains unchanged.
    assert_eq!(h.cash["USD"], dec!(-1000));
}

/// Interest, fees, taxes, and refunds are netted by operation type:
/// interest = 67; fees = 0; taxes = 20; cash = 47.
#[test]
fn interest_and_refunds() {
    let txs = vec![
        Transaction::cash(ACC, TransactionKind::Interest, d(2024, 1, 31), dec!(100), "USD")
            .with_taxes(dec!(13)),
        Transaction::cash(
            ACC,
            TransactionKind::InterestCharge,
            d(2024, 2, 28),
            dec!(20),
            "USD",
        ),
        Transaction::cash(ACC, TransactionKind::Fee, d(2024, 3, 1), dec!(15), "USD"),
        Transaction::cash(ACC, TransactionKind::FeeRefund, d(2024, 3, 15), dec!(15), "USD"),
        Transaction::cash(ACC, TransactionKind::Tax, d(2024, 4, 1), dec!(30), "USD"),
        Transaction::cash(ACC, TransactionKind::TaxRefund, d(2024, 5, 1), dec!(10), "USD"),
    ];

    let h = build_holdings(&txs, "USD", &FakeRates::new()).unwrap();
    assert_eq!(h.interest_base, dec!(67));
    assert_eq!(
        h.fees_base,
        Decimal::ZERO,
        "a refund cancels the fee instead of becoming income"
    );
    assert_eq!(h.taxes_base, dec!(20));
    assert_eq!(h.cash["USD"], dec!(47));

    let v = value_holdings(
        &h,
        "USD",
        d(2024, 6, 1),
        &FakePrices::new("USD"),
        &FakeRates::new(),
    )
    .unwrap();
    // 0 unrealized + 0 realized + 0 dividends + 67 interest − 0 fees − 20 taxes.
    assert_eq!(v.total_pnl_base(), dec!(47));
}
