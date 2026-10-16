#!/usr/bin/env bash
# Rebuild the complete icon set from app-icon.svg.
#
# `tauri icon` reads SVG, but does not cover three platform requirements:
#
#  1. macOS expects an 824x824 squircle inside a 1024 canvas with transparent
#     margins, so the .icns uses a separately masked image.
#  2. Windows and Linux show the same rounded tile, cropped tighter: they draw an
#     icon to the edge of its box, so the macOS margin would leave it a fifth
#     smaller than its neighbours (ADR-0098).
#  3. iOS forbids alpha in icons, and iOS and Android mask the icon themselves:
#     they keep the square art, with the remaining PNG alpha removed.
#
# Requires: rsvg-convert (brew install librsvg), python3, iconutil (macOS), magick.

set -euo pipefail

cd "$(dirname "$0")/.."
SRC="app-icon.svg"
BG="#16171b"          # background matching the SVG
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# 1. The squircle: 824x824 inside 1024 (a close approximation of Apple's curve;
#    a normal rounded rectangle looks noticeably sharper). The same tile with its
#    viewBox cut to 880 is the Windows and Linux icon.
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
body = (
    f'<defs><clipPath id="sq"><polygon points="{" ".join(points)}"/></clipPath></defs>'
    f'<image clip-path="url(#sq)" x="100" y="100" width="824" height="824" '
    f'xlink:href="data:image/png;base64,{art}"/></svg>'
)
head = '<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" '
open(f"{work}/icon-macos.svg", "w").write(
    head + 'width="1024" height="1024" viewBox="0 0 1024 1024">' + body
)
open(f"{work}/icon-rounded.svg", "w").write(
    head + 'width="1024" height="1024" viewBox="72 72 880 880">' + body
)
PY

# 2. Base set: iOS and Android from the square art, the desktop and Store files
#    from the rounded tile.
npx tauri icon "$SRC" --ios-color "$BG"
npx tauri icon "$WORK/icon-rounded.svg" -o "$WORK/rounded" >/dev/null
for f in "$WORK"/rounded/*.png "$WORK"/rounded/icon.ico; do
  cp "$f" src-tauri/icons/
done

# 3. iOS: remove alpha by replacing transparency with the background.
for f in src-tauri/icons/ios/*.png; do
  magick "$f" -background "$BG" -alpha remove -alpha off PNG24:"$f"
done

# 4. macOS: the squircle with its full margin.
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

# 5. Microsoft Store package logos (ADR-0096), from the rounded tile: every scale
#    and target size, so `makepri` finds the closest one for each surface.
IMAGES=src-tauri/msix/Images
mkdir -p "$IMAGES"
logo() { rsvg-convert -w "$2" -h "$2" -b none "$WORK/icon-rounded.svg" -o "$IMAGES/$1.png"; }
for scale in 100 200 400; do
  logo "Square150x150Logo.scale-$scale" $((150 * scale / 100))
  logo "Square44x44Logo.scale-$scale" $((44 * scale / 100))
  logo "StoreLogo.scale-$scale" $((50 * scale / 100))
done
for size in 16 24 32 48 256; do
  logo "Square44x44Logo.targetsize-$size" "$size"
  cp "$IMAGES/Square44x44Logo.targetsize-$size.png" "$IMAGES/Square44x44Logo.targetsize-${size}_altform-unplated.png"
done

# 6. Dev-server and window favicon: the same file served by Vite.
cp "$SRC" public/app-icon.svg

echo "icons rebuilt"
