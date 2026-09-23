use super::*;

/// Risk statistics use numbers; monetary values remain strings.
#[test]
fn risk_metrics_cross_as_numbers_not_strings() {
    use sq_core::calc::{Drawdown, RiskMetrics};

    let json = serde_json::to_value(RiskMetrics {
        days: 120,
        volatility: 0.183,
        semi_deviation: 0.121,
        max_drawdown: Some(Drawdown {
            peak: chrono::NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(),
            trough: chrono::NaiveDate::from_ymd_opt(2024, 8, 5).unwrap(),
            recovered: None,
            depth: -0.12,
        }),
        max_drawdown_days: Some(35),
        longest_drawdown: None,
        longest_drawdown_days: None,
        current_drawdown: -0.04,
        current_drawdown_since: Some(chrono::NaiveDate::from_ymd_opt(2024, 7, 1).unwrap()),
        sharpe: Some(0.74),
        annualized_return: 0.17,
        best_day: Some((chrono::NaiveDate::from_ymd_opt(2024, 7, 3).unwrap(), 0.021)),
        worst_day: None,
        positive_days_share: 0.54,
    })
    .unwrap();

    assert!(json["volatility"].is_number());
    assert!(json["max_drawdown"]["depth"].is_number());
    assert_eq!(json["max_drawdown"]["recovered"], Value::Null);
    // Durations are day counts, not dates; the standing drawdown is a plain number.
    assert!(json["max_drawdown_days"].is_number());
    assert!(json["current_drawdown"].is_number());
    assert_eq!(json["current_drawdown_since"], Value::String("2024-07-01".into()));
    // Tuples serialize as `[string, number]`.
    assert!(json["best_day"].is_array());
    assert_eq!(json["best_day"][0], Value::String("2024-07-03".into()));
    assert!(json["best_day"][1].is_number());
}

/// Risk series use numeric statistics and string growth values.
#[test]
fn risk_and_growth_series_keep_their_two_conventions() {
    use sq_core::calc::{GrowthSeries, StatSeries};

    let mut stat = StatSeries::default();
    stat.push(chrono::NaiveDate::from_ymd_opt(2024, 6, 4).unwrap(), 0.0123);
    let stat: Value = serde_json::to_value(stat).unwrap();
    assert_eq!(keys(&stat), ["dates", "values"]);
    assert!(stat["values"][0].is_number());
    assert_eq!(stat["dates"][0], "2024-06-04");

    let mut growth = GrowthSeries::default();
    growth.push(chrono::NaiveDate::from_ymd_opt(2024, 6, 4).unwrap(), dec!(1.0123));
    let growth: Value = serde_json::to_value(growth).unwrap();
    assert_eq!(keys(&growth), ["dates", "values"]);
    assert_eq!(growth["values"][0], "1.0123");
}

/// Risk report is returned as one screen-level object.
#[test]
fn risk_report_keys_match_the_typescript_types() {
    use rust_decimal::Decimal;
    use sq_core::calc::ValueSeries;
    use sq_core::calc::{RiskReport, risk_report};

    let d = |day: u32| chrono::NaiveDate::from_ymd_opt(2024, 6, day).unwrap();
    let series = ValueSeries {
        base_currency: "EUR".into(),
        dates: vec![d(3), d(4), d(5)],
        total_value_base: vec![dec!(100), dec!(110), dec!(99)],
        external_flow_base: vec![Decimal::ZERO, Decimal::ZERO, Decimal::ZERO],
    };
    let report: RiskReport = risk_report(&series, 0.02, 2);
    let json: Value = serde_json::to_value(report).unwrap();

    assert_eq!(
        keys(&json),
        [
            "drawdown",
            "episodes",
            "metrics",
            "returns",
            "rolling_volatility",
            "window_days",
        ]
    );
    assert_eq!(
        keys(&json["episodes"][0]),
        ["depth", "peak", "recovered", "trough"]
    );
    assert!(json["metrics"]["volatility"].is_number());
}

/// Performance data keeps the value series and derived returns together.
#[test]
fn performance_payload_carries_the_curve_and_the_period_returns() {
    use rust_decimal::Decimal;
    use sq_app_lib::commands::performance::PerformanceData;
    use sq_core::calc::{
        ChargeSummary, GrowthSeries, Peak, PeriodReturn, PeriodSummary, TradingVolume, ValueSeries,
    };

    let d = |day: u32| chrono::NaiveDate::from_ymd_opt(2024, 6, day).unwrap();
    let data = PerformanceData {
        from: "2024-06-03".into(),
        to: "2024-06-05".into(),
        base_currency: "EUR".into(),
        twr: dec!(-0.01),
        twr_annualized: Some(dec!(-0.7770)),
        xirr: None,
        summary: PeriodSummary {
            start_value_base: dec!(100),
            end_value_base: dec!(99),
            net_flow_base: Decimal::ZERO,
            absolute_change_base: dec!(-1),
            delta_base: dec!(-1),
            invested_capital_base: dec!(100),
            average_capital_base: dec!(100),
        },
        costs: ChargeSummary {
            count: 1,
            fees_base: dec!(0.92),
            taxes_base: Decimal::ZERO,
        },
        fee_rate: Some(dec!(0.0092)),
        tax_rate: Some(Decimal::ZERO),
        volume: TradingVolume {
            bought_base: dec!(500),
            sold_base: dec!(200),
            volume_base: dec!(700),
            trades: 2,
        },
        turnover_rate: Some(dec!(7)),
        peak: Some(Peak {
            date: d(3),
            value: dec!(100),
            current: dec!(99),
            distance: Some(dec!(-0.01)),
            days_since: 2,
        }),
        series: ValueSeries {
            base_currency: "EUR".into(),
            dates: vec![d(3)],
            total_value_base: vec![dec!(100)],
            external_flow_base: vec![Decimal::ZERO],
        },
        growth: GrowthSeries {
            dates: vec![d(3)],
            values: vec![Decimal::ONE],
        },
        monthly_returns: vec![PeriodReturn {
            from: d(1),
            to: d(30),
            twr: dec!(-0.01),
        }],
        annual_returns: Vec::new(),
    };
    let json: Value = serde_json::to_value(data).unwrap();

    assert_eq!(
        keys(&json),
        [
            "annual_returns",
            "base_currency",
            "costs",
            "fee_rate",
            "from",
            "growth",
            "monthly_returns",
            "peak",
            "series",
            "summary",
            "tax_rate",
            "to",
            "turnover_rate",
            "twr",
            "twr_annualized",
            "volume",
            "xirr",
        ]
    );
    assert_eq!(
        keys(&json["summary"]),
        [
            "absolute_change_base",
            "average_capital_base",
            "delta_base",
            "end_value_base",
            "invested_capital_base",
            "net_flow_base",
            "start_value_base",
        ]
    );
    // Money stays a string; a count is a number.
    assert!(json["summary"]["delta_base"].is_string());
    assert!(json["costs"]["fees_base"].is_string());
    assert!(json["costs"]["count"].is_number());
    assert_eq!(keys(&json["monthly_returns"][0]), ["from", "to", "twr"]);
    // Period returns are strings because they participate in decimal chaining.
    assert!(json["monthly_returns"][0]["twr"].is_string());
}

/// The calculation sheet keeps money as strings and names its chunk by two dates, so the
/// frontend never derives a month from an index.
#[test]
fn calculation_sheet_rows_carry_their_own_window_and_string_money() {
    use sq_core::calc::{CalculationRow, CalculationSheet, ChargeSummary, Period, PeriodSummary};

    let row = CalculationRow {
        from: chrono::NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
        to: chrono::NaiveDate::from_ymd_opt(2024, 2, 29).unwrap(),
        start_value_base: dec!(1100),
        external_flow_base: dec!(0),
        income_base: dec!(50),
        costs: ChargeSummary {
            count: 1,
            fees_base: dec!(0),
            taxes_base: dec!(5),
        },
        costs_base: dec!(5),
        market_change_base: dec!(0),
        delta_base: dec!(45),
        end_value_base: dec!(1145),
        twr: dec!(0.040909),
        cumulative_twr: dec!(0.145),
    };
    let sheet = CalculationSheet {
        base_currency: "EUR".into(),
        period: Period::Month,
        rows: vec![row],
        total: PeriodSummary {
            start_value_base: dec!(0),
            end_value_base: dec!(1145),
            net_flow_base: dec!(1000),
            absolute_change_base: dec!(1145),
            delta_base: dec!(145),
            invested_capital_base: dec!(1000),
            average_capital_base: dec!(1000),
        },
        twr: dec!(0.145),
    };

    let json = serde_json::to_value(&sheet).unwrap();
    assert_eq!(keys(&json), ["base_currency", "period", "rows", "total", "twr"]);
    // The granularity crosses as the core's own enum spelling, not as a frontend word.
    assert_eq!(json["period"], Value::String("MONTH".into()));
    assert_eq!(json["twr"], Value::String("0.145".into()));

    let row = &json["rows"][0];
    assert_eq!(
        keys(row),
        [
            "costs",
            "costs_base",
            "cumulative_twr",
            "delta_base",
            "end_value_base",
            "external_flow_base",
            "from",
            "income_base",
            "market_change_base",
            "start_value_base",
            "to",
            "twr",
        ]
    );
    assert_eq!(row["from"], Value::String("2024-02-01".into()));
    assert_eq!(row["to"], Value::String("2024-02-29".into()));
    assert_eq!(row["start_value_base"], Value::String("1100".into()));
    assert_eq!(row["costs"]["taxes_base"], Value::String("5".into()));
    assert_eq!(row["costs_base"], Value::String("5".into()));
    // The count of charges is a number; the money beside it is not.
    assert!(row["costs"]["count"].is_number());
}

/// A goal's reading crosses with money as strings and its unanswerable questions as `null` —
/// "cannot tell" is not "behind", and the frontend must be able to tell them apart.
#[test]
fn goal_progress_keeps_its_absent_answers_null() {
    use sq_core::calc::GoalProgress;

    let json = serde_json::to_value(GoalProgress {
        goal_id: "goal-1".into(),
        name: "House".into(),
        target_base: dec!(40000),
        current_base: dec!(17000),
        missing_base: dec!(23000),
        progress: dec!(0.425),
        months_left: Some(18),
        required_monthly_base: Some(dec!(1277.78)),
        months_to_target: None,
        projected_date: None,
        on_track: None,
        monthly_base: None,
        expected_return: dec!(0),
    })
    .unwrap();

    assert_eq!(json["target_base"], Value::String("40000".into()));
    assert_eq!(json["progress"], Value::String("0.425".into()));
    // A month count is a number, not money.
    assert!(json["months_left"].is_number());
    assert_eq!(json["on_track"], Value::Null);
    assert_eq!(json["monthly_base"], Value::Null);
    assert_eq!(json["required_monthly_base"], Value::String("1277.78".into()));
}

/// A limit's year is two dates it carries, so the frontend never derives one from the other.
#[test]
fn limit_usage_carries_its_own_year() {
    use sq_core::calc::LimitUsage;

    let json = serde_json::to_value(LimitUsage {
        limit_id: "limit-1".into(),
        account_id: "acc-1".into(),
        name: "ISA".into(),
        from: chrono::NaiveDate::from_ymd_opt(2025, 4, 6).unwrap(),
        to: chrono::NaiveDate::from_ymd_opt(2026, 4, 5).unwrap(),
        allowance: dec!(20000),
        used: dec!(12000),
        remaining: dec!(8000),
        share: dec!(0.6),
        currency: "GBP".into(),
    })
    .unwrap();

    assert_eq!(
        keys(&json),
        [
            "account_id",
            "allowance",
            "currency",
            "from",
            "limit_id",
            "name",
            "remaining",
            "share",
            "to",
            "used",
        ]
    );
    assert_eq!(json["from"], Value::String("2025-04-06".into()));
    assert_eq!(json["to"], Value::String("2026-04-05".into()));
    assert_eq!(json["used"], Value::String("12000".into()));
}
