# 49: A protected profile keeps its database encrypted

- Status: Accepted

## Context

ADR-0048 sealed only the provider keys and left `portfolio.db` readable by anyone with the file,
which the lock screen could not change: it guarded the interface, not the data. The user asked
for the database itself to be encrypted behind the same password.

## Decision

- `rusqlite` is built on **SQLCipher** (`bundled-sqlcipher`). Apple targets use CommonCrypto; the
  others build OpenSSL from source (`bundled-sqlcipher-vendored-openssl`, a target-specific
  dependency in `core/Cargo.toml`), so no platform needs a system crypto library. An unencrypted
  file opens exactly as with plain SQLite.
- A protected profile's database is encrypted with a **random 256-bit key** sealed in its vault
  under the data key (`vault.json` → `db_key`), used raw (`PRAGMA key = "x'…'"`), so SQLCipher runs
  no key derivation of its own — the password was already stretched once, by Argon2id. The
  password changes without touching the database; the database key never changes.
- The core gains `Store::open_encrypted`, `Store::export_to` (`sqlcipher_export` into a new file,
  encrypted or plain) and `Store::checkpoint`. Converting a file is the host's `dbfile::convert`:
  export a complete copy, open it, move the original aside, move the copy in, open it there, then
  delete the original — with a rollback at each step. Setting a password encrypts; removing it
  decrypts. A vault holding a key beside a file that is still plain (an interrupted conversion, or
  a vault from ADR-0048) is encrypted on the next open instead of being reported unreadable.
- A locked profile's database is **not open at all**: the host holds an empty in-memory stand-in
  that `store()` never hands out. Background connections (quote refresh, AI turns) get path and
  key through `AppState::db_access`, which a locked profile refuses, and hold
  `AppState::db_in_use` while open; a conversion takes that gate exclusively and answers `busy`
  rather than swap a file under another connection. The venue lookup writes through the main
  connection after its network call instead of opening a second one.

## Alternatives

- **SQLCipher's own passphrase (PBKDF2 on every open).** A second, slower derivation of the same
  secret, and the password could not change without `PRAGMA rekey` over the whole file.
- **Encrypt the file ourselves, outside SQLite.** Decrypting to a temporary file puts the plain
  database on disk anyway, and WAL would have to be given up.
- **Encrypt every profile, with a keychain key when there is no password.** Protects nothing the
  OS account does not already protect, and brings the keychain prompts back for every profile.

## Consequences

- A forgotten password loses the portfolio of that profile, not only its keys. The UI says so
  where the password is set.
- `settings.json` (language, theme, dashboards, the dashboard summary texts) and the import layouts
  stay plain beside the encrypted database: the lock screen needs the language before any
  password. A summary tile's text can therefore be read from disk.
- The previous plain file is unlinked, not wiped: on an SSD its blocks may survive until reused.
- Builds are slower (SQLCipher; OpenSSL on non-Apple targets), and `cargo test` exercises the
  SQLCipher library on every platform.
