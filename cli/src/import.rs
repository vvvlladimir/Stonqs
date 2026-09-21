//! Exercising the CSV import by hand: broker layouts, a preview of a file, a commit, and prices.

use crate::demo::demo_world;
use crate::fmt::{money, print_preview};
use sq_core::import::{
    BrokerPreset, ImportField, ImportOptions, ImportService, ParseConfig, PriceMapping, SecurityDraft,
    builtin_presets, presets_to_json,
};
use sq_core::prelude::*;

/// Import transactions or prices from CSV.
pub fn run(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("prices") => import_prices(&args[1..]),
        Some("scan") => import_scan(&args[1..]),
        Some("presets") => import_presets(&args[1..]),
        _ => import_trades(args),
    }
}

/// Prints what the detector made of every file in a directory as preset JSON, ready to be
/// pasted into `core/presets/brokers.json` and edited by hand. Writing one from a blank page
/// is the part nobody does.
pub fn import_presets(args: &[String]) -> Result<()> {
    let dir = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("cli/samples");
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("csv") | Some("txt")))
        .collect();
    files.sort();

    let world = demo_world()?;
    let service = ImportService::new(&world.store);

    let mut presets = Vec::new();
    for path in &files {
        let name = path.file_stem().and_then(|n| n.to_str()).unwrap_or("?");
        let content = std::fs::read(path)?;
        let Ok(detected) = service.preview(&content, &ParseConfig::default(), None, &[]) else {
            continue;
        };
        presets.push(BrokerPreset::of(name, &detected.config, &detected.mapping));
    }
    println!("{}", presets_to_json(&presets)?);
    Ok(())
}

/// Detection matrix over a directory of broker exports: one line per file, nothing written.
/// The point is to see at a glance which files the parser reads without a single setting.
pub fn import_scan(args: &[String]) -> Result<()> {
    let dir = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .map(String::as_str)
        .unwrap_or("cli/samples");

    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| matches!(p.extension().and_then(|e| e.to_str()), Some("csv") | Some("txt")))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(Error::Invalid(format!("folder {dir} holds no csv files")));
    }

    let world = demo_world()?;
    let account = &world.accounts[0];
    let service = ImportService::new(&world.store);

    println!(
        "{:<26} {:<4} {:<24} {:<4} {:>5} {:>6} {:>6} {:>7} {:>6}",
        "file", "delim", "date format", "dec", "fields", "rows", "ready", "errors", "kinds"
    );
    println!("{}", "-".repeat(98));

    let (mut total, mut ready) = (0usize, 0usize);
    for path in &files {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let content = std::fs::read(path)?;
        let detected = match service.preview(&content, &ParseConfig::default(), None, &[]) {
            Ok(p) => p,
            Err(e) => {
                println!("{name:<26} not parsed: {e}");
                continue;
            }
        };
        // The account is the one thing no file can carry, so give it and judge the rest.
        let mapping = detected.mapping.clone().with_account(&account.id);
        let preview = match service.preview(&content, &ParseConfig::default(), Some(&mapping), &[]) {
            Ok(p) => p,
            Err(e) => {
                println!("{name:<26} not parsed: {e}");
                continue;
            }
        };

        let config = &preview.config;
        let fields = ImportField::ALL
            .iter()
            .filter(|f| preview.mapping.column(**f).is_some())
            .count();
        println!(
            "{:<26} {:<4} {:<24} {:<4} {:>5} {:>6} {:>6} {:>7} {:>6}",
            name,
            config.delimiter.unwrap_or(',').to_string(),
            config.date_format.as_deref().unwrap_or("—"),
            config.decimal_separator.unwrap_or('.').to_string(),
            fields,
            preview.summary.total,
            preview.summary.ready,
            preview.summary.invalid,
            preview.unknown_kinds().len(),
        );
        total += preview.summary.total;
        ready += preview.summary.ready;
    }

    println!("{}", "-".repeat(98));
    println!("files {}, rows {total}, ready {ready}", files.len());
    Ok(())
}

/// Preview a broker export; write changes only when `--commit` is supplied.
pub fn import_trades(args: &[String]) -> Result<()> {
    let commit = args.iter().any(|a| a == "--commit");
    let resolve = args.iter().any(|a| a == "--resolve");
    // Reading a file through a shipped layout instead of through detection.
    let preset = args
        .iter()
        .position(|a| a == "--preset")
        .and_then(|i| args.get(i + 1))
        .map(|name| {
            builtin_presets()
                .iter()
                .find(|p| p.name == *name)
                .ok_or_else(|| Error::Invalid(format!("no preset {name:?}")))
        })
        .transpose()?;
    let named: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    // The argument after `--preset` is its name, not the file.
    let path = named
        .iter()
        .find(|a| preset.is_none_or(|p| p.name != ***a))
        .map(|a| a.as_str())
        .unwrap_or("cli/samples/trades.csv");
    let content = std::fs::read(path)?;

    let world = demo_world()?;
    // Reuse demo history to make duplicate detection visible.
    let account = &world.accounts[0];
    let service = ImportService::new(&world.store);

    let config = preset.map(|p| p.config.clone()).unwrap_or_default();
    // First pass: detect the file configuration and columns.
    let detected = service.preview(&content, &config, None, &[])?;
    println!("file {path}");
    println!(
        "detected: delimiter {:?}, dates {}, decimal mark {:?}",
        detected.config.delimiter.unwrap_or(','),
        detected.config.date_format.as_deref().unwrap_or("not detected"),
        detected.config.decimal_separator.unwrap_or('.'),
    );
    print!("columns:");
    for field in ImportField::ALL {
        if let Some(column) = detected.mapping.column(*field) {
            print!(" {field:?}={column}");
        }
    }
    println!("\n");

    // Second pass: apply the account and a broker-specific kind alias.
    let mut mapping = preset
        .map(|p| p.mapping())
        .unwrap_or_else(|| detected.mapping.clone())
        .with_account(&account.id)
        .with_kind_alias("Umbuchung", TransactionKind::DeliveryInbound);
    let preview = service.preview(&content, &config, Some(&mapping), &[])?;

    // Third pass: resolve unknown symbols through the instrument directory.
    if resolve {
        let lookup = MarketDataService::new().with_search(Box::new(YahooProvider::new()));
        println!("identifying instruments:");
        for symbol in preview.unresolved_symbols() {
            let query = symbol.isin.clone().unwrap_or_else(|| symbol.resolved.clone());
            match lookup.resolve_preferring(&query, symbol.currency.as_deref().or(Some("EUR")))? {
                Some(found) => {
                    let draft = SecurityDraft::from_match(&found, "EUR");
                    println!(
                        "  {:<14} → {:<10} {} ({})",
                        symbol.value, draft.symbol, draft.name, draft.currency
                    );
                    mapping = mapping.with_new_security(&symbol.value, draft);
                }
                None => println!("  {:<14} → not found", symbol.value),
            }
        }
        println!();
    }
    let preview = service.preview(&content, &config, Some(&mapping), &[])?;
    print_preview(&preview);

    if !commit {
        println!("\nnothing was written. Repeat with --commit");
        return Ok(());
    }

    let result = service.commit(&preview, &ImportOptions::default())?;
    println!(
        "\ntransactions written: {}, skipped: {}",
        result.imported, result.skipped
    );
    if !result.created_securities.is_empty() {
        println!("instruments created:");
        for s in world.store.list_securities()? {
            println!(
                "  {:<10} {:<5} {:<45} {}",
                s.symbol,
                s.currency,
                s.name,
                s.data_source.as_deref().unwrap_or("no source")
            );
        }
    }
    for p in &result.problems {
        println!("  ! {}", p.message);
    }
    Ok(())
}

/// Import quotes from CSV, using the argument as a fallback security symbol.
pub fn import_prices(args: &[String]) -> Result<()> {
    let commit = args.iter().any(|a| a == "--commit");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let path = positional
        .first()
        .map(|s| s.as_str())
        .unwrap_or("cli/samples/prices.csv");
    let symbol = positional.get(1).map(|s| s.as_str()).unwrap_or("IWDA");
    let content = std::fs::read(path)?;

    let world = demo_world()?;
    let security = world
        .store
        .find_security_by_symbol(symbol)?
        .ok_or_else(|| Error::NotFound(format!("instrument {symbol}")))?;

    let service = ImportService::new(&world.store);
    let detected = service.preview_prices(&content, &ParseConfig::default(), None)?;
    let mapping = PriceMapping::detect(&["Datum".to_string(), "Schlusskurs".to_string()])
        .with_security(&security.id)
        .with_currency(&security.currency);
    let import = service.preview_prices(&content, &ParseConfig::default(), Some(&mapping))?;

    println!(
        "file {path}, instrument {} — quotes parsed: {} (without the instrument it would be {})",
        security.symbol,
        import.quotes.len(),
        detected.quotes.len()
    );
    for q in &import.quotes {
        println!("{}  {:>10} {}", q.date, money(q.close), q.currency);
    }
    for p in &import.problems {
        println!("! {}", p.message);
    }

    if !commit {
        println!("\nnothing was written. Repeat with --commit");
        return Ok(());
    }
    println!("\nquotes stored: {}", service.commit_prices(&import)?);
    Ok(())
}
