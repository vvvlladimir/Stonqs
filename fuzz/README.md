# Fuzzing the import

Every byte the import reads was written by somebody else — a broker, a bank, an exchange, or a
file the user picked by mistake. The property under test is not "it works" but "it answers":
no panic, no hang, no unbounded allocation, whatever the bytes are.

The crate is outside the workspace on purpose. `cargo fuzz` builds with `-Z` flags that need
nightly, and the product stays on the toolchain pinned in `rust-toolchain.toml`.

```bash
cargo install cargo-fuzz
cargo +nightly fuzz list
cargo +nightly fuzz run parse_any -- -max_total_time=60
cargo +nightly fuzz run preview   -- -max_total_time=300
```

Four targets, in the order a file passes through them:

| target | what it covers |
| --- | --- |
| `parse_any` | `parse_file` — the reader the wizard calls, which picks between the three formats itself |
| `parse_canonical` | the app's own transaction file, read back from somebody's backup |
| `parse_flex` | Interactive Brokers Flex XML |
| `preview` | detection, mapping, drafts and checks, against an in-memory store |

`corpus/<target>/` is the seed corpus, and it is worth keeping small and *varied* rather than
large: one file per shape the reader branches on. Seed it from the conformance fixtures:

```bash
cp core/tests/fixtures/presets/*/*.csv fuzz/corpus/parse_any/
```

**A crash is committed.** Copy the reproducing input into `core/tests/fixtures/crashes/`, add the
case to `core/tests/phase3_import/robustness.rs`, then fix it — the test is what keeps it fixed,
and the corpus file is what keeps the fuzzer from having to find it again.

CI runs a short session per target on a schedule (`.github/workflows/fuzz.yml`), not on a pull
request: a fuzzer given ninety seconds is a smoke test, and treating it as a gate would make a
pull request fail for the machine it happened to land on.
