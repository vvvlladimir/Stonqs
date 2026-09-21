use super::*;

/// Headers and rows the way a broker prints them, for the value-aware detection.
fn detect(headers: &[&str], rows: &[&[&str]]) -> ImportMapping {
    let headers: Vec<String> = headers.iter().map(|s| s.to_string()).collect();
    let rows: Vec<Vec<String>> = rows
        .iter()
        .map(|r| r.iter().map(|c| c.to_string()).collect())
        .collect();
    ImportMapping::detect_with_values(&headers, &rows)
}

#[test]
fn a_currency_column_is_not_the_instrument() {
    // Avanza's "Instrumentvaluta" contains "instrument", and its values are plainly codes.
    let mapping = detect(
        &[
            "Datum",
            "Värdepapper/beskrivning",
            "Transaktionsvaluta",
            "Instrumentvaluta",
        ],
        &[
            &["2025-01-14", "Storebrand", "SEK", "NOK"],
            &["2025-01-15", "Swedbank", "SEK", "SEK"],
        ],
    );
    assert_eq!(mapping.column(ImportField::Symbol), None);
    assert_eq!(mapping.column(ImportField::Currency), Some("Transaktionsvaluta"));
    assert_eq!(mapping.column(ImportField::Name), Some("Värdepapper/beskrivning"));
}

#[test]
fn a_word_buried_in_a_long_header_does_not_claim_the_column() {
    // Freetrade has no account column; "Price per Share in Account Currency" is not one.
    let mapping = detect(
        &["Timestamp", "Type", "Price per Share in Account Currency"],
        &[&["2024-04-18T18:09:12.259Z", "ORDER", "12.50"]],
    );
    assert_eq!(mapping.column(ImportField::Account), None);
    assert_eq!(mapping.column(ImportField::Date), Some("Timestamp"));
}

#[test]
fn the_field_that_names_a_column_best_keeps_the_others_off_it() {
    // eToro's "Asset type" is a kind column, so it is not the symbol column — even though
    // the kind is already mapped to "Type" and Symbol has nothing else to take.
    let mapping = detect(
        &["Date", "Type", "Asset type"],
        &[&["01/01/2024", "Interest Payment", "Stocks"]],
    );
    assert_eq!(mapping.column(ImportField::Kind), Some("Type"));
    assert_eq!(mapping.column(ImportField::Symbol), None);
}

#[test]
fn a_fee_column_has_to_hold_numbers() {
    // Bitvavo splits the fee into "Fee currency" and "Fee amount": the letters in the
    // first one must not be read as money.
    let mapping = detect(
        &["Date", "Type", "Amount", "Fee currency", "Fee amount"],
        &[&["2023-11-30", "buy", "10.00", "ETH", "0.02"]],
    );
    assert_eq!(mapping.column(ImportField::Fee), Some("Fee amount"));
    assert_eq!(mapping.column(ImportField::Currency), Some("Fee currency"));
}

#[test]
fn an_isin_column_is_found_by_its_values_alone() {
    // Parqet calls it "identifier"; no alias of ours does.
    let mapping = detect(
        &["date", "type", "identifier"],
        &[
            &["02.08.2024", "Buy", "LU2089238203"],
            &["03.08.2024", "Buy", "IE00B5BMR087"],
        ],
    );
    assert_eq!(mapping.column(ImportField::Isin), Some("identifier"));
}

#[test]
fn a_column_that_is_empty_everywhere_is_not_a_mapping() {
    // Investimental leaves "Settlement Date" blank and dates the row elsewhere.
    let mapping = detect(
        &["Settlement Date", "Side", "Update Time"],
        &[
            &["", "Buy", "2024-03-05 10:15:40"],
            &["", "Sell", "2024-03-06 11:02:11"],
        ],
    );
    assert_eq!(mapping.column(ImportField::Date), Some("Update Time"));
}

#[test]
fn broker_wording_is_recognised_by_keyword_in_any_language() {
    let mapping = detect(
        &["Datum", "Type"],
        &[
            &["2024-01-01", "Aankoop"],
            &["2024-01-02", "Verkoop"],
            &["2024-01-03", "Stocks/ETF purchase"],
            &["2024-01-04", "Withholding tax"],
            &["2024-01-05", "Custody Fee"],
            &["2024-01-06", "Uttag till konto"],
            &["2024-01-07", "Free funds interests"],
            &["2024-01-08", "Provento etf"],
        ],
    );
    assert_eq!(mapping.kind_of("Aankoop"), Some(TransactionKind::Buy));
    // "Verkoop" contains the "Koop" that means buy: the sell keywords go first.
    assert_eq!(mapping.kind_of("Verkoop"), Some(TransactionKind::Sell));
    assert_eq!(mapping.kind_of("Stocks/ETF purchase"), Some(TransactionKind::Buy));
    assert_eq!(mapping.kind_of("Withholding tax"), Some(TransactionKind::Tax));
    assert_eq!(mapping.kind_of("Custody Fee"), Some(TransactionKind::Fee));
    assert_eq!(
        mapping.kind_of("Uttag till konto"),
        Some(TransactionKind::Withdrawal)
    );
    assert_eq!(
        mapping.kind_of("Free funds interests"),
        Some(TransactionKind::Interest)
    );
    assert_eq!(mapping.kind_of("Provento etf"), Some(TransactionKind::Dividend));
}

#[test]
fn moving_value_inside_the_portfolio_is_one_keyword_for_both_legs() {
    // Crypto.com writes the pair as two rows of one wording, and the arrow forms of it
    // are a different string per pair — only the sign tells the legs apart.
    let mapping = detect(
        &["Timestamp (UTC)", "Transaction Kind"],
        &[
            &["2025-03-31 12:53:56", "Balance Conversion"],
            &["2025-01-07 11:43:48", "EGLD -> USDC"],
            &["2024-08-08 06:27:13", "USDT -> EUR"],
            &["2025-05-31 08:35:20", "crypto_to_exchange_transfer"],
            &["2024-01-16 05:50:24", "Cardholder CRO Stake"],
            &["2024-11-17 08:54:14", "Cardholder CRO Unstake"],
        ],
    );
    for value in [
        "Balance Conversion",
        "EGLD -> USDC",
        "USDT -> EUR",
        "crypto_to_exchange_transfer",
        "Cardholder CRO Stake",
        "Cardholder CRO Unstake",
    ] {
        // The reversible side: a negative amount turns it into TRANSFER_OUT.
        assert_eq!(
            mapping.kind_of(value),
            Some(TransactionKind::TransferIn),
            "{value}"
        );
    }
    assert_eq!(
        TransactionKind::TransferIn.reversed(),
        Some(TransactionKind::TransferOut)
    );
}

#[test]
fn income_that_is_neither_a_dividend_nor_interest_has_its_own_wording() {
    let mapping = detect(
        &["Timestamp (UTC)", "Type"],
        &[
            &["2024-08-08 06:28:12", "Card Cashback"],
            &["2024-01-15 04:02:52", "CRO Lockup Rewards"],
            &["2024-11-11 00:33:50", "Cardholder CRO Stake Reward"],
            &["2024-05-01 00:00:00", "referral_card_cashback"],
        ],
    );
    assert_eq!(mapping.kind_of("Card Cashback"), Some(TransactionKind::Cashback));
    assert_eq!(
        mapping.kind_of("referral_card_cashback"),
        Some(TransactionKind::Cashback)
    );
    assert_eq!(
        mapping.kind_of("CRO Lockup Rewards"),
        Some(TransactionKind::Reward)
    );
    // A reward beats the stake it is paid on: the keyword order decides, not the file.
    assert_eq!(
        mapping.kind_of("Cardholder CRO Stake Reward"),
        Some(TransactionKind::Reward)
    );
}

#[test]
fn a_wording_we_cannot_read_stays_unmapped() {
    // "Corporate action" could be a dividend or a split: guessing would forge the answer.
    let mapping = detect(&["Date", "Type"], &[&["02-Apr-2025", "Corporate action"]]);
    assert_eq!(mapping.kind_of("Corporate action"), None);
}

#[test]
fn detects_english_headers() {
    let headers: Vec<String> = ["date", "type", "symbol", "quantity", "unit_price", "fee"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mapping = ImportMapping::detect(&headers);
    assert_eq!(mapping.column(ImportField::Date), Some("date"));
    assert_eq!(mapping.column(ImportField::Kind), Some("type"));
    assert_eq!(mapping.column(ImportField::Price), Some("unit_price"));
    assert!(mapping.missing_required().is_empty());
}

#[test]
fn detects_german_headers() {
    let headers: Vec<String> = ["Datum", "Typ", "Symbol", "Stück", "Kurs", "Gebühr", "Währung"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mapping = ImportMapping::detect(&headers);
    assert_eq!(mapping.column(ImportField::Date), Some("Datum"));
    assert_eq!(mapping.column(ImportField::Quantity), Some("Stück"));
    assert_eq!(mapping.column(ImportField::Price), Some("Kurs"));
    assert_eq!(mapping.column(ImportField::Currency), Some("Währung"));
}

#[test]
fn one_column_is_not_assigned_twice() {
    let headers: Vec<String> = ["Datum", "Typ", "Kurs", "Wechselkurs"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let mapping = ImportMapping::detect(&headers);
    assert_eq!(mapping.column(ImportField::Price), Some("Kurs"));
    assert_eq!(mapping.column(ImportField::FxRate), Some("Wechselkurs"));
}

#[test]
fn exact_header_beats_a_substring_earlier_in_the_file() {
    let headers: Vec<String> = [
        "datetime",
        "date",
        "account_type",
        "category",
        "type",
        "asset_class",
        "name",
        "symbol",
        "shares",
        "price",
        "amount",
        "fee",
        "tax",
        "currency",
        "original_amount",
        "original_currency",
        "fx_rate",
        "description",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let mapping = ImportMapping::detect(&headers);
    assert_eq!(mapping.column(ImportField::Kind), Some("type"));
    assert_eq!(mapping.column(ImportField::Symbol), Some("symbol"));

    assert_eq!(mapping.column(ImportField::Date), Some("date"));

    assert_eq!(mapping.column(ImportField::Amount), Some("amount"));
    assert_eq!(mapping.column(ImportField::Currency), Some("currency"));

    assert_eq!(mapping.column(ImportField::Account), Some("account_type"));
}

#[test]
fn kind_aliases_ignore_case_and_separators() {
    let mapping = ImportMapping::detect(&[]);
    assert_eq!(mapping.kind_of("Transfer In"), Some(TransactionKind::TransferIn));
    assert_eq!(mapping.kind_of("kauf"), Some(TransactionKind::Buy));
    assert_eq!(mapping.kind_of("Umbuchung"), None);
}

#[test]
fn user_alias_wins_over_defaults() {
    let mapping = ImportMapping::detect(&[]).with_kind_alias("BUY", TransactionKind::DeliveryInbound);
    assert_eq!(mapping.kind_of("BUY"), Some(TransactionKind::DeliveryInbound));
}

#[test]
fn a_column_of_unique_identifiers_is_not_a_link() {
    // Trade Republic prints one id per row under a header a link column would also use.
    // Reading it as a link marks every transfer internal, and `calc` then sees a portfolio
    // with no deposits and no withdrawals at all.
    let mapping = detect(
        &["Date", "Type", "Reference", "Amount"],
        &[
            &["2025-06-10", "Incoming transfer", "019abf02-9caa-7bf2", "1000.00"],
            &["2025-06-13", "Outgoing transfer", "019ac99b-feed-75d5", "-250.00"],
        ],
    );
    assert_eq!(mapping.column(ImportField::LinkId), None);
}

#[test]
fn a_column_whose_values_repeat_is_the_link_it_claims_to_be() {
    // The two legs of one move carry one value; that repetition is the whole claim.
    let mapping = detect(
        &["Date", "Type", "Reference", "Amount"],
        &[
            &["2025-06-10", "Balance Conversion", "move-7", "-100.00"],
            &["2025-06-10", "Balance Conversion", "move-7", "94.20"],
        ],
    );
    assert_eq!(mapping.column(ImportField::LinkId), Some("Reference"));
}
