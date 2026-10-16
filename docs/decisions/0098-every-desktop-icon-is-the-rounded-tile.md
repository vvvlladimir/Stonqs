# 98: Every desktop icon is the rounded tile

- Status: Accepted

## Context

`app/scripts/make-icons.sh` squircled the macOS icon only and gave every other target the square
`app-icon.svg`. On a Windows taskbar and desktop and in a Linux dock that square is a hard-edged
block beside rounded neighbours, and the Store package of ADR-0096 needs the same art again in its
own sizes.

## Decision

Windows and Linux use the macOS squircle too, cropped tighter: the same tile with its `viewBox`
set to `72 72 880 880`, so the squircle fills 824 of 880 rather than 824 of 1024. The bundler's
PNGs and `icon.ico`, the `Square*Logo`/`StoreLogo` files and the Store logos in
`app/src-tauri/msix/Images/` are rendered from it. iOS and Android keep the square art: both mask
the icon themselves, and iOS forbids transparency.

## Alternatives

- The macOS grid unchanged: one fewer variant, but Windows and Linux draw an icon to the edge of
  its box, so the macOS margin would leave the tile a fifth smaller than its neighbours.

## Consequences

The window, the installers and the Store package show one rounded shape; the `.icns` keeps the
wider macOS margin. Two dozen generated PNGs follow the drawing with nothing to say when one
drifts: redrawing the logo means rerunning `make-icons.sh`.
