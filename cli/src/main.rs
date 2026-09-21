//! Temporary CLI for manually exercising `sq-core`; it is not a product surface.
//! It intentionally uses only `std::env::args` and the core crate.

mod args;
mod demo;
mod fmt;
mod import;
mod market;
mod report;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("demo");

    // The core stays silent; the CLI owns output and exit status.
    let result = match command {
        "demo" => demo::run(),
        "series" => report::series(&args[1..]),
        "risk" => report::risk(&args[1..]),
        "allocation" => report::allocation(&args[1..]),
        "benchmark" => report::benchmark(&args[1..]),
        "rebalance" => report::rebalance(&args[1..]),
        "import" => import::run(&args[1..]),
        "lookup" => market::lookup(&args[1..]),
        "listings" => market::listings(&args[1..]),
        "quotes" => market::quotes(&args[1..]),
        "rates" => market::rates(&args[1..]),
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        other => {
            eprintln!("unknown command: {other}\n");
            print_help();
            std::process::exit(2);
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn print_help() {
    println!(
        "sq-cli — a hand check of the core\n\n\
         commands over demo data (in-memory database, no network):\n  \
         demo                             valuation, TWR, XIRR\n  \
         series [from] [to]               daily value series, monthly totals\n  \
         risk [from] [to]                 volatility, drawdown, Sharpe\n  \
         allocation [taxonomy|currency|account|security]\n  \
         {:33}breakdowns of the current valuation\n  \
         benchmark [from] [to]            portfolio against a benchmark\n  \
         rebalance                        plan to reach the target structure\n  \
         import [file] [--preset name] [--resolve] [--commit]\n  \
         {:33}parse a broker CSV (defaults to\n  \
         {:33}cli/samples/trades.csv); --resolve identifies\n  \
         {:33}instruments in the directory, which needs network\n  \
         import scan [dir]                detection matrix: one row per file\n  \
         import presets [dir]             draft presets from a folder of exports (JSON)\n  \
         import prices [file] [symbol] [--commit]\n  \
         {:33}import quotes from a CSV\n\n\
         live data (needs the internet):\n  \
         lookup <isin|ticker|name>        identify an instrument in the data source\n  \
         listings <isin> [currency]       an instrument's venues and the best of them\n  \
         quotes <symbol> <from> <to>      download quotes from Yahoo\n  \
         rates <from-cur> <to-cur> <from> <to>\n  \
         {:33}download rates from the ECB\n  \
         help                             this help\n\n\
         dates in YYYY-MM-DD format",
        "", "", "", "", "", ""
    );
}
