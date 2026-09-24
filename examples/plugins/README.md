# Example plugins

A plugin is a folder holding a `plugin.json` and the files that manifest names. Install one from
**Settings → Plugins → Install from folder…**; the app copies the manifest and the declared files
into its own plugin directory, so the folder you picked is free to move afterwards.

See [ADR-0070](../../docs/decisions/0070-a-plugin-brings-data-and-shows-it-it-never-changes-what-a-number-means.md)
for what a plugin may and may not be, and
[ADR-0073](../../docs/decisions/0073-a-file-reader-is-a-wasm-component-that-produces-the-canonical-file.md)
for the file reader. This build honours four kinds of content: themes, broker import layouts,
classification sets and file readers.

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
it puts **Regions** beside `Import from CSV…` on the Allocation screen; pressing it opens the same
preview a file opens, with the same choice of extending an existing tree or creating a new one.

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

Returning `not-mine` is not a failure — the app moves on to the next reader and then to its own.
Returning `malformed` is, and it says so with the reader's own reason. A warning does not stop
anything: it is shown beside the preview, in the reader's words.

## When a package is wrong

Installation is the check, and it refuses rather than half-installs:

| What is wrong | What happens |
| --- | --- |
| A required field is missing or misspelled (`id`, `api`, `name`; a layout's `id`/`file`/`sample`) | Refused, naming the field |
| `id` has anything but lowercase letters, digits, dot, dash, underscore | Refused |
| `api` is not the version this build speaks | Installs, and is listed **not loaded** with both versions — a package from a later build is not an error, it is a package for later |
| `provides` misspelled, or only content this build does not know | Refused: "declares nothing this build can use" |
| A file the manifest names is missing, or its name leaves the package (`../`) | Refused |
| The layout file does not parse, or names a column or operation the app has no such thing as | Refused, and the message lists what the app does have |
| The layout does not recognise its own sample, leaves a wording of it unmapped, or reads a row of it as invalid | Refused, saying which |
| A reader's `file` is not a WebAssembly component, or the module fails to start | Refused |
| A reader does not recognise its own sample, produces something that is not a transaction file, or produces a different one from `expected` | Refused, saying which |
| A classification set's CSV does not read as a tree, or leaves a row invalid | Refused, saying which |

A field the manifest carries that this build does not know is **ignored**, not refused — that is
what lets a package add something for a later version without breaking this one. The cost is that a
typo in an optional field is silently dropped, which is why a package that ends up declaring nothing
at all is refused outright: that is the shape such a typo takes.

An already-installed folder whose `plugin.json` stops parsing is listed as broken, with the reason,
rather than disappearing.
