# Verification — 2026-11-11

Environment: macOS Apple Silicon, Rust 1.99.1; eframe/egui 0.33.3, dependencies locked in Cargo.lock.

## Confirmed

- Rust test suite: 36 tests covering original/legacy layouts, invalid imports, RPM, geometry, bounded history, editing commands, real egui pointer interaction, per-field expiry, all six F1 packet kinds, UDP loopback/rebinding, storage restart/recovery, and monitor placement calculations.
- Continuous canvas dragging and opacity slider changes undo in one step. Real filesystem permission failures exercise preservation and retries. Reintroducing the old premature-save/reset acknowledgement reproduced failures in the recovery tests.
- Integer-valued JSON float dimensions normalize correctly; Unicode labels retain the complete valid UTF-16 prefix.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and release build passed.
- Existing Electron project unit suite: 21 tests passed; original source changes were not overwritten.
- Original default, all-widget, and legacy fixtures passed JS → Rust → JS round trips with deep equality after both validators' migrations.
- Native app launched on macOS. App-owned framebuffer captures after 30 frames verified stable dark theme, Thai/English labels, dashboard widgets, and wide canvas. See [studio](studio.png) and [ultrawide](ultrawide.png).
- macOS .app and ZIP created; application signed ad hoc locally. No Developer ID/notarization or publishing.
- Fresh independent code review completed. Important findings about undo grouping, monitor placement, save acknowledgement/retry, and unreadable files were addressed and covered by regression tests.

## Not established by these checks

- F1 25 from an actual running game: packet and loopback tests use synthetic data.
- Installation or runtime on Windows: manual GitHub workflow is supplied but was not run remotely.
- Physical mixed-DPI monitors, disconnect/reconnect, or a four-monitor rig: coordinate tests are deterministic; hardware verification remains needed.
- Actual native file-dialog interaction has not been automated. Native fullscreen entry and Escape were verified on the connected macOS display; mixed-monitor transitions remain unverified.
- OS window screenshot capture was unavailable. Captures use this application's own framebuffer only.

## Implementation decisions

- Work is on `feat/simdisplay-rust` in the requested checkout, rather than a separate worktree; changes are scoped to the new Rust project, its docs, and its build workflow. The original dirty working-tree files remain intact.
- Execution continued from the user's approved design/start instruction without an additional plan-approval pause.
- Unicode truncation and stale canvas dimensions were fixed because they could lose entered text or misrepresent the result of Undo.
- Integer-valued JSON floats are normalized for compatibility with JavaScript's numeric model.
- The Rust UI recreates the existing workflow and theme; it is not a pixel-identical HTML rendering. Standard exported layouts retain schema compatibility.

## Remaining minor limitations

- Atomic saves sync file contents and replace via a same-directory temporary file, but do not fsync the parent directory. Ordinary process errors preserve the old file; power-loss guarantees are weaker.
- Hand-authored non-RPM widgets with arbitrary `rpmConfig` types may be accepted by the old JavaScript validator and rejected by typed Rust. Standard exports have no such field on non-RPM widgets.

## Reproduce visual checks

```sh
cargo run --example capture -- /tmp/studio.png
cargo run --example capture -- /tmp/wide.png 1920 480
```

The capture example uses a temporary workspace and renders only its own application window.

## Race-display freeze regression

The launch handler called `Context::pixels_per_point()` / `zoom_factor()` inside `Context::input()`, recursively acquiring egui's context lock when native window bounds were available. The fix snapshots those bounds before reading scale. A bounded regression test timed out before the fix and passes afterward, checking entry commands and Escape. Earlier headless editor checks did not populate native outer/inner bounds and missed this branch.

`cargo run --locked --example fullscreen_smoke` now exercises the actual native window in an isolated temporary workspace: synthetic click on the race-display button, observed native fullscreen, synthetic Escape, observed return to windowed mode, clean exit. This passed on macOS on the connected display. Release binary and macOS .app/ZIP were rebuilt after the fix.

## Separate lap widgets

Best lap (`bestLap`) and Last lap (`lastLapTime`, mapped to the `lastLap` telemetry field) are covered by layout round-trip, precise time rendering, missing-data, and warning-threshold tests. The existing `lastLap` widget remains the combined Lap times display. New widget types are a Rust extension; the original Electron application cannot import layouts containing them.

## DRS and driver assists

Added DRS active, Traction control, and ABS. Verified the selected-player offsets and field meanings against EA F1 25 v3 pages 10–11. Tests cover DRS actual state independent of DRS allowed, TC/ABS levels including zero, unknown codes clearing old values, per-field expiry, rendered states, Demo, and layout round-trips. Full suite: 36 tests. TC/ABS report configured assist settings, not instantaneous intervention. Real-game verification is still pending.

## Windows x64 cross-build

On 2026-11-11, `scripts/package-windows.sh` successfully built the current Rust source for `x86_64-pc-windows-msvc` on macOS using cargo-xwin 0.23.1 and LLVM/LLD 23.1.5, with `--release --locked` and static CRT. Outputs are `release/windows-x64/simdisplay.exe` and `release/Apex-Display-Rust-Windows-x64.zip`.

`file` and `llvm-readobj` confirmed a PE32+ AMD64 Windows GUI executable. Import inspection found Windows system DLLs and no dynamic MSVC CRT dependency. The linker reported LNK4099 warnings for unavailable Microsoft CRT debug PDB files; the release build completed successfully. ZIP CRC checks and equality of the archived executable with the build output passed. Windows runtime, actual game input, and Windows fullscreen behavior have not been tested on a Windows machine.

Windows icon rebuild: user-supplied Neon Sim Racing Dashboard artwork center-cropped to a square for the window icon and a seven-size ICO. The rebuilt EXE contains ICON and GROUP_ICON resources, and all seven image payloads match the ICO. Repackaged ZIP passed CRC and executable/README equality checks. Windows Explorer/taskbar appearance still requires a Windows runtime check.
