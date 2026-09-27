# 87: A reader that breaks is passed over, and one that refuses stops the import

- Status: Accepted

## Context

ADR-0086 made every failure of a plugin's file reader an error of the import: refused, trapped,
timed out, out of memory. A reader that declares no file endings is offered every file the shipped
readers do not recognise, so one whose module traps on input it does not know — before it could
answer `not-mine` — refused every CSV the user opened, and the only way out was to find and remove
the plugin. A trap says nothing about whose file it was; `malformed` does.

## Decision

The two are told apart (`reader::Refusal`). `Malformed` — the reader's own answer that the file is
its kind and cannot be read — still stops the import as `UiError::Reader`, because the reader after
it would be reading a file somebody has said is theirs. `Broken` — the module did not start,
trapped, ran past its deadline or its memory — is passed over like `not-mine`: the file goes on to
the next reader and then to the app's own, and the wizard names each skipped reader
(`ImportPreviewData::skipped_readers`: its key and the runtime's detail; the sentence is the
frontend's). The install check is unchanged: a reader that breaks on its own sample is refused.

## Alternatives

- **Keep the hard error.** One broken package blocks every import, which is the failure this
  replaces.
- **Skip silently.** The file might then be read by the CSV reader as something it is not, with
  nothing saying a plugin meant for it failed.
- **Disable a reader after it breaks.** State the list would have to explain and a restart would
  have to clear; one banner per file says the same without it.

## Consequences

- The consequence in ADR-0086 that listed "trapped, timed out, out of memory" as import failures is
  narrowed to install failures; `malformed` and `needs-password` remain the import's.
- A reader that traps on its own kind of file now leaves that file to the CSV reader, which usually
  fails to map it — with the banner above the result saying which plugin should have read it.
