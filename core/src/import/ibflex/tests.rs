use super::*;

const TWO_LEVELS: &str = r#"<FlexQueryResponse queryName="q" type="AF">
<FlexStatements count="1"><FlexStatement accountId="U123" fromDate="20240101" toDate="20240131">
<Trades>
<Trade accountId="U123" symbol="AAPL" isin="US0378331005" description="APPLE INC"
  tradeDate="20240115" quantity="5" tradePrice="100" proceeds="-500" ibCommission="-1"
  currency="USD" buySell="BUY" levelOfDetail="EXECUTION" multiplier="1" />
<Trade accountId="U123" symbol="AAPL" isin="US0378331005" description="APPLE INC"
  tradeDate="20240115" quantity="5" tradePrice="102" proceeds="-510" ibCommission="-1"
  currency="USD" buySell="BUY" levelOfDetail="EXECUTION" multiplier="1" />
<Trade accountId="U123" symbol="AAPL" isin="US0378331005" description="APPLE INC"
  tradeDate="20240115" quantity="10" tradePrice="101" proceeds="-1010" ibCommission="-2"
  currency="USD" buySell="BUY" levelOfDetail="ORDER" multiplier="1" />
</Trades></FlexStatement></FlexStatements></FlexQueryResponse>"#;

fn column(parsed: &ParsedCsv, row: usize, name: &str) -> String {
    parsed.value(row, name).unwrap_or_default().to_string()
}

#[test]
fn order_level_wins_over_its_executions() {
    let parsed = parse_flex(TWO_LEVELS.as_bytes()).unwrap();
    assert_eq!(parsed.rows.len(), 1);
    assert_eq!(column(&parsed, 0, QUANTITY), "10");
    assert_eq!(column(&parsed, 0, DATE), "2024-01-15");
    assert_eq!(column(&parsed, 0, AMOUNT), "-1010");
    assert_eq!(column(&parsed, 0, ACCOUNT), "U123");
}

#[test]
fn executions_are_kept_when_the_file_has_no_orders() {
    let only_fills = TWO_LEVELS.replace(r#"levelOfDetail="ORDER""#, r#"levelOfDetail="CLOSED_LOT""#);
    let parsed = parse_flex(only_fills.as_bytes()).unwrap();
    assert_eq!(parsed.rows.len(), 2);
}

#[test]
fn a_flex_file_is_recognised_and_a_csv_is_not() {
    assert!(is_flex(TWO_LEVELS.as_bytes()));
    assert!(!is_flex(b"date,type,amount\n2024-01-15,BUY,100\n"));
}

#[test]
fn dates_arrive_in_every_shape_the_web_office_offers() {
    assert_eq!(date("20240115"), "2024-01-15");
    assert_eq!(date("20240115;112233"), "2024-01-15");
    assert_eq!(date("2024-01-15"), "2024-01-15");
    assert_eq!(date("2024-01-15, 11:22:33"), "2024-01-15");
    assert_eq!(date(""), "");
}

#[test]
fn a_trade_without_proceeds_still_signs_its_amount() {
    let one = |attrs: &str| {
        let xml = format!(
            r#"<FlexQueryResponse><FlexStatements><FlexStatement accountId="U1">
                <Trades><Trade symbol="X" tradeDate="20240115" buySell="BUY" {attrs} />
                </Trades></FlexStatement></FlexStatements></FlexQueryResponse>"#
        );
        let parsed = parse_flex(xml.as_bytes()).unwrap();
        column(&parsed, 0, AMOUNT)
    };
    // tradeMoney is positive for a purchase and negative for a sale: the mirror of proceeds.
    assert_eq!(one(r#"quantity="4" tradePrice="25" tradeMoney="100""#), "-100");
    assert_eq!(one(r#"quantity="4" tradePrice="25" proceeds="-100""#), "-100");
    assert_eq!(one(r#"quantity="-4" tradePrice="25" tradeMoney="-100""#), "100");
}

#[test]
fn the_mapping_answers_every_wording_this_reader_writes() {
    let mapping = mapping();
    for invented in [CORPORATE_ACTION, DERIVATIVE_TRADE, CANCELLED_TRADE] {
        assert!(mapping.is_ignored(invented), "{invented} is not on the skip list");
    }
    assert_eq!(
        mapping.kind_of(CURRENCY_CONVERSION),
        Some(TransactionKind::TransferIn)
    );
    assert_eq!(
        mapping.kind_of("Deposits/Withdrawals"),
        Some(TransactionKind::Deposit)
    );
    assert!(mapping.missing_required().is_empty());
    for column in COLUMNS {
        assert!(
            mapping.columns.values().any(|c| c == column),
            "{column} is produced but mapped to nothing"
        );
    }
}

#[test]
fn an_empty_statement_is_an_error_rather_than_an_empty_preview() {
    let empty = r#"<FlexQueryResponse><FlexStatements><FlexStatement accountId="U1">
            </FlexStatement></FlexStatements></FlexQueryResponse>"#;
    assert!(parse_flex(empty.as_bytes()).is_err());
}
