#!/bin/sh
# Cross-build a portable Windows app: package-windows.sh [x64|x86].
# Requires the corresponding Rust MSVC target, cargo-xwin, and LLVM/LLD on PATH.
set -eu
cd "$(dirname "$0")/.."
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C target-feature=+crt-static"
export SIMDISPLAY_WINDOWS_ARCH="${1:-x64}"
case "$SIMDISPLAY_WINDOWS_ARCH" in
  x64) export XWIN_ARCH=x86_64; target=x86_64-pc-windows-msvc ;;
  x86) export XWIN_ARCH=x86; target=i686-pc-windows-msvc ;;
  *) echo "Usage: $0 [x64|x86]" >&2; exit 2 ;;
esac
cargo xwin build --release --locked --target "$target" --bin simdisplay
out="release/windows-$SIMDISPLAY_WINDOWS_ARCH"
mkdir -p "$out/licenses"
cp "target/$target/release/simdisplay.exe" "$out/simdisplay.exe"
cp README.md "$out/README.md"
cp assets/fonts/OFL-*.txt "$out/licenses/"
python3 - <<'PY'
import os
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
arch = os.environ['SIMDISPLAY_WINDOWS_ARCH']
folder = Path(f'release/windows-{arch}')
archive = Path(f'release/Apex-Display-Rust-Windows-{arch}.zip')
with ZipFile(archive, 'w', ZIP_DEFLATED) as z:
    for item in [folder/'simdisplay.exe', folder/'README.md', *sorted((folder/'licenses').glob('OFL-*.txt'))]:
        z.write(item, item.relative_to(folder))
print(f'Built {folder / "simdisplay.exe"}')
print(f'Packaged {archive}')
PY
