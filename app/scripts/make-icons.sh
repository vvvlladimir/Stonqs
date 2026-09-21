#!/usr/bin/env bash
# Rebuild the complete icon set from app-icon.svg.
#
# `tauri icon` reads SVG, but does not cover two platform requirements:
#
#  1. iOS forbids alpha in icons; remove the remaining PNG alpha after the
#     CLI applies --ios-color.
#  2. macOS expects an 824x824 squircle inside a 1024 canvas with transparent
#     margins, so the .icns uses a separately masked image.
#
# Requires: rsvg-convert (brew install librsvg), python3, iconutil (macOS).

set -euo pipefail

cd "$(dirname "$0")/.."
SRC="app-icon.svg"
BG="#16171b"          # background matching the SVG
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# 1. Base set: desktop + Windows Store + iOS + Android adaptive icons.
npx tauri icon "$SRC" --ios-color "$BG"

# 2. iOS: remove alpha by replacing transparency with the background.
for f in src-tauri/icons/ios/*.png; do
  magick "$f" -background "$BG" -alpha remove -alpha off PNG24:"$f"
done

# 3. macOS: 824x824 inside 1024 with a squircle mask (a close approximation
#    of Apple's curve; a normal rounded rectangle looks noticeably sharper).
rsvg-convert -w 824 -h 824 "$SRC" -o "$WORK/art824.png"
python3 - "$WORK" <<'PY'
import base64, math, sys

work = sys.argv[1]
n, r, pad = 5.0, 412.0, 100.0
points = []
for i in range(720):
    t = 2 * math.pi * i / 720
    ct, st = math.cos(t), math.sin(t)
    x = math.copysign(abs(ct) ** (2 / n), ct) * r + r + pad
    y = math.copysign(abs(st) ** (2 / n), st) * r + r + pad
    points.append(f"{x:.3f},{y:.3f}")

art = base64.b64encode(open(f"{work}/art824.png", "rb").read()).decode()
open(f"{work}/icon-macos.svg", "w").write(
    '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
    'width="1024" height="1024" viewBox="0 0 1024 1024">'
    f'<defs><clipPath id="sq"><polygon points="{" ".join(points)}"/></clipPath></defs>'
    f'<image clip-path="url(#sq)" x="100" y="100" width="824" height="824" '
    f'xlink:href="data:image/png;base64,{art}"/></svg>'
)
PY

ICONSET="$WORK/Portfolio.iconset"
mkdir -p "$ICONSET"
for s in 16 32 64 128 256 512 1024; do
  rsvg-convert -w "$s" -h "$s" -b none "$WORK/icon-macos.svg" -o "$WORK/px_$s.png"
done
cp "$WORK/px_16.png"   "$ICONSET/icon_16x16.png"
cp "$WORK/px_32.png"   "$ICONSET/icon_16x16@2x.png"
cp "$WORK/px_32.png"   "$ICONSET/icon_32x32.png"
cp "$WORK/px_64.png"   "$ICONSET/icon_32x32@2x.png"
cp "$WORK/px_128.png"  "$ICONSET/icon_128x128.png"
cp "$WORK/px_256.png"  "$ICONSET/icon_128x128@2x.png"
cp "$WORK/px_256.png"  "$ICONSET/icon_256x256.png"
cp "$WORK/px_512.png"  "$ICONSET/icon_256x256@2x.png"
cp "$WORK/px_512.png"  "$ICONSET/icon_512x512.png"
cp "$WORK/px_1024.png" "$ICONSET/icon_512x512@2x.png"
iconutil -c icns "$ICONSET" -o src-tauri/icons/icon.icns

# 4. Dev-server and window favicon: the same file served by Vite.
cp "$SRC" public/app-icon.svg

echo "icons rebuilt"
