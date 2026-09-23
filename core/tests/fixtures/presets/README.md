# Layout conformance fixtures

One folder per broker layout. The harness is `core/tests/phase3_import/conformance.rs`; it asserts
two things about every folder here: the file is recognised as the layout it names, and importing it
produces exactly the operations in `expected.json`.

```
<slug>/
  fixture.json    what this is (below)
  <the file>      the broker's export, redacted
  expected.json   the app's own transaction format (ADR-0066), regenerated, read once by eye
```

`fixture.json`:

| field | meaning |
| --- | --- |
| `preset` | the shipped layout's name, exactly |
| `file` | the export's file name inside the folder |
| `sample` | `redacted` — a real export with names and numbers replaced; `declared` — written from the broker's published columns, which proves the layout self-consistent and nothing more |
| `note` | what this sample is meant to cover |
| `account` | the account the file lands on: `SECURITIES` for a broker statement, `DEPOSIT` for a bank or wallet one |
| `questions` | how much the wizard had to ask a human. All zeros is the point of having a layout |

Regenerate an expectation after changing a layout:

```bash
UPDATE_FIXTURES=1 cargo test -p sq-core --test phase3_import conformance
```

It rewrites `expected.json` and then fails on purpose — read the diff, then run again without the
variable. An expectation nobody read is worth nothing.

**Redact, never invent.** A sample carries no real name, account number or balance; the shapes and
the wordings are what matter and they must stay exactly as the broker prints them. A layout with no
sample is listed in `WITHOUT_A_FIXTURE` in the harness, so the debt is visible and shrinking.
