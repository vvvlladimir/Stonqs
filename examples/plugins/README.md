# Example plugins

A plugin is a folder holding a `plugin.json` and the files that manifest names. Install one from
**Settings → Plugins → Install from folder…**; the app copies the manifest and the declared files
into its own plugin directory, so the folder you picked is free to move afterwards.

See [ADR-0070](../../docs/decisions/0070-a-plugin-brings-data-and-shows-it-it-never-changes-what-a-number-means.md)
for what a plugin may and may not be. This build honours one kind of content: themes.

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
