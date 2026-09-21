use super::*;
use chrono::NaiveDate;
use sq_core::calc::expected_dividends;
use sq_core::model::SecurityEvent;

const SHELL: &str = "sec-shell";
const SILENT: &str = "sec-silent";

fn reported(security_id: &str, date: NaiveDate, amount: Decimal, currency: &str) -> SecurityEvent {
    SecurityEvent::dividend(security_id, date, amount, currency, "yahoo")
}

/// A quarterly USD payer held as 10 shares in a EUR portfolio, as of 2024-11-10.
///
/// Reported ex-dates: Feb 9, May 10, Aug 12 at 0.24 and Nov 8 at 0.25 — a raise.
/// Received: Feb 15, May 16, Aug 15 → lags 6, 6, 3 days, median 6.
/// The last one received was 2.40 USD gross, 0.36 tax → net share 2.04 / 2.40 = 0.85.
///
/// - Nov 8 is reported and not yet received: due Nov 14, 0.25 × 10 = 2.50 USD × 0.92 = 2.30 EUR,
///   net 2.30 × 0.85 = 1.955 → 1.96.
/// - Feb 9 and May 10 next year carry the latest amount, 0.25, not the 0.24 paid then:
///   due Feb 15 and May 16 2025, 2.30 EUR each.
/// - Aug 12 2025 is due Aug 18, past the window's end of May 31: not listed.
#[test]
fn a_quarterly_payer_carries_its_latest_amount_forward_from_the_reported_dates() {
    let txs = vec![
        Transaction::buy(ACC, AAPL, d(2023, 1, 10), dec!(10), dec!(150), "USD").with_fx_rate(dec!(0.90)),
        Transaction::dividend(ACC, AAPL, d(2024, 2, 15), dec!(2.40), "USD")
            .with_taxes(dec!(0.36))
            .with_fx_rate(dec!(0.90)),
        Transaction::dividend(ACC, AAPL, d(2024, 5, 16), dec!(2.40), "USD")
            .with_taxes(dec!(0.36))
            .with_fx_rate(dec!(0.90)),
        Transaction::dividend(ACC, AAPL, d(2024, 8, 15), dec!(2.40), "USD")
            .with_taxes(dec!(0.36))
            .with_fx_rate(dec!(0.90)),
    ];
    let holdings = build_holdings(&txs, "EUR", &FakeRates::new()).unwrap();
    let events = vec![
        reported(AAPL, d(2024, 2, 9), dec!(0.24), "USD"),
        reported(AAPL, d(2024, 5, 10), dec!(0.24), "USD"),
        reported(AAPL, d(2024, 8, 12), dec!(0.24), "USD"),
        reported(AAPL, d(2024, 11, 8), dec!(0.25), "USD"),
    ];
    let rates = FakeRates::new().with("USD", "EUR", d(2024, 11, 1), dec!(0.92));

    let rows = expected_dividends(
        &holdings,
        &events,
        &holdings.income,
        DateRange::new(d(2024, 11, 10), d(2025, 5, 31)),
        "EUR",
        &rates,
    )
    .unwrap();

    let dates: Vec<_> = rows.iter().map(|r| r.pay_date.unwrap()).collect();
    assert_eq!(dates, vec![d(2024, 11, 14), d(2025, 2, 15), d(2025, 5, 16)]);
    assert!(rows[0].reported, "Nov 8 is the source's own record");
    assert!(!rows[1].reported && !rows[2].reported);
    assert_eq!(rows[1].ex_date, d(2025, 2, 9));
    assert_eq!(rows[1].frequency, DividendFrequency::Quarterly);
    for row in &rows {
        assert_eq!(row.per_share, dec!(0.25));
        assert_eq!(row.gross_base, dec!(2.30));
        assert_eq!(row.net_base, Some(dec!(1.96)));
    }
}

/// A half-yearly EUR payer, 100 shares, as of 2024-12-01, nothing received yet.
///
/// Reported: 2023-05-05 1.00 (final), 2023-10-06 0.50 (interim), 2024-05-03 1.10,
/// 2024-06-01 5.00 (special), 2024-10-04 0.55.
/// Gaps 154, 210, 29, 125 → median (125 + 154) / 2 = 139 days → half-yearly.
/// The year up to 2024-10-04 holds 0.50, 1.10, 5.00, 0.55; median 1.10, 5.00 > 3 × 1.10 is special.
///
/// Each payment keeps its own amount a year on: 2025-05-03 1.10 × 100 = 110 EUR and
/// 2025-10-04 0.55 × 100 = 55 EUR. The 2023 interim projects onto 2024-10-06 and 2025-10-06,
/// both within half a gap of a payment already there, so it adds nothing. No payment received,
/// so no pay date and no net.
///
/// A second instrument last paid 2023-06-01 on a quarterly schedule: 18 months of silence is more
/// than two gaps, so it is a suspended dividend, not a late one — nothing forecast.
#[test]
fn a_half_yearly_payer_repeats_interim_and_final_and_drops_the_special() {
    let txs = vec![
        Transaction::buy(ACC, SHELL, d(2023, 1, 10), dec!(100), dec!(20), "EUR"),
        Transaction::buy(ACC, SILENT, d(2023, 1, 10), dec!(5), dec!(100), "EUR"),
    ];
    let holdings = build_holdings(&txs, "EUR", &FakeRates::new()).unwrap();
    let events = vec![
        reported(SHELL, d(2023, 5, 5), dec!(1.00), "EUR"),
        reported(SHELL, d(2023, 10, 6), dec!(0.50), "EUR"),
        reported(SHELL, d(2024, 5, 3), dec!(1.10), "EUR"),
        reported(SHELL, d(2024, 6, 1), dec!(5.00), "EUR"),
        reported(SHELL, d(2024, 10, 4), dec!(0.55), "EUR"),
        reported(SILENT, d(2022, 12, 1), dec!(1), "EUR"),
        reported(SILENT, d(2023, 3, 1), dec!(1), "EUR"),
        reported(SILENT, d(2023, 6, 1), dec!(1), "EUR"),
    ];

    let rows = expected_dividends(
        &holdings,
        &events,
        &holdings.income,
        DateRange::new(d(2024, 12, 1), d(2025, 12, 31)),
        "EUR",
        &FakeRates::new(),
    )
    .unwrap();

    let got: Vec<_> = rows
        .iter()
        .map(|r| (r.ex_date, r.per_share, r.gross_base))
        .collect();
    assert_eq!(
        got,
        vec![
            (d(2025, 5, 3), dec!(1.10), dec!(110.00)),
            (d(2025, 10, 4), dec!(0.55), dec!(55.00)),
        ]
    );
    assert!(rows.iter().all(|r| r.security_id == SHELL));
    assert!(rows.iter().all(|r| r.frequency == DividendFrequency::SemiAnnual));
    assert!(rows.iter().all(|r| r.pay_date.is_none() && r.net_base.is_none()));
}
