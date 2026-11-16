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

## Windows x86 build — 2026-11-16

Built the current widget styling (including rounded RPM lights) for `i686-pc-windows-msvc` with `scripts/package-windows.sh x86`, cargo-xwin 0.23.1, release/locked dependencies, and static CRT. Outputs: `release/windows-x86/simdisplay.exe` and `release/Apex-Display-Rust-Windows-x86.zip`.

`file`, PE header parsing, and `llvm-readobj` confirm a PE32 Intel i386 Windows GUI executable. Imports contain Windows system DLLs, without a dynamic MSVC CRT dependency. ZIP CRC, archived EXE equality, README equality and bundled font licenses were checked. ZIP SHA-256: `84492df1c0e6b154a6f82028ffda58d3e7263c4e82df0e03b77c7037a8c079a1`.

Build completed with LNK4099 warnings about unavailable Microsoft CRT debug PDB files. Windows runtime, live telemetry and fullscreen behavior still need verification on a Windows machine.

## ERS widgets and 2026 Season Pack — 2026-11-16

Added ERS battery energy (MJ), MGU-K electric motor power (kW), and ERS deployment mode (NONE/MEDIUM/HOTLAP/BOOST). The source is Car Status; BOOST is a deployment mode, not confirmation of 2026 Overtake eligibility. No battery capacity is assumed to derive a percentage.

Implemented EA's published 2026 structures alongside 2025: 24 cars; 59-byte Car Telemetry and Car Status records; adjusted pressure offset and Session/Lap/Damage packet lengths. Session History retains its existing layout. Reference: https://forums.ea.com/blog/f1-games-game-info-hub-en/ea-sports%E2%84%A2-f1%C2%AE25-2026-season-pack-udp-specification/12187347 (2026 Season Pack Telemetry Output Structures attachment).

Windows release test suite: 43 tests passed, including last-player offsets for both seasons, Joules-to-MJ and Watts-to-kW conversion, invalid values clearing stale data, expiry, 2026 UDP loopback, rendering units/missing values, and layout round-trips. The catalog scrolling UI test now waits for smooth scrolling to settle before selecting its final item.

A three-second local UDP probe received no game packets, so live-game values remain unverified.
## F1 25 v3 output catalog — 2026-11-16

Read the user's local 31-page Data Output from F1 25 v3.pdf and supplied C++ structures. Generated 402 selectable primitive/text output fields (including indexed arrays) across packet IDs 0–15. Together with 22 Common widgets there are 424 catalog entries. Header routing fields are not offered as widgets. Field metadata is derived data; the generator does not execute the source document.

Checked every packed packet size against the PDF: 1349, 753, 1285, 45, 1284, 1133, 1352, 1239, 1042, 954, 1041, 1460, 231, 273, 101, 1131 bytes. Values use little-endian decoding, signed integer/float support, UTF-8 names, event-union discrimination, actual array counts, and correct participant RGB offsets. Player selection is automatic for car arrays; Inspector can choose other cars and array items, and selection survives layout export/import. Session History and Tyre Sets remain scoped to the primary player.

Common is the default widget category. Category browsing and global word-based search expose the expanded catalog. Each new widget displays one selected scalar or text value, including source units and inline documented enum names; appendix-only IDs remain numeric codes. All output fields have Demo and missing-data rendering. The expanded catalog targets format 2025; existing 2026 Common support remains separate.

Windows checks passed: cargo fmt --check; cargo test --release --locked (51 tests); cargo clippy --release --locked --all-targets -- -D warnings; cargo build --release --locked. Tests cover all packet sizes and truncation, final-car offsets, per-car UTF-8/RGB arrays, event union clearing, multidimensional lap positions and valid counts, session reset and slow/event-driven field retention, every new widget's Demo/missing rendering and JSON round-trip, invalid indices, and UI search/add interaction.

Live-game packets have not yet verified this expanded catalog.

## Assetto Corsa shared memory — 2026-11-16

Added the original Assetto Corsa Windows receiver with automatic retry and LIVE/PAUSED/REPLAY/WAITING/ERROR status. It opens existing Local\acpmf_physics, Local\acpmf_graphics, Local\acpmf_static mappings using FILE_MAP_READ. It never creates game mappings. The production receiver checks for acs.exe or acs_x86.exe to distinguish AC from other games with the same mapping names. Missing games, incomplete connections and frozen counters return empty WAITING snapshots. Volatile page copies compare packet counters with bounded retries; this is a best-effort snapshot per page, not a transaction across all pages.

Added 130 selectable fields: AC Physics (59), AC Graphics (29), AC Static (42). Metadata follows pack(4) and Windows UTF-16, with page sizes 580/296/684 bytes. Contact vectors use wheel[4][XYZ=3], correcting the Python helper's nested multiplication order. Python generates checked-in metadata only; the runtime is Rust.

Common mapping covers gear conversion (AC 0/1/2 to R/N/1), litres, pedal percentages, RPM/max RPM, lap timing, lap count, position, session time conversion to seconds, tyre core temperatures/pressures in FL/FR/RL/RR order, brake temperatures, compound and DRS for supported cars. TC, ABS and tyre wear remain raw AC fields. Unsupported Common ERS energy/power/mode remain missing. KERS charge is offered as a percentage without inferring stored energy from per-lap deployment limits.

Windows validation passed: cargo fmt --check; cargo test --release --locked (58 tests); cargo clippy --release --locked --all-targets -- -D warnings; cargo build --release --locked. Coverage includes isolated real Windows named mappings, missing/partial connection cleanup, subsequent updates and reconnect, UTF-16 Thai/surrogate pairs, 4x3 contact arrays, wheel order, unit/gear/time conversion, invalid numbers, missing capabilities, stale-counter recovery, status handling, widget Demo/missing rendering and layout round-trips, and UI selection of AC and return to Demo.

References:
- https://www.assettocorsamods.net/threads/doc-shared-memory-reference.58/
- https://github.com/ac-custom-shaders-patch/acc-extension-apps/blob/master/apps/python/AccExtHelper/sim_info.py

Actual AC driving-session telemetry remains unverified.
