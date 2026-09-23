# Example plugins

A plugin is a folder holding a `plugin.json` and the files that manifest names. Install one from
**Settings → Plugins → Install from folder…**; the app copies the manifest and the declared files
into its own plugin directory, so the folder you picked is free to move afterwards.

See [ADR-0070](../../docs/decisions/0070-a-plugin-brings-data-and-shows-it-it-never-changes-what-a-number-means.md)
for what a plugin may and may not be. This build honours two kinds of content: themes and broker
import layouts.

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

A field the manifest carries that this build does not know is **ignored**, not refused — that is
what lets a package add something for a later version without breaking this one. The cost is that a
typo in an optional field is silently dropped, which is why a package that ends up declaring nothing
at all is refused outright: that is the shape such a typo takes.

An already-installed folder whose `plugin.json` stops parsing is listed as broken, with the reason,
rather than disappearing.
