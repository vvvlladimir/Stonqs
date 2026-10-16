# 97: The Store copy asks the Store for its update and sends the user there

- Status: Accepted

## Context

A copy installed from the Microsoft Store (ADR-0096) runs from a read-only `WindowsApps` directory.
The updater plugin would offer it every release from the feed and then fail to replace the binary.
A release also reaches the Store only after certification, so the feed names versions the Store
cannot install yet. The Store answers the question for a packaged desktop app through
`StoreContext.GetAppAndOptionalStorePackageUpdatesAsync`, with no window handle needed for the look.

## Decision

`update_store_check` decides by the executable's path: under a `WindowsApps` directory it asks
`StoreContext` and answers the newer package's version (`major.minor.build`, `release.yml` writing
the release as `major.minor.patch.0`), anywhere else it answers nothing and the frontend asks the
updater plugin as before. The once-a-day schedule, the switch and `Skip this version` are the same
for both.

A Store offer has no notes and no download. Its button, `Update in the Microsoft Store`, calls
`update_store_open`, which opens `ms-windows-store://pdp/?PFN=<package family name>` through
`Launcher.LaunchUriAsync`: the app's page in the Store app, whose own Update button installs it.
The family name comes from the running package, so the Store's product id is written down only in
`store.yml`. `sq-app` depends on `windows` 0.61, on Windows only, the version Tauri already locks.

## Alternatives

- The release feed with a link to the Store: no new dependency, but it names a release before the
  Store holds it, sending the user to a page with nothing to update.
- Installing in place through `RequestDownloadAndInstallStorePackageUpdatesAsync`: one click fewer,
  but needs the window's `HWND` through `IInitializeWithWindow` and closes the app mid-session.
- Detecting the package through `GetCurrentPackageFullName`: the documented test, but the path
  already answers it without a call.

## Consequences

A Store user sees a new version only once the Store can install it, and the click lands on the
button that does. The Store copy makes no request to GitHub for updates. The offer carries no
release notes. The Windows-only code is compiled only by the Windows legs of CI (nightly) and of
the release.
