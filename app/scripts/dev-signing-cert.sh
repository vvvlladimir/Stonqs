#!/bin/sh
# One-time, per machine: a self-signed code-signing identity for development builds, imported into
# the login keychain. Afterwards `pnpm tauri dev` signs sq-app with it (app/scripts/dev-run.sh), so
# a keychain "Always Allow" survives rebuilds. Trusted on this machine only; never for release.
set -e
name="${STONQS_DEV_SIGNING_IDENTITY:-Stonqs Dev}"
if security find-identity -p codesigning | grep -q "\"$name\""; then
  echo "\"$name\" already exists."
  exit 0
fi
dir="$(mktemp -d)"
trap 'rm -rf "$dir"' EXIT
openssl req -x509 -newkey rsa:2048 -days 3650 -nodes \
  -keyout "$dir/dev.key" -out "$dir/dev.crt" -subj "/CN=$name" \
  -addext "keyUsage=critical,digitalSignature" -addext "extendedKeyUsage=codeSigning"
# -legacy: the default p12 cipher imports without error but leaves the key unusable for signing.
openssl pkcs12 -export -legacy -in "$dir/dev.crt" -inkey "$dir/dev.key" \
  -out "$dir/dev.p12" -password pass:dev
security import "$dir/dev.p12" -k "$HOME/Library/Keychains/login.keychain-db" -P dev -T /usr/bin/codesign
# No trust setting is needed: codesign signs with an untrusted identity, and the keychain matches
# the designated requirement (identifier + certificate leaf hash), which stays the same per build.
echo "Imported \"$name\"."
