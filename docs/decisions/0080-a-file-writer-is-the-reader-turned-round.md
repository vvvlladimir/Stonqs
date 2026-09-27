# 80: A file writer is the reader turned round

- Status: Accepted

## Context

The export writes one format, the app's own transaction file (ADR-0066). Somebody leaving for
another program, or handing operations to an accountant, needs that program's format, and there
are more of those than this repository will ever write. ADR-0086 built the machinery for the
opposite direction — a stranger's WASM component that turns bytes into the canonical file, with no
filesystem, no network, a frozen clock and a fixture it must pass before it installs — and left the
second contract undefined.

## Decision

- A second world, `stonqs:writer@1.0.0` (`app/src-tauri/wit/writer.wit`):
  `write(canonical: string) -> result<list<u8>, string>`. The input is a `stonqs.transactions`
  document; the row schema is again not restated in WIT. The error is the writer's own reason,
  shown as `UiError::Writer` naming the plugin.
- One sandbox for both. What `reader.rs` granted moves into `plugins/sandbox.rs` unchanged, and the
  reader and the writer are two contracts over it — a grant written twice drifts.
- The writer is handed what `transactions_export` already built: the screen's filter and scope,
  names instead of ids. The document is built and the store lock released before the module runs.
  It sees no portfolio and no path; the host writes the bytes where the user chose.
- The manifest declares `provides.writers: [{ id, name, file, sample, expected, extension }]`.
  Installing runs the writer on `sample` and requires exactly the bytes of `expected`. `name` is
  what the export menu shows; `extension` is the ending the saved file gets.
- `transactions_export_save` takes an optional `format` (`<plugin id>/<writer id>`); absent saves
  the canonical file as before.

## Alternatives

- **Hand the writer the store or a richer model.** A new grant for every format, and a surface
  that changes whenever the model does. The canonical file is already versioned and is what a
  reader produces, so both directions speak one language.
- **Return a media type and a suggested name.** The save dialog is shown before the writer runs,
  so the ending has to be known in advance; the manifest says it once.
- **Compare the expectation loosely, as a reader's is compared as JSON.** A reader's output is a
  document the app parses; a writer's is bytes another program reads, and that program is not
  lenient about which bytes.

## Consequences

- A writer adds no permission: no network, no filesystem, no portfolio access.
- Only the canonical file imports back. A plugin format is a one-way door, and the guide says so.
- Reports (`report_save`) are not writers yet: their input is not a transaction file, and they
  would need a document of their own before a plugin could render them.
