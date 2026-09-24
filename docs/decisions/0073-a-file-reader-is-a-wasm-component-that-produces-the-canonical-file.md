# 73: A file reader is a WASM component that produces the canonical file

- Status: Accepted

## Context

ADR-0070 named two compute contracts worth opening first and left both undefined. Of the two, the
quote source is almost redundant: `market::custom` (ADR-0054) already lets a user *describe* a
source in data — a URL template plus a path into JSON or the columns of a CSV — so a WASM source
would add only the exotic cases, a signed request or an unusual pagination. The file reader has no
such way around, and behind it sits the largest gap against Portfolio Performance: a hundred PDF
extractors written by its community, but written inside its repository.

The contract is already half-decided. Every reader flattens its bytes into one `ParsedCsv` and
everything after that is shared (`.claude/rules/import.md`), and since ADR-0066 there is a
transaction file of the app's own for a reader to aim at. What is missing is who may produce it
and what that producer is allowed to touch.

Two properties the import layer already has must survive a stranger's code. The preview is
deterministic — `build_preview` is pure, and the host re-reads the loaded bytes on every preview
and again on commit (`commands/import.rs`), so a reader that answered differently the second time
would commit something the user never saw. And a reader is handed somebody's complete brokerage
history, which is the one thing in this app that must be *unable* to leave the machine.

## Decision

### The contract

```
read(bytes, hints{ file-name, password }) -> result<{ canonical, warnings }, error>
```

`canonical` is a `stonqs.transactions` file (ADR-0066) as text. The row schema is **not** restated
in WIT: the format already carries its own `format` and `version`, so version negotiation is
solved once, in the format, rather than twice. What WIT types is the envelope — the hints, the
warnings, and an error that is a code: `not-mine`, `needs-password`, `malformed`.

`warnings` join the preview's own `ImportProblem` list. A reader may warn; it may not refuse a row
on the user's behalf.

### The reader runs once, at load, and is then gone

The host runs the reader in `import_load` and stores **its output** where the file's bytes would
have been. Every later call — each preview, then the commit — reads that canonical file through
`parse_canonical`, which is the path a canonical file has always taken.

So "level 2 has no privileges" is not a promise, it is the construction. A plugin cannot make the
commit differ from the preview because nothing calls it a second time, and the wizard, the
duplicate and restatement rules (ADR-0065) and the commit are reached by exactly one road.

`parse_file` therefore does not become a registry and `core` gains no dependency: the registry is
the host's, one layer above, and `wasmtime` stays out of `core` as ADR-0070 requires.

### The sandbox, and why it has a clock that does not move

No filesystem, no network, no sockets — a reader of a brokerage statement must be unable to send
it anywhere, which is the whole reason this is not a native library.

A clock and a random source are a different matter. A guest carrying a real language runtime links
them at startup and will not instantiate without them, so they are provided as **stubs**: a frozen
clock and a seeded generator. The plugin is then not merely denied the time, it is incapable of
answering differently twice — which is the same property `build_preview` is held to. Standard
output and standard error are captured into a string and surfaced only in a failure.

Two limits are not optional: a memory ceiling, and a deadline by epoch interruption. A stranger's
loop must fail the import, not hang the app.

### Which reader gets the bytes

The shipped readers first, in the order they already run — canonical, Flex, CSV. Then plugins,
narrowed by what their manifest declares about the files they read, one at a time; `not-mine` moves
on to the next. Nothing hands twenty megabytes to every installed plugin in turn.

### A reader ships its fixture or it does not install

The manifest carries a `sample` and the `expected` canonical file for it. Installation runs the
reader on the sample and compares. A package failing that installs nothing — the rule broker
layouts already live under, and the reason is stronger here: a layout that misreads a column is
visible in the wizard, a reader that misreads one produces a canonical file that looks correct.

### The runtime

Component Model / WIT over wasmtime, as ADR-0070 said. On iOS the Pulley interpreter is what runs,
which is also the only form of the runtime that platform permits; parsing a file is not a hot loop
and an interpreter is ample. Cranelift is disabled where it is not needed, because the runtime is
the largest dependency this host has taken.

## Alternatives

- **Type the rows in WIT.** Honest and self-describing, and it puts the transaction schema in a
  second place that must be kept in step with the first. The format already versions itself.
- **Extism.** The same wasmtime underneath with a ready-made harness and host SDKs in many
  languages, and its bytes-in/bytes-out shape is literally this contract. Rejected because it adds
  its own ABI and its own versioning on top of a payload that is already a versioned document.
- **Run the reader on every preview.** Simpler to describe and it hands a plugin the one thing it
  must not have: the chance to commit something other than what was shown.
- **A reader registry inside `core::import::parse_file`.** The obvious place, and it drags the
  runtime into a library that is meant to know nothing about who calls it.
- **A native dynamic library.** Faster, and it makes "this reader cannot exfiltrate your statement"
  something the app can no longer claim.
- **Keep writing readers in this repository.** The Portfolio Performance outcome: contributors
  behind a release cycle, users waiting months.

## Consequences

- The manifest gains a `readers` section beside `themes` and `layouts`, and `PluginInfo` reports
  them the same way. `UiError` gains the reader-shaped failures: refused, trapped, timed out, out
  of memory, needs a password.
- `LoadedFile` keeps naming the file the user chose and its real size; what is held beside it is no
  longer necessarily what was on disk. An export of "the file as loaded" is therefore the canonical
  one, which is the readable half of the same thing.
- The wizard's per-file settings — separator, skipped rows, date format — mean nothing for a file a
  reader produced, exactly as they mean nothing for a Flex statement today.
- A reader cannot be tried in the wizard and dropped: it is chosen by the file, not by the user. A
  file a plugin claimed wrongly is the plugin's bug, and removing the plugin is the answer.
- The first reader is written in this repository on purpose. If it needs a hole in the contract,
  the contract is wrong while nobody outside has paid for it yet.
