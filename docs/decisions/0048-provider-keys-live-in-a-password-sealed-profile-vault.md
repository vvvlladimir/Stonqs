# 48: Provider keys live in a password-sealed profile vault

- Status: Accepted; "the database stays unencrypted" superseded by ADR-0049

## Context

ADR-0046 kept every provider key in one device-wide keychain entry. With profiles (ADR-0047) a
key must belong to a profile, and the user wants one password at the entrance rather than
keychain prompts during work. The agreed scope: the password protects the **keys only** — the
portfolio database stays unencrypted — and the OS keychain remains available as an optional
"remember on this device".

## Decision

- A profile's password is optional. Its presence is `vault.json` in the profile's folder. A key
  can be saved only behind a password: `ai_key_save` on an unprotected profile is
  `password_required`, and the UI asks for a password first.
- The password is stretched with **Argon2id** (64 MiB, 3 passes, 1 lane; parameters and a
  16-byte salt stored in the file) into a key-encryption key. That key seals a random 32-byte
  **data key** with **XChaCha20-Poly1305**; the data key seals the JSON of provider keys. Every
  ciphertext's associated data names its purpose and the profile id, so a blob moved to another
  field or profile does not open. Changing the password re-seals only the data key.
- Unlocked, the data key and the keys live in host memory (`AppState::vault`, wiped on drop)
  until the profile is locked or left. A protected profile opens **locked**: `AppState::store()`
  refuses with `locked`, which stops nearly every command, the frontend shows the lock screen
  instead of the app, and the startup refresh waits for the unlock. The password is asked once
  per run and never during AI work.
- "Remember on this device" stores the **data key** (never the password) in the OS keychain
  (`app.stonqs.desktop.ai`, account `profile:<id>`); a profile it opens starts unlocked. Deleting the
  profile or removing its password forgets it.
- Removing the password removes the keys with it. There is no recovery: a forgotten password
  loses the keys, not the portfolio.
- The device-wide keys of ADR-0046 stay readable by the profile a single-profile install became
  (`default`) until that profile gets a password; they then move into its vault and leave the
  keychain. No other profile ever sees them, and nothing new is written there.

## Alternatives

- **Encrypt the whole database (SQLCipher).** Real protection of the portfolio at rest, but it
  touches the core's `Store::open`, needs a migration of existing files and a key before any
  read. Deferred by the user's choice; the lock is a guard of the interface, not of the file.
- **Keep keys in the keychain, per profile, with the password only as a UI gate.** The keychain
  prompts this work set out to remove would return, and the password would protect nothing.
- **Tauri Stronghold.** Deprecated, to be removed in Tauri v3.
- **Password stored as a hash and checked.** A check proves knowledge but protects nothing; with
  the data key sealed by the password, a failed authentication *is* the wrong-password answer.

## Consequences

- New error codes cross IPC: `locked`, `password_required`, `wrong_password`.
- The key helpers moved from `ai::keys` to `AppState` (`secrets.rs`); `ai::keys` only reads the
  pre-password device entry.
- Unlocking costs a deliberate fraction of a second; the commands doing it are async so it runs
  off the main thread.
- The database of a protected profile is still readable by anyone with the file.
