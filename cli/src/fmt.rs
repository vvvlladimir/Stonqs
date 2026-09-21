//! Printing. The core never writes to a terminal, so every line the CLI shows is composed here.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use sq_core::calc::AllocationBucket;
use sq_core::import::{ImportPreview, RowStatus};
use sq_core::model::TransactionKind;

/// Round to two decimals for display only; calculations keep full precision.
pub fn money(v: Decimal) -> String {
    format!("{:.2}", v)
}

pub fn percent(v: Decimal) -> String {
    format!("{:.2}%", v * dec!(100))
}

/// Format statistics calculated as `f64`, such as volatility and Sharpe.
pub fn percent_f64(v: f64) -> String {
    format!("{:.2}%", v * 100.0)
}

pub fn print_buckets(buckets: &[AllocationBucket], depth: usize) {
    for b in buckets {
        println!(
            "{:indent$}{:<24} {:>14} {:>9}",
            "",
            b.label,
            money(b.value_base),
            percent(b.weight),
            indent = depth * 2
        );
        print_buckets(&b.children, depth + 1);
    }
}

pub fn print_preview(preview: &ImportPreview) {
    println!(
        "{:<4} {:<12} {:<12} {:<18} {:>8} {:>12} instrument",
        "#", "status", "date", "kind", "qty", "amount"
    );
    println!("{}", "-".repeat(78));
    for row in &preview.rows {
        let status = match row.status {
            RowStatus::Ready => "ready",
            RowStatus::Duplicate => "duplicate",
            RowStatus::UnknownSecurity => "no instrument",
            RowStatus::Ignored => "skipped",
            RowStatus::Invalid => "error",
        };
        match &row.draft {
            Some(d) => println!(
                "{:<4} {:<12} {:<12} {:<18} {:>8} {:>12} {}",
                row.number,
                status,
                d.date,
                short_kind(d.kind),
                d.quantity,
                money(d.amount),
                d.symbol.as_deref().unwrap_or("—"),
            ),
            None => println!("{:<4} {:<12} row not parsed", row.number, status),
        }
        for problem in &row.problems {
            println!("     ! {}", problem.message);
        }
    }
    println!("{}", "-".repeat(78));
    let s = &preview.summary;
    println!(
        "total {}, ready {}, duplicates {}, without an instrument {}, skipped {}, with errors {}",
        s.total, s.ready, s.duplicates, s.unknown_securities, s.ignored, s.invalid
    );
    if !preview.unknown_kinds().is_empty() {
        println!(
            "unknown transaction kinds: {}",
            preview.unknown_kinds().join(", ")
        );
    }
    if !preview.unknown_symbols().is_empty() {
        println!(
            "instruments not in the database: {}",
            preview.unknown_symbols().join(", ")
        );
    }
    for problem in &preview.problems {
        println!("! {}", problem.message);
    }
}

/// Format a transaction kind for compact table output.
pub fn short_kind(kind: TransactionKind) -> String {
    kind.as_str().to_lowercase().replace('_', " ")
}
