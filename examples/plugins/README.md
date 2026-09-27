# Example plugins

A plugin is a folder holding a `plugin.json` and the files that manifest names. Install one from
**Settings → Plugins → Install from folder…**; the app copies the manifest and the declared files
into its own plugin directory, so the folder you picked is free to move afterwards.

See [ADR-0070](../../docs/decisions/0070-a-plugin-brings-data-and-shows-it-it-never-changes-what-a-number-means.md)
for what a plugin may and may not be, and
[ADR-0086](../../docs/decisions/0086-a-file-reader-is-a-wasm-component-that-produces-the-canonical-file.md)
for the file reader, [ADR-0080](../../docs/decisions/0080-a-file-writer-is-the-reader-turned-round.md)
for the file writer, [ADR-0083](../../docs/decisions/0083-a-plugin-widget-is-a-page-with-no-origin-fed-by-the-host.md)
for the dashboard widget, [ADR-0084](../../docs/decisions/0084-a-plugin-screen-and-the-one-document-a-plugin-keeps.md)
for the screen, [ADR-0085](../../docs/decisions/0085-a-plugin-assistant-tool-is-a-component-that-answers-from-declared-reads.md)
for the assistant tool. This build honours nine kinds of content: themes, broker import layouts,
classification sets, operation dictionaries, file readers, file writers, dashboard widgets,
screens and assistant tools.

## `midnight` — a theme

```json
{
  "id": "app.stonqs.midnight",
  "api": 1,
  "name": "Midnight",
  "version": "1.0.0",
  "provides": {
    "themes": [{ "id": "midnight", "name": "Midnight", "file": "midnight.css", "base": "dark" }]
  }
}
```

- `id` is the folder name the app installs into, and the prefix its secrets would carry: lowercase
  letters, digits, dot, dash, underscore.
- `api` is the plugin API the package is built for. A package naming another version is listed in
  Settings with the reason and is **not loaded** — never half of it.
- `base` is the built-in scheme the theme varies, `light` or `dark`. What the stylesheet does not
  redefine keeps that scheme's value, so a theme can be twelve lines long.

The stylesheet writes plain `:root` rules — it is put in force only while it is the chosen theme,
so it needs no selector of the app's own. The properties it can redefine are the ones in
`app/src/styles/tokens.css`; anything else it declares is ignored by the app and inherited by
nothing.

## `northbay` — a broker layout

An invented broker, so nothing here is any real one's format. Installing it puts **Northbay
Securities** in the import wizard's layout list under *From plugins*; `sample.csv` in the same
folder is a file to try it on — the wizard should recognise it by itself, with nothing left to
answer but which account it lands on.

```json
{
  "id": "app.stonqs.northbay",
  "api": 1,
  "name": "Northbay Securities",
  "version": "1.0.0",
  "provides": {
    "layouts": [{ "id": "northbay", "file": "layout.json", "sample": "sample.csv" }]
  }
}
```

`file` is one layout in the shape `core/presets/brokers.json` uses — the app writes exactly that
shape, so the quickest way to author one is to lay a file out in the import wizard, save it, and
copy the entry out of the profile's `import_templates.json`.

`sample` is **required**: a redacted export the layout must read. It is not decoration. Before a
package is installed the app checks that

- the layout recognises that sample at all,
- every operation wording in it is mapped — an unmapped one is a question the wizard would put to
  whoever imports the file, and
- no row of it reads as invalid.

A package failing any of the three installs nothing at all. The layouts shipped with the app
answer the same questions, plus a third — the exact operations they produce — in
`core/tests/fixtures/presets/`.

A layout from a plugin appears in the wizard's list under **From plugins**, after the user's own
and before the shipped ones. It cannot be deleted there: it arrived with the plugin and it leaves
with it.

## `regions` — a classification set

A ready classification tree, shipped as the same CSV the taxonomy import already reads. Installing
it puts **Import Regions…** on each tree's menu on the Allocation screen, and — while no
classification exists yet — beside `Import from CSV…` in the empty state. Either way it opens the
same preview a file opens, extending the tree it was invoked on or creating a new one.

```json
{
  "id": "app.stonqs.regions",
  "api": 1,
  "name": "Regions classification",
  "version": "1.0.0",
  "provides": {
    "taxonomies": [{ "id": "regions", "name": "Regions", "file": "regions.csv" }]
  }
}
```

The CSV is read by meaning rather than by template: level columns are found by their headers
(`Levels 1`, `Levels 2`, … or `Category`), a row with a ticker or an ISIN is a security, and the
first level repeats on every row and is therefore the tree's own name. A security is matched by
ISIN first and by ticker second, so a set works on a portfolio that bought the same fund on another
exchange.

`name` is what the tree is called when it is created, and only then: a classification's name is the
user's data from that moment on, renamed freely and never translated by the app.

There is **no** `expected` here, unlike a reader's. The file *is* the data, so an expectation would
be a copy of it. What the install checks instead is that the set reads as a tree at all and that no
row of it is invalid — against an empty instrument list, so nothing about the open portfolio is
touched and a set is judged for being a tree rather than for fitting this particular portfolio. A
set that matches none of your instruments still installs: the tree is there and the assignments are
yours to make.

## `finnish-words` — an operation dictionary

The import recognises an operation by its wording — `Buy`, `Kauf`, `Verkoop`, `Покупка` — from a
dictionary kept **per language, never per broker**, because a word learned from one broker's file
helps every broker writing in that language. A dictionary plugin adds a language the app does not
speak. This one is Finnish: `Osto`, `Myynti`, `Osinko`, `Talletus`, `Nosto`, …

```json
{
  "id": "app.stonqs.finnish-words",
  "api": 1,
  "name": "Finnish operation words",
  "version": "1.0.0",
  "provides": {
    "dictionaries": [{ "id": "fi", "file": "words.json", "sample": "sample.csv" }]
  }
}
```

`words.json` is an ordered list:

```json
{ "words": [{ "word": "Nosto", "kind": "WITHDRAWAL" }, { "word": "Osto", "kind": "BUY" }] }
```

A word matches any wording that **contains** it, case and spacing aside, and the first match wins —
which is why `Nosto` (withdrawal) comes before `Osto` (buy), the word it contains. `kind` is one of
the operation names the app's own transaction file uses (`BUY`, `SELL`, `DIVIDEND`, `INTEREST`,
`DEPOSIT`, `WITHDRAWAL`, `FEE`, `TAX`, …).

The app's own words are always asked **first**. An added word only answers a wording the app would
otherwise leave to the user, so a dictionary can never turn a `Buy` into a sale — and one that tries
is refused at install rather than kept as a word that would silently never apply. The words are
read at every preview and are not copied into a layout, so removing the plugin removes them; a
layout the user saved keeps only the answers it was saved with.

A dictionary brings operation words only. Column headers are a different dictionary, so the sample
here uses English headers and Finnish operations.

`sample` is **required**, and it has to be a file only this dictionary can read: the install checks
that the app alone leaves some of its wordings unanswered, and that with the dictionary none is left
and no row is invalid. A sample the app already reads by itself would prove nothing about the words.

## `mt940` — a file reader

MT940 is the SWIFT bank statement most European banks still export: tagged text, one `:61:` line
per entry, no columns anywhere. No import layout can express it, which is exactly the case a
reader exists for.

```json
{
  "id": "app.stonqs.mt940",
  "api": 1,
  "name": "MT940 bank statements",
  "version": "1.0.0",
  "provides": {
    "readers": [
      {
        "id": "mt940",
        "file": "reader.wasm",
        "sample": "sample.sta",
        "expected": "expected.json",
        "extensions": [".sta", ".mt940", ".940"]
      }
    ]
  }
}
```

A reader is a **WebAssembly component**, and it is handed one thing and asked for one thing:

```
read(bytes, hints{ file-name, password }) -> result<{ canonical, warnings }, error>
```

`canonical` is one of the app's own transaction files — the same format **Export** writes, so
there is nothing new to learn and nothing to keep in step. The contract itself is
`app/src-tauri/wit/reader.wit`; `src/` here is the guest, about 250 lines of Rust with one
dependency, built by `./build.sh` (`rustup target add wasm32-wasip2` once).

What the module can reach is the whole of what it is given:

- **No filesystem, no network, no address of any kind.** A reader of your bank statement is
  *unable* to send it anywhere. That is the reason this is WebAssembly and not a plain library.
- **No real clock and no real randomness.** Both are linked, because a language runtime will not
  start without them, and both are frozen and seeded — so a reader cannot answer differently twice.
- A ceiling on memory, and a deadline. A module that runs past either fails the import rather than
  hanging the app.

The reader runs **once**, when the file is loaded, and what it produced is what the wizard previews
and what the commit writes. Nothing calls it a second time, so the preview cannot show one thing
and the import write another.

`extensions` is what the reader is offered. Empty means every file the app did not already
recognise, which is honest for a format with no ending of its own and expensive for everyone else.

`expected` is **required**, and it is stricter than a layout's `sample`: it is the document the
sample must come out as. Before installing, the app runs the reader on its own sample and compares.
A layout that misreads a column leaves a visible question in the wizard; a reader that misreads one
hands over a file that looks perfectly correct, so it is checked against an answer rather than
against a shrug.

A sealed file — a PDF statement with a password — is answered with `needs-password`. The app then
asks the user, and calls your reader again with `hints.password` set; the password goes to the
reader that asked and to no other, and it is never stored. Answer `needs-password` again when it is
wrong: the prompt says so. Your `sample` must still be a file you read **without** a password —
installing asks nobody for one.

Returning `not-mine` is not a failure — the app moves on to the next reader and then to its own.
Returning `malformed` is, and it says so with the reader's own reason. A module that traps, runs
past its deadline or out of memory never said the file was its own, so it is passed over like
`not-mine` — the wizard says which plugin failed — rather than blocking every file the user opens. A warning does not stop
anything: it is shown beside the preview, in the reader's words.

## `ledger` — a file writer

The reader turned round: it is handed the app's own transaction file and returns the bytes of
another format. This one writes the plain-text accounting journal that `ledger` and `hledger` read —
one balanced entry per operation, several lines each, which no column layout could express.
Installing it makes **Export** on the Transactions screen ask which format: `Stonqs file` or
**Ledger journal**.

```json
{
  "id": "app.stonqs.ledger",
  "api": 1,
  "name": "Ledger journal export",
  "version": "1.0.0",
  "provides": {
    "writers": [
      {
        "id": "ledger",
        "name": "Ledger journal",
        "file": "writer.wasm",
        "sample": "sample.json",
        "expected": "expected.journal",
        "extension": "journal"
      }
    ]
  }
}
```

The contract is `app/src-tauri/wit/writer.wit`:

```
write(canonical) -> result<bytes, reason>
```

`canonical` is the same document **Export** saves as a `Stonqs file` — the operations the screen
shows, with its filter, already resolved by the app. The writer sees nothing else: not the
portfolio, not a path, not the name the user will save under. It runs in exactly the reader's
sandbox — no filesystem, no network, a frozen clock, a memory ceiling and a deadline — so the
two are one grant, not two.

`name` is what the export menu shows, in the plugin's own words. `extension` is the ending the
saved file gets, letters and digits without the dot.

`expected` is **required**, and it is compared byte for byte with what the writer makes of
`sample`: the output *is* bytes, and the program that reads the file judges every one of them. A
wrong journal looks like a journal, so it is checked against an answer. `src/` is the guest, about
150 lines of Rust with `serde_json` and `wit-bindgen`, built by `./build.sh`.

This writer names an operation it has no journal equivalent for — a delivery, a transfer between
one's own accounts — in a comment line rather than guessing at it. Refusing the whole document is
also allowed: the reason it returns is shown to the user, naming the plugin.

## When a package is wrong

Installation is the check, and it refuses rather than half-installs:

| What is wrong | What happens |
| --- | --- |
| A required field is missing or misspelled (`id`, `api`, `name`; a layout's `id`/`file`/`sample`) | Refused, naming the field |
| `id` has anything but lowercase letters, digits, dot, dash, underscore | Refused |
| The `id` of a theme, layout, reader, widget, tool… has anything else, or two of one kind share one | Refused |
| A tool's schema has no `required` array, or requires a property it does not declare | Refused |
| A tool would reach the assistant under a name another installed plugin's tool already has (`.` and `-` both become `_`) | Refused, naming the plugin |
| `api` is not the version this build speaks | Installs, and is listed **not loaded** with both versions — a package from a later build is not an error, it is a package for later |
| `provides` misspelled, or only content this build does not know | Refused: "declares nothing this build can use" |
| A file the manifest names is missing, or its name leaves the package (`../`) | Refused |
| The layout file does not parse, or names a column or operation the app has no such thing as | Refused, and the message lists what the app does have |
| The layout does not recognise its own sample, leaves a wording of it unmapped, or reads a row of it as invalid | Refused, saying which |
| A reader's `file` is not a WebAssembly component, or the module fails to start | Refused |
| A reader does not recognise its own sample, produces something that is not a transaction file, or produces a different one from `expected` | Refused, saying which |
| A classification set's CSV does not read as a tree, or leaves a row invalid | Refused, saying which |
| A dictionary carries no words, a word the app already reads as another operation, or a word shorter than three letters (two ideographs) | Refused, naming the words |
| A dictionary's sample reads without it, or still leaves a wording unmapped or a row invalid with it | Refused, saying which |
| A writer's `extension` is not letters and digits, its sample is not a transaction file, or writing it does not give exactly `expected` | Refused, saying which |

A field the manifest carries that this build does not know is **ignored**, not refused — that is
what lets a package add something for a later version without breaking this one. The cost is that a
typo in an optional field is silently dropped, which is why a package that ends up declaring nothing
at all is refused outright: that is the shape such a typo takes.

An already-installed folder whose `plugin.json` stops parsing is listed as broken, with the reason,
rather than disappearing.

## `heat` — a dashboard widget

A tile of one's own on the dashboard: here, every position as a cell sized by its weight and
coloured by its unrealized result over cost. That percentage is the plugin's figure, not the
app's — a plugin may show numbers of its own, under its own name, and may never replace one the
app shows ([ADR-0082](../../docs/decisions/0082-a-plugin-may-show-a-number-of-its-own-under-its-own-name.md)).
The tile's header always names the plugin.

```json
{
  "id": "app.stonqs.heat",
  "api": 1,
  "name": "Heat map",
  "version": "1.0.0",
  "provides": {
    "widgets": [
      {
        "id": "heat",
        "name": "Position heat map",
        "description": "Each position as a cell sized by its weight and coloured by its unrealized result.",
        "file": "heat.js",
        "reads": ["positions"],
        "size": { "w": 6, "h": 8 },
        "min": { "w": 3, "h": 4 }
      }
    ]
  }
}
```

- `file` is **one self-contained ES module**. The page allows no script but its own, so a widget
  that imports another file must be bundled into one first.
- `reads` is what the tile is handed, from a closed list: `valuation`, `positions`, `performance`.
  Nothing else is reachable — there is no request to make. The palette tells the user this list
  before the tile is placed.
- `periodic: true` gives the tile a period in its settings; `performance` requires it.
- `size` and `min` are in the board's units: width in twelfths (1–12), height in rows.

The module registers one function, called again whenever the date, the period, the data source,
the language, the theme or the data itself changes:

```js
stonqs.render((root, { context, data }) => {
  // root: the page's <body>, yours to fill
  // context: { api, date, period: { from, to } | null, base_currency, locale,
  //            theme: { scheme: "light" | "dark", tokens: { "--text": "…", "--pos": "…", … } } }
  // data: only the reads the manifest declared
});
```

The theme's properties are set on the page's root, so `var(--text)`, `var(--pos)`, `var(--neg)`,
`var(--surface-2)` and the palette slots `var(--slot-1)` … `var(--slot-8)` follow the app's theme,
a plugin theme included.

The data are the plugin API's own names, not the app's internal ones, and they change only with
`api`. Every amount is a **string** in the base currency, exactly as the app keeps it; whatever
you compute from it is your figure, not the app's:

| read | shape |
| --- | --- |
| `valuation` | `date`, `base_currency`, `total_value`, `securities_value`, `cash`, `cost_basis`, `unrealized_result`, `realized_result`, `dividends`, `interest`, `fees`, `taxes` |
| `positions` | `date`, `base_currency`, `total_value`, `rows[]`: `symbol`, `name`, `currency` (of `price`), `quantity`, `price`, `value`, `cost_basis`, `unrealized_result`, `weight` (`"0.25"` is a quarter), `day_change` (a fraction or `null`) |
| `performance` | `from`, `to`, `base_currency`, `twr`, `twr_annualized`, `xirr`, `start_value`, `end_value`, `net_flow`, `earned`, `series[]`: `date`, `value`, `flow` |

What the page can reach is what it is handed:

- **No network.** The page's policy has no `connect-src`: no `fetch`, no socket, no image from an
  address. A peer connection (`RTCPeerConnection`) is removed before your module runs, and DNS
  prefetching is off — the two ways out a content policy does not cover. A tile of your
  positions cannot send them anywhere.
- **No origin.** The frame is sandboxed with scripts only — no storage, no cookies, no popups, no
  navigating the app — and the app's own commands are not reachable from it.
- A widget that throws, rejects a promise, or never calls `stonqs.render` shows that in its tile,
  with its own message; the rest of the board is untouched. A busy loop, however, is not
  interruptible — it runs in the app's window.

There is no `sample` and no `expected` here, unlike a reader: a drawing has no answer the app could
compare it with. What the install checks is the shape — a `.js` file, known reads, sizes that fit
the grid.

## `spending` — a screen

Where the money that left went: every withdrawal of the period sorted into a category by the first
rule whose text the operation's note contains. It is the test this API was built against — a
feature of its own, written as a plugin, needing no hole in the app. Import a bank statement (the
`mt940` reader above is one way), open **Plugins → Spending**, and sort what it asks about.

```json
{
  "id": "app.stonqs.spending",
  "api": 1,
  "name": "Spending",
  "version": "1.0.0",
  "provides": {
    "screens": [
      {
        "id": "spending",
        "name": "Spending",
        "description": "Where the money that left went: withdrawals sorted into categories by rules you write.",
        "file": "spending.js",
        "reads": ["transactions"],
        "periodic": true,
        "storage": true
      }
    ]
  }
}
```

A screen is a widget with the whole page: the same module, the same `stonqs.render`, the same
page with no network and no origin. Three things differ.

- **It follows the app's lenses** — the account picker and the date — and has no data source of
  its own. With `periodic`, the app puts its period control in the screen's header.
- **`transactions` is a screen's read**, and requires `periodic`: the operations of the accounts in
  view over the period. A widget cannot ask for it, because a widget has a source of its own and
  the operation list does not.

  | read | shape |
  | --- | --- |
  | `transactions` | `base_currency`, `rows[]`: `id` (opaque, stable while the row exists), `date`, `kind` (`BUY`, `WITHDRAWAL`, …), `account` (its name), `symbol`, `amount` and `currency`, `amount_base`, `net_base` (signed cash leg), `note` |

- **With `storage`, it keeps one document** in the open profile:

  ```js
  stonqs.render((root, { data }) => {
    const rules = data.state?.rules ?? []; // null before the first save
    // …
    stonqs.save({ rules: [...rules, { match: "REWE", category: "Groceries" }] });
  });
  ```

  `save` replaces the whole document (up to 256 KiB) and the page is rendered again with it. It is
  the plugin's data, not the portfolio's: it cannot change an operation, an account or a figure, and
  nothing in the app reads it — which is why saving asks nothing. It lives in the profile's
  database, so a profile with a password encrypts it, and removing the plugin leaves it in place.

The page has no forms (`<form>` does not submit in a sandbox with scripts only): use buttons with
click handlers, as `spending.js` does.

## `concentration` — an assistant tool

A question the app's own assistant has no tool for: how concentrated the positions in view are —
the effective number of holdings (one over the sum of squared weights) and the share of the largest
few. Ask the assistant "am I diversified?" once it is installed.

```json
{
  "id": "app.stonqs.concentration",
  "api": 1,
  "name": "Concentration",
  "version": "1.0.0",
  "provides": {
    "tools": [
      {
        "id": "concentration",
        "name": "Concentration",
        "description": "How concentrated the positions in view are: …",
        "file": "tool.wasm",
        "schema": "schema.json",
        "reads": ["positions"],
        "sample": "sample.json",
        "expected": "expected.json"
      }
    ]
  }
}
```

A tool is a **WebAssembly component**, like a reader (`app/src-tauri/wit/tool.wit`):

```
call(args, data) -> result<answer, error>     // all three are JSON text
```

- `args` is what the model called it with. `schema.json` says what that may be, and it must be the
  strict subset every provider accepts — an object, `"additionalProperties": false`, every property
  required — the `required` array is there even when empty — each one a string, number, integer
  or boolean. A schema outside it would not fail this
  tool alone: the provider refuses the whole request, and with it every chat. So it is checked at
  install.
- `data` is exactly the declared `reads`, in the same shape a widget is handed (the table under
  `heat` above). A tool reading `performance` or `transactions` is asked for a period: the app adds
  a `period` argument to the schema itself, resolves it, and hands the tool `data.period` with the
  dates — the model never writes a date. Do not name `period` or `reason` in your own schema.
- The answer is any JSON value up to 64 KiB. The model receives it as
  `{ "plugin": …, "tool": …, "answer": … }`, is told the figures are the plugin's own and not the
  app's, and the user is asked before the first call like for any reading. A tool cannot write
  anything and has no network.

`sample.json` is a call (`{ "args": …, "data": … }`) and `expected.json` what it must answer,
compared as JSON; a package whose tool answers anything else installs nothing. `src/` is the guest,
about 80 lines of Rust, built by `./build.sh`.

