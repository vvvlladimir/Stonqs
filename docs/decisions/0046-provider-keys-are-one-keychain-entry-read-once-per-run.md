# 46: Provider keys are one keychain entry, read once per run

- Status: Superseded by ADR-0048 (where keys are stored); the dev-signing runner still holds

## Context

ADR-0037 put each provider's key in its own OS keychain entry (account = provider id) and read
it fresh before every call. On macOS every read of a keychain item can raise a password prompt,
and the item's access list is tied to the reading binary's code signature. The development binary
carries the linker's ad-hoc signature, which changes with every build, so "Always Allow" was
forgotten at each rebuild. Reads also happened far more often than calls: `ai_providers_list`
and `ai_key_status` checked presence by reading the secret itself. The result was one prompt per
provider, again and again, in a single session.

## Decision

- All keys live in **one** entry: service `app.stonqs.desktop.ai`, account `keys`, a JSON object
  `{provider: key}`. An empty set removes the entry.
- The entry is read **once per run** and kept in the host's memory (`ai::keys`). Presence is
  answered from that copy. The lock around it is held across the read, so concurrent callers wait
  for one prompt instead of raising several. A refused read is not cached; a write updates the
  copy only after the keychain accepted it.
- Old per-provider entries are moved into the new one the first time it is missing: written
  there first, then removed.
- Development builds are re-signed with a stable local identity by a Cargo runner
  (`.cargo/config.toml` → `app/scripts/dev-run.sh`), created once per machine by
  `app/scripts/dev-signing-cert.sh`. Without that identity the runner only runs the binary.

Still true from ADR-0037: no command returns a key, and nothing secret is written to the
database or `settings.json`.

## Alternatives

- **Cache per provider, keep one entry each.** Still one prompt per provider per run.
- **Check presence with attributes only.** Avoids reading the secret, but still one item per
  provider, and the call itself reads it anyway.
- **Our own encrypted store behind an app password.** The planned next step (profiles with an
  optional password, keys encrypted with a key derived by Argon2id, the keychain kept only as an
  optional "remember on this device"). Larger; this decision removes the repeated prompts now and
  its single entry is what that "remember" option will hold.

## Consequences

- At most one keychain prompt per launch, and none once "Always Allow" is granted to a stably
  signed binary.
- A key lives in the process's memory for the whole run rather than only around a call.
- Every key is read when any one is needed; for a handful of providers this costs nothing.
- Cargo runs every macOS binary through the runner script (it signs only `sq-app`).
