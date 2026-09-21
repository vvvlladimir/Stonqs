use super::*;

/// Taxonomy members include distinct monetary and weight fields.
#[test]
fn node_member_keys_match_the_typescript_types() {
    use sq_core::calc::{NodeMember, SubjectKind};

    let member = NodeMember {
        subject_id: "sec-1".into(),
        kind: SubjectKind::Security,
        symbol: "IWDA.L".into(),
        name: "iShares Core MSCI World".into(),
        value_base: dec!(1000),
        weight: dec!(0.142857142857142857142857143),
        subject_value_base: dec!(2000),
        assigned_share: dec!(0.5),
        excluded: false,
    };
    let json: Value = serde_json::to_value(member).unwrap();

    assert_eq!(
        keys(&json),
        [
            "assigned_share",
            "excluded",
            "kind",
            "name",
            "subject_id",
            "subject_value_base",
            "symbol",
            "value_base",
            "weight",
        ]
    );
    assert_eq!(json["value_base"], "1000");
    assert_eq!(json["subject_value_base"], "2000");
    assert_eq!(json["assigned_share"], "0.5");
    // Cash subjects use a composite key and have no security card.
    assert_eq!(json["kind"], "SECURITY");

    let cash = NodeMember {
        subject_id: sq_core::model::cash_subject_key("acc-1", "eur"),
        kind: SubjectKind::Cash,
        symbol: "EUR".into(),
        name: "Trade Republic".into(),
        value_base: dec!(500),
        weight: dec!(0),
        subject_value_base: dec!(500),
        assigned_share: dec!(1),
        excluded: true,
    };
    let json: Value = serde_json::to_value(cash).unwrap();
    assert_eq!(json["kind"], "CASH");
    assert_eq!(json["subject_id"], "cash:acc-1:EUR");
    assert_eq!(json["excluded"], Value::Bool(true));
}

/// One classification list serves both securities and cash subjects.
#[test]
fn assignment_keys_match_the_typescript_types() {
    use sq_core::calc::Assignment;
    use sq_core::model::{CashClassification, SecurityClassification};

    let security = Assignment::from(&SecurityClassification::new("sec-1", "node-1", dec!(0.6)));
    let json: Value = serde_json::to_value(&security).unwrap();
    assert_eq!(keys(&json), ["node_id", "subject_id", "weight"]);
    assert_eq!(json["weight"], Value::String("0.6".into()));

    let cash = Assignment::from(&CashClassification::new("acc-1", "eur", "node-2", dec!(1)));
    let json: Value = serde_json::to_value(&cash).unwrap();
    assert_eq!(json["subject_id"], "cash:acc-1:EUR");
}

/// Rebalance rows expose portfolio and parent-relative target weights.
#[test]
fn rebalance_item_keys_match_the_typescript_types() {
    use sq_core::calc::{CashDeposit, RebalanceItem, RebalanceTrade};

    let item = RebalanceItem {
        node_id: "node-core".into(),
        label: "Core".into(),
        current_base: dec!(5000),
        current_weight: dec!(0.5),
        target_weight: dec!(0.48),
        relative_target_weight: dec!(0.8),
        relative_current_weight: dec!(0.8333),
        target_base: dec!(4800),
        drift_base: dec!(-200),
        drift_weight: dec!(-0.02),
        leaf: true,
        trades: vec![RebalanceTrade {
            security_id: "sec-1".into(),
            symbol: "IWDA.L".into(),
            quantity: dec!(-2),
            estimated_base: dec!(-200),
            price_base: dec!(100),
            weight_after: dec!(0.48),
        }],
        deposits: vec![CashDeposit {
            subject_id: "cash:acc-1:EUR".into(),
            account_id: "acc-1".into(),
            currency: "EUR".into(),
            label: "Bank".into(),
            current_base: dec!(46),
            amount_base: dec!(54),
            weight_after: dec!(0.1),
        }],
    };
    let json: Value = serde_json::to_value(&item).unwrap();

    assert_eq!(
        keys(&json),
        [
            "current_base",
            "current_weight",
            "deposits",
            "drift_base",
            "drift_weight",
            "label",
            "leaf",
            "node_id",
            "relative_current_weight",
            "relative_target_weight",
            "target_base",
            "target_weight",
            "trades",
        ]
    );
    assert_eq!(json["target_weight"], Value::String("0.48".into()));
    assert_eq!(json["relative_target_weight"], Value::String("0.8".into()));
    assert_eq!(json["leaf"], Value::Bool(true));

    assert_eq!(
        keys(&json["trades"][0]),
        [
            "estimated_base",
            "price_base",
            "quantity",
            "security_id",
            "symbol",
            "weight_after",
        ]
    );
    assert_eq!(
        keys(&json["deposits"][0]),
        [
            "account_id",
            "amount_base",
            "currency",
            "current_base",
            "label",
            "subject_id",
            "weight_after",
        ]
    );
    assert_eq!(json["deposits"][0]["amount_base"], Value::String("54".into()));
    assert_eq!(json["trades"][0]["price_base"], Value::String("100".into()));
    assert_eq!(json["trades"][0]["weight_after"], Value::String("0.48".into()));
}
