#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release --locked
app='release/Apex Display Rust.app'
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" release/AppIcon.iconset
cp target/release/simdisplay "$app/Contents/MacOS/simdisplay"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" assets/icon.png --out "release/AppIcon.iconset/icon_${size}x${size}.png" >/dev/null
  double=$((size * 2))
  sips -z "$double" "$double" assets/icon.png --out "release/AppIcon.iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns release/AppIcon.iconset -o "$app/Contents/Resources/AppIcon.icns"
cp packaging/Info.plist "$app/Contents/Info.plist"
codesign --force --deep --sign - "$app"
ditto -c -k --sequesterRsrc --keepParent "$app" 'release/Apex-Display-Rust-macOS.zip'
printf 'Built %s\n' "$app"
