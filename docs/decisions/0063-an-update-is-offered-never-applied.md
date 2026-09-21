# 63: An update is offered, never applied

- Status: Accepted

## Context

The app is distributed as a downloaded installer, outside any store. Without a way to update
itself, every fix reaches only the people who happen to visit the release page again, and a
security fix reaches almost nobody.

Tauri ships an updater plugin, so the question is not whether one can be built but what it is
allowed to do on its own. An updater is the one component that replaces the running binary: it is
the most valuable thing in the project to compromise, and the most annoying thing to get wrong for
the user. Both risks are decided by the same two choices — what the update must prove about itself,
and who presses the button.

## Decision

Updates are fetched from the project's GitHub releases and are **verified against a key compiled
into the binary** (`plugins.updater.pubkey`). Nothing else is accepted, whatever the endpoint
returns. The private half signs the artifacts in the release workflow and exists nowhere else; it
has no expiry and cannot be rotated without stranding every copy already installed, which is why
the key is created before the first public release rather than when updates are first wanted.

Every step is the user's: the app checks once a calendar day, shows what the release says about
itself, and downloads nothing until `Update now`. `Skip this version` is remembered, the automatic
check has an off switch, and `Check for updates` works whatever that switch says. The decision and
the wording live in the frontend (`lib/updates.tsx`, `components/domain/UpdateDialog.tsx`); the host
adds the plugin and nothing else. An automatic check that fails is silent — being offline is not an
error the user caused — while a check they pressed says so.

The updater is desktop-only, behind `cfg(desktop)`: on iOS and Android a store does this, and the
OS does not let an app replace its own bundle.

The release feed is a draft release published by hand. `latest.json` becomes reachable only on
publication, so a build nobody has looked at cannot be offered to anybody.

## Alternatives

- **Silent background updates.** Fewer prompts, and what most browsers do. Rejected: this app is
  installed unsigned by platform standards today, it holds financial records, and a version
  changing underneath someone mid-session is not something they can undo.
- **No updater; a download page and a release feed.** Nothing to compromise and nothing to
  maintain. Rejected: it makes every future fix optional in practice.
- **Checking on every start rather than once a day.** Rejected as noise; a daily check finds a
  release within a day of it existing, which is fast enough for software nobody is paid to watch.
- **A version pinned in `tauri.conf.json`.** Rejected in the same change: the version now comes
  from the workspace `Cargo.toml`, so the tag, the binary and the feed cannot disagree — and the
  release workflow refuses a tag that does not match it.

## Consequences

- The signing key is a permanent asset. Losing it ends updates for every installed copy; leaking it
  hands somebody else the ability to install code on every installed copy. It lives in repository
  secrets and in a password manager, and nowhere in the repository.
- The endpoint contains the repository's owner and name. Renaming either breaks the feed for copies
  already in the field, so the name is fixed before the first release.
- `.deb` and `.rpm` are not updated this way — a package manager owns those files. AppImage, the
  Windows installers and the macOS bundle are.
- On macOS an update writes where the app is installed, so a copy running from a read-only or
  user-owned location can fail; the failure says so and changes nothing.
- A user who declines an update is not asked again for that version, so a release worth insisting on
  needs a new version, not a repeated prompt.
