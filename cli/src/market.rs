//! The subcommands that reach the network: instrument lookup, venue listings, quotes and rates.

use crate::args::parse_date;
use crate::fmt::money;
use sq_core::prelude::*;

/// Resolve an instrument and print the preferred result plus all candidates.
pub fn lookup(args: &[String]) -> Result<()> {
    let query = args.join(" ");
    if query.trim().is_empty() {
        return Err(Error::Invalid("give an ISIN, a ticker or a name".into()));
    }
    let service = MarketDataService::new().with_search(Box::new(YahooProvider::new()));

    // Prefer EUR so European listings avoid an unnecessary FX conversion.
    match service.resolve_preferring(&query, Some("EUR"))? {
        Some(best) => println!(
            "best: {} · {} · {} · {} · {:?}",
            best.symbol,
            best.name,
            best.currency.as_deref().unwrap_or("currency unknown"),
            best.exchange.as_deref().unwrap_or("exchange unknown"),
            best.kind,
        ),
        None => println!("nothing found for {query:?}"),
    }

    println!("\nfull result:");
    for found in service.search(&query)? {
        println!(
            "  {:<12} {:<10} {}",
            found.symbol,
            found.exchange.as_deref().unwrap_or("—"),
            found.name
        );
    }
    Ok(())
}

/// List all venues for an ISIN and select the best one for a preferred currency.
pub fn listings(args: &[String]) -> Result<()> {
    let Some(isin) = args.first() else {
        return Err(Error::Invalid("usage: listings <isin> [currency]".into()));
    };
    let preferred = args.get(1).map(String::as_str).unwrap_or("EUR");
    let service = MarketDataService::new()
        .with_search(Box::new(YahooProvider::new()))
        .with_directory(Box::new(OpenFigiDirectory::new()));

    let found = service.listings(isin)?;
    println!("venues found: {}", found.len());
    for listing in service.probe_listings(found, 8) {
        println!(
            "  {:<12} {:<28} {:<4} {:>12}  {}",
            listing.symbol.as_deref().unwrap_or("—"),
            listing.exchange.as_deref().unwrap_or(&listing.mic),
            listing.currency.as_deref().unwrap_or("?"),
            listing
                .last_close
                .map(|c| c.to_string())
                .unwrap_or_else(|| "—".into()),
            match listing.has_history {
                Some(true) => "has prices",
                Some(false) => "no prices",
                None => "not checked",
            }
        );
    }

    match service.best_listing(isin, Some(preferred))? {
        Some(best) => println!(
            "\nbest for base {preferred}: {} · {}",
            best.symbol.as_deref().unwrap_or("—"),
            best.currency.as_deref().unwrap_or("?")
        ),
        None => println!("\nno working listing was found"),
    }
    Ok(())
}

pub fn quotes(args: &[String]) -> Result<()> {
    let ([symbol, from, to] | [symbol, from, to, _]) = args else {
        return Err(Error::Invalid(
            "usage: quotes <symbol> <from> <to> [yahoo|stooq]".into(),
        ));
    };
    // Named outright: the app ships no default source (ADR-0076), and this harness registers
    // both providers two lines below whatever the argument says.
    let source = args.get(3).map(String::as_str).unwrap_or(YahooProvider::ID);
    let range = DateRange::new(parse_date(from)?, parse_date(to)?);

    let store = Store::open_in_memory()?;
    let security =
        Security::new(symbol.to_uppercase(), symbol, "USD", SecurityKind::Stock).with_source(source, symbol);
    store.save_security(&security)?;

    let market = MarketDataService::new()
        .with(Box::new(YahooProvider::new()))
        .with(Box::new(StooqProvider::new()));

    let saved = market.ensure_history(&store, &security, range)?;
    println!("quotes downloaded and stored: {saved}");

    // The covered range should make this call a cache hit.
    let again = market.ensure_history(&store, &security, range)?;
    println!("the same range requested again: {again} new (the cache works)");

    for q in store.quotes_in_range(&security.id, range)? {
        println!("{}  {:>12} {}", q.date, money(q.close), q.currency);
    }
    Ok(())
}

/// Fetch and print exchange rates through the shipped FX chain.
pub fn rates(args: &[String]) -> Result<()> {
    let [from_cur, to_cur, from, to] = args else {
        return Err(Error::Invalid(
            "usage: rates <from-cur> <to-cur> <from> <to>".into(),
        ));
    };
    let range = DateRange::new(parse_date(from)?, parse_date(to)?);

    let store = Store::open_in_memory()?;
    let fx = sq_core::sources::fx_service();
    let (saved, source) = fx.ensure_rates(&store, from_cur, to_cur, range)?;
    println!("rates stored for {from_cur}/{to_cur} from {source}: {saved}");

    for r in store.fx_rates_in_range(from_cur, to_cur, range)? {
        println!("{}  {}", r.date, r.rate.round_dp(6));
    }

    // As with quotes, as-of lookup fills a non-trading day from the prior rate.
    let weekend = range.to;
    if let Some(rate) = store.rate_as_of(from_cur, to_cur, weekend)? {
        println!("\nrate on {weekend} (as-of) = {}", rate.round_dp(6));
    }
    Ok(())
}
