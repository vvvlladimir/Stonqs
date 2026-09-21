# Stonqs (workspace)

Rust application for tracking an investment portfolio — with a layered core library and a
separate UI host.

```
.
├── core/    sq-core library — all application logic (see core/README.md)
└── cli/     temporary binary for manually exercising the core; not part of the product
```

## Quick start

```bash
cargo build
cargo test

# in-memory demo portfolio: positions, EUR valuation, TWR, XIRR, reports
cargo run -p sq-cli -- demo
cargo run -p sq-cli -- series          # daily value series grouped by month
cargo run -p sq-cli -- risk            # volatility, drawdown, Sharpe
cargo run -p sq-cli -- allocation      # + currency | account | security
cargo run -p sq-cli -- benchmark       # portfolio versus benchmark
cargo run -p sq-cli -- rebalance       # plan for reaching target allocation
cargo run -p sq-cli -- import          # parse broker CSV (preview)
cargo run -p sq-cli -- import cli/samples/trades.csv --commit
cargo run -p sq-cli -- import prices cli/samples/prices.csv IWDA

# live data (internet required)
cargo run -p sq-cli -- quotes AAPL 2024-06-03 2024-06-10
cargo run -p sq-cli -- rates USD EUR 2024-06-03 2024-06-08
```

See [`core/README.md`](core/README.md) for core architecture and design decisions.
