#!/bin/sh
# Cargo runner on macOS (see .cargo/config.toml). Signs the app binary with a stable identity so
# the keychain keeps its "Always Allow" across rebuilds; without the identity it only runs it.
# Create the identity once with app/scripts/dev-signing-cert.sh.
identity="${STONQS_DEV_SIGNING_IDENTITY:-Stonqs Dev}"
if [ "$(basename "$1")" = "sq-app" ] && security find-identity -p codesigning | grep -q "\"$identity\""; then
  codesign --force --sign "$identity" "$1" 2>/dev/null || echo "dev-run: could not sign with \"$identity\"" >&2
fi
exec "$@"
