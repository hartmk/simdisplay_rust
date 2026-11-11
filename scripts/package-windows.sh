#!/bin/sh
# Cross-build a portable Windows x64 app from macOS/Linux with cargo-xwin.
# Requires Rust, the x86_64-pc-windows-msvc target, cargo-xwin, and LLVM/LLD on PATH.
set -eu
cd "$(dirname "$0")/.."
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C target-feature=+crt-static"
export XWIN_ARCH=x86_64
cargo xwin build --release --locked --target x86_64-pc-windows-msvc --bin simdisplay
out=release/windows-x64
mkdir -p "$out/licenses"
cp target/x86_64-pc-windows-msvc/release/simdisplay.exe "$out/simdisplay.exe"
cp README.md "$out/README.md"
cp assets/fonts/OFL-*.txt "$out/licenses/"
python3 - <<'PY'
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
folder = Path('release/windows-x64')
archive = Path('release/Apex-Display-Rust-Windows-x64.zip')
with ZipFile(archive, 'w', ZIP_DEFLATED) as z:
    for item in [folder/'simdisplay.exe', folder/'README.md', *sorted((folder/'licenses').glob('OFL-*.txt'))]:
        z.write(item, item.relative_to(folder))
print(f'Built {folder / "simdisplay.exe"}')
print(f'Packaged {archive}')
PY
