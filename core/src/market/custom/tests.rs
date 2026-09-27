use super::*;
use crate::model::SecurityKind;
use rust_decimal_macros::dec;

fn source(format: CustomFormat) -> CustomProvider {
    CustomProvider::new(
        CustomSource {
            id: "custom:feed".into(),
            label: "Feed".into(),
            role: CustomRole::Quotes,
            url: "https://example.com/{SYMBOL}?from={FROM}&to={TO}&t={FROM:%s}&k={KEY}".into(),
            headers: vec![],
            format,
            date_format: None,
            factor: None,
            currency: None,
        },
        Some("secret".into()),
    )
}

fn json(date: &str, close: &str) -> CustomProvider {
    source(CustomFormat::Json {
        date_path: date.into(),
        close_path: close.into(),
    })
}

fn d(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

#[test]
fn fills_the_url_template() {
    let sec = Security::new("A B", "A", "EUR", SecurityKind::Etf).with_source("custom:feed", "A B");
    let url = json("$", "$").fill(
        &json("$", "$").def.url,
        &sec,
        DateRange::new(d("2024-06-03"), d("2024-06-07")),
    );
    assert_eq!(
        url,
        "https://example.com/A%20B?from=2024-06-03&to=2024-06-07&t=1717372800&k=secret"
    );
}

#[test]
fn zips_a_list_of_objects_newest_first_into_oldest_first() {
    let body =
        r#"{"values":[{"datetime":"2024-06-04","close":"101.5"},{"datetime":"2024-06-03","close":"100"}]}"#;
    let rows = json("$.values[*].datetime", "$.values[*].close")
        .parse(body)
        .unwrap();
    assert_eq!(
        rows,
        vec![(d("2024-06-03"), dec!(100)), (d("2024-06-04"), dec!(101.5))]
    );
}

#[test]
fn zips_parallel_arrays_with_unix_seconds() {
    // 1717372800 = 2024-06-03T00:00Z, as Yahoo's chart API prints its timestamps.
    let body = r#"{"chart":{"result":[{"timestamp":[1717372800],"close":[194.03]}]}}"#;
    let rows = json("$.chart.result[0].timestamp[*]", "$.chart.result[0].close[*]")
        .parse(body)
        .unwrap();
    assert_eq!(rows, vec![(d("2024-06-03"), dec!(194.03))]);
}

#[test]
fn reads_csv_columns_by_name() {
    let feed = source(CustomFormat::Csv {
        date_column: "Date".into(),
        close_column: "Close".into(),
    });
    let rows = feed
        .parse("Date,Open,Close\n2024-06-03,1,10.5\n2024-06-04,1,11\n")
        .unwrap();
    assert_eq!(rows[1], (d("2024-06-04"), dec!(11)));
}

#[test]
fn a_path_that_finds_nothing_is_the_source_s_fault_not_an_empty_week() {
    let err = json("$.values[*].date", "$.values[*].close").parse(r#"{"values":[{"date":"x","close":"y"}]}"#);
    assert!(matches!(err, Err(Error::BadProviderData { .. })));
}

#[test]
fn a_definition_must_be_https_and_name_both_paths() {
    let mut def = json("$[*].d", "$[*].c").def;
    assert_eq!(def.validate(), Ok(()));
    def.url = "http://example.com".into();
    assert_eq!(def.validate(), Err("not_https"));
}

#[test]
#[ignore = "requires network"]
fn reads_a_real_feed_described_as_data() {
    let feed = CustomProvider::new(
        CustomSource {
            id: "custom:eod".into(),
            label: "EOD".into(),
            role: CustomRole::Quotes,
            url: "https://eodhd.com/api/eod/{SYMBOL}?from={FROM}&to={TO}&fmt=json&api_token={KEY}".into(),
            headers: vec![],
            format: CustomFormat::Json {
                date_path: "$[*].date".into(),
                close_path: "$[*].close".into(),
            },
            date_format: None,
            factor: None,
            currency: Some("USD".into()),
        },
        Some("demo".into()),
    );
    let aapl =
        Security::new("AAPL", "Apple", "USD", SecurityKind::Stock).with_source("custom:eod", "AAPL.US");
    let quotes = feed
        .fetch(&aapl, DateRange::new(d("2024-06-03"), d("2024-06-07")))
        .unwrap();
    assert_eq!((quotes.len(), quotes[0].close), (5, dec!(194.03)));
}

#[test]
fn a_pair_template_names_both_currencies() {
    let mut feed = json("$", "$");
    feed.def.url = "https://fx.example/{BASE}{QUOTE}?from={FROM}".into();
    let url = feed.fill_pair("EUR", "ARS", DateRange::new(d("2024-06-03"), d("2024-06-07")));
    assert_eq!(url, "https://fx.example/EURARS?from=2024-06-03");
}
