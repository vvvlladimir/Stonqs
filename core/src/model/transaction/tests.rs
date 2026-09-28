use super::*;
use rust_decimal_macros::dec;

#[test]
fn cash_sign_matches_cash_delta() {
    for &kind in TransactionKind::ALL {
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
    for &kind in TransactionKind::ALL {
        let Some(back) = kind.reversed() else {
            assert!(matches!(
                kind,
                TransactionKind::Buy
                    | TransactionKind::Sell
                    | TransactionKind::Dividend
                    | TransactionKind::Cashback
                    | TransactionKind::Reward
            ));
            continue;
        };
        assert_eq!(back.reversed(), Some(kind), "{kind:?}");
        if kind.cash_sign() != 0 {
            assert_eq!(back.cash_sign(), -kind.cash_sign(), "{kind:?}");
        }
    }
}

#[test]
fn every_kind_parses_back_from_its_name() {
    for &kind in TransactionKind::ALL {
        assert_eq!(TransactionKind::parse(kind.as_str()).unwrap(), kind);
    }
    assert!(TransactionKind::parse("BUYY").is_err());
}

/// `Decimal` panics on overflow rather than saturating, so a figure large enough to overflow
/// the first product taken of it must never reach the ledger: once stored, every screen that
/// values the portfolio meets it again.
#[test]
fn a_figure_too_large_to_calculate_with_is_refused() {
    let base = Transaction::cash(
        "acc",
        TransactionKind::Deposit,
        "2024-01-02".parse().unwrap(),
        dec!(100),
        "USD",
    );

    assert!(base.validate().is_ok());
    assert!(
        Transaction {
            amount: Decimal::MAX,
            ..base.clone()
        }
        .validate()
        .is_err()
    );
    // A quantity of a billion tokens is ordinary in crypto and stays allowed.
    assert!(
        Transaction {
            quantity: dec!(1000000000),
            price: dec!(0.00001),
            ..base
        }
        .validate()
        .is_ok()
    );
}
