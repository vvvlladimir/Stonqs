# 47: A profile is a folder of its own

- Status: Accepted

## Context

One device may hold several independent sets of data (a personal and a family portfolio, a test
one), and the planned app password (stage 2 of the plan behind ADR-0046) is per profile. Until now
the host kept exactly one database in the app data directory, and every other file it writes —
`settings.json`, `import_templates.json`, `import_presets_hidden.json` — sits beside that database
(`db_path.with_file_name(...)`).

## Decision

- A profile is one folder, `profiles/<id>/`, holding a database and everything beside it. Because
  every host file is already derived from the database path, a profile needs no second path: the
  whole of it follows from `db_path`. `profiles.json` in the data directory lists the profiles
  (`id`, `name`, `created_at`) and remembers the last one opened.
- On the first start with profiles, the files of a single-profile install move into
  `profiles/default/` (fixed id, per-file atomic renames, registry written last), so an
  interrupted move completes on the next start.
- The host always has one profile open; the last one used is opened at start. Switching
  (`profile_open`) opens the new database **before** touching anything, then swaps store,
  portfolio, settings, scope and path under the store lock, cancels a running refresh and AI
  turn, and clears the per-run caches (model lists, import file, pending consents). Jobs already
  running keep the path they copied and finish into the profile they began in; a refresh writes
  its `last_refresh` only when that path is still the open one.
- The frontend reloads the window after a switch instead of invalidating queries: every cached
  answer, the UI state and the language belong to the profile being left. With more than one
  profile the app asks at launch, once per window (`sessionStorage`).
- AI provider keys stay device-wide in this step; they become per profile with the password.

## Alternatives

- **Restart the process on switch.** Simplest state handling, but `tauri dev` exits with the
  app, and mobile platforms do not allow an app to relaunch itself.
- **One database with a profile column.** Every query and every table would need the column, and
  deleting a profile would be a data migration rather than a folder removal.
- **Keep the old data in the data directory as a special profile.** Avoids the move, but leaves a
  profile that cannot be deleted like the others and a root folder that is both registry and data.

## Consequences

- `AppState::db_path` is read through a method; it changes only in `open_profile`.
- A file the host adds beside the database must also join `profiles::LEGACY_FILES`.
- Deleting a profile removes its folder — the only command that deletes a database file.
