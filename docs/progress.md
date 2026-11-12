# Hover Progress Log

## 2026-04-27 - Checklist and reference audit start

What changed:
- Read the full source checklist from `hover_agent_checklist_zero_resource.md`.
- Cloned `https://github.com/cosmokud/rust-hover-preview` into `reference/rust-hover-preview`.
- Read the reference `README.md`, `Cargo.toml`, `CHANGELOG.md`, and source module layout.
- Built the reference project successfully with `cargo build`.
- Created `docs/reference-rust-hover-preview-audit.md` before major Hover implementation work.

Files touched:
- `reference/rust-hover-preview/`
- `docs/reference-rust-hover-preview-audit.md`
- `docs/progress.md`

Resource impact:
- Repository clone and reference build are one-time development actions only.
- No Hover resident app code exists yet, so no Hover idle resource impact has been introduced.

Tests run:
- `cargo build` in `reference/rust-hover-preview` passed.

Checklist items completed:
- Read the full checklist before coding.
- Cloned the reference repository into the required folder.
- Read required reference docs and source modules.
- Built the reference project locally.
- Created the required reference audit document.

Blockers:
- [blocked] Reference runtime hover behavior was not manually observed because this tool session cannot drive File Explorer hover/navigation and inspect tray interactions interactively. The audit records source-grounded behavior instead.

## 2026-04-27 - Workspace, core, decode, render, browser host, and native shell scaffold

What changed:
- Created a Rust workspace with `hover-core`, `hover-settings`, `hover-decode`, `hover-render`, `hover-win`, and `hover-browser-host` crates.
- Implemented zero idle work state accounting, cursor-centered zoom math, pan clamp, fit scale helpers, monitor-aware preview placement, hard-capped LRU cache, cancellation tokens, and replacement decode slots.
- Implemented settings defaults and a lossy parser that forces telemetry off, idle polling to zero, and eligible polling to 30 Hz maximum.
- Implemented image header sniffing, hard decode limits, a WIC-first decode module, and decoded bitmap byte accounting.
- Implemented dirty render state and render-plan math so hidden/static states produce zero hidden render presents.
- Implemented a native messaging host skeleton that validates browser URLs without persisting URL history.
- Implemented a Manifest V3 browser extension skeleton that detects `img` and image-link hover targets and sends native messages.
- Implemented the first Windows native shell: blocking message loop, `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`, Dormant default, conditional cursor timer for eligible foreground contexts, and a hidden no-activate topmost overlay window.

Files touched:
- `Cargo.toml`
- `README.md`
- `crates/hover-core/**`
- `crates/hover-settings/**`
- `crates/hover-decode/**`
- `crates/hover-render/**`
- `crates/hover-win/**`
- `crates/hover-browser-host/**`
- `browser-extension/**`
- `docs/progress.md`

Resource impact:
- The implemented resident shell starts in `Dormant` with a blocking Win32 message loop and foreground WinEvent hook only.
- Cursor timer is conditionally armed only after an eligible foreground window is detected.
- Decode workers are not spawned by the current shell scaffold, so Dormant decode work remains zero in code.
- Browser network work is absent from the resident core; browser host URL acceptance currently performs no fetch.

Tests run:
- `cargo fmt` passed.
- `cargo test --workspace` passed: 20 Rust tests passed, 0 failed.

Checklist items completed:
- Rust workspace created.
- Stable Rust toolchain verified: `rustc 1.95.0`, `cargo 1.95.0`.
- Release and release-small profiles added.
- Zero idle state accounting implemented.
- Unit tests added for zoom math, fit scales, pan clamp, placement, cache eviction, stale decode replacement, settings parse/fallback, image sniffing, and dirty render presents.
- Browser extension and native messaging host skeleton added.

Blockers:
- [blocked] Full Explorer/Desktop hover target resolution is not yet wired to Shell/UI Automation in the Hover shell.
- [blocked] Overlay does not yet render decoded image pixels; it only creates a hidden no-activate native overlay and paints a neutral test surface.
- [blocked] Direct2D/DirectComposition backend benchmarking has not been performed.
- [blocked] Runtime performance certification requires launching the finished GUI app and measuring with Task Manager/WPR/Process Explorer in an interactive Windows desktop session.

## 2026-04-27 - Tray, installer skeleton, and documentation surface

What changed:
- Added a native Win32 tray icon and event-driven context menu to `hover-win`.
- Added architecture, performance budget, dependency notes, image format matrix, input behavior, decoder fallback order, and testing docs.
- Added WiX and MSIX installer skeletons.
- Added integration-test folder notes for blocked OS-interactive test coverage.
- Reviewed dependency tree with `cargo tree --workspace --depth 2`.

Files touched:
- `crates/hover-win/src/platform.rs`
- `docs/architecture.md`
- `docs/performance-budget.md`
- `docs/dependency-notes.md`
- `docs/image-format-matrix.md`
- `docs/input-behavior.md`
- `docs/decoder-fallback-order.md`
- `docs/testing.md`
- `installer/wix/Product.wxs`
- `installer/msix/AppxManifest.xml`
- `tests/integration/README.md`
- `.gitignore`

Resource impact:
- Tray support uses shell notification messages only; no tray polling loop was added.
- Installer/docs add no resident runtime cost.

Tests run:
- `cargo fmt` passed.
- `cargo test --workspace` passed: 20 Rust tests passed, 0 failed.
- `cargo tree --workspace --depth 2` reviewed.

Checklist items completed:
- Native tray icon/menu skeleton added after the core MVP scaffold.
- Installer skeletons added.
- Required documentation files added.
- Dependency budget notes added.

Blockers:
- [blocked] Startup registry toggle is represented in state but not yet persisted to HKCU.
- [blocked] WiX/MSIX installers have not been built or run.
- [blocked] Native messaging registration is documented by example manifest only; installer registry registration is not implemented yet.

## 2026-04-27 - Final automated validation and status count

What changed:
- Updated `hover_agent_checklist_zero_resource.md` with completed and blocked statuses for implemented/audited items.
- Ran final formatting, workspace tests, release build, dependency count, checklist count, and a short release-process counter snapshot.

Files touched:
- `hover_agent_checklist_zero_resource.md`
- `docs/progress.md`

Resource impact:
- Release GUI quick process snapshot after `WaitForInputIdle(2000)`:
	- CPU seconds: 0
	- Working set: 7,729,152 bytes
	- Private memory: 1,458,176 bytes
	- Handles: 136
	- Threads: 4
- This snapshot is not a full zero idle work certification. Task Manager/WPR/Process Explorer measurement remains blocked until the complete preview app can be run interactively.

Tests run:
- `cargo fmt` passed.
- `cargo test --workspace` passed: 20 Rust tests passed, 0 failed.
- `cargo build --release` passed.
- `cargo tree --workspace --depth 2` reviewed.

Checklist status:
- Completed checklist entries: 202
- Blocked checklist entries: 102
- Still-open checklist entries: 681

Blockers:
- [blocked] Full Explorer/Desktop hover target resolution is not implemented.
- [blocked] Decoded bitmap rendering in the overlay is not implemented.
- [blocked] Native wheel capture/default-on wheel zoom is not wired.
- [blocked] Direct2D/DirectComposition backend is not implemented or benchmarked.
- [blocked] Full runtime idle CPU/GPU/disk/network certification requires the completed GUI app and interactive Windows profiling tools.

## 2026-04-27 - Working local preview pipeline, GIF, video, folder, and ZIP previews

What changed:
- Added an MSAA plus Shell COM target resolver for Explorer hover targets, adapted from the reference repository's layered detection approach without adopting its permanent polling loop.
- Wired stable-hover detection into the foreground-gated Win32 timer path.
- Added cancellable generation-based preview workers for local targets.
- Added decoded bitmap rendering in the no-activate overlay using an interim GDI `StretchDIBits` backend.
- Added animated GIF decoding with canvas compositing, disposal handling, frame timing, memory cap, frame cap, and visible-only animation timer playback.
- Added active-only ffplay video preview for common video extensions, with the process stopped when preview hides or target changes.
- Added bounded folder and ZIP list previews for hovered folders and `.zip` files, without background indexing or recursive scans.
- Added `HOVER_PREVIEW_PATH` as a manual test hook for the same preview pipeline when Explorer interaction is unavailable.

Files touched:
- `Cargo.toml`
- `crates/hover-core/src/state.rs`
- `crates/hover-decode/Cargo.toml`
- `crates/hover-decode/src/lib.rs`
- `crates/hover-win/Cargo.toml`
- `crates/hover-win/src/lib.rs`
- `crates/hover-win/src/platform.rs`
- `crates/hover-win/src/resolver.rs`
- `docs/reference-rust-hover-preview-audit.md`
- `docs/dependency-notes.md`
- `hover_agent_checklist_zero_resource.md`

Resource impact:
- Dormant still has no cursor timer, no decode receiver, no animation timer, no overlay paint, and no ffplay process.
- Folder and ZIP enumeration run only after a stable hover target is confirmed and are capped to preview entries.
- GIF animation uses a timer only while the preview is visible and stops on hide/foreground change.
- Video playback is an explicit user-requested active-preview feature and requires `ffplay` on PATH; no FFmpeg process is resident while idle.

Tests run:
- `cargo fmt` passed.
- `cargo test --workspace` passed: 22 Rust tests passed, 0 failed.

## 2026-04-27 - Video stability and scrollable panels

What changed:
- Added a media-player foreground grace period so ffplay/mpv/VLC startup focus churn no longer forces Hover into Dormant and kills the preview immediately.
- Throttled external video window restyling/topmost repositioning after attachment instead of calling `SetWindowPos` every cursor timer tick.
- If a video backend exits or errors, Hover now shows a stable status panel instead of repeatedly restarting and flickering on the same hover target.
- Added shared overlay scroll state for text and list previews.
- Mouse wheel over text/Markdown/NFO, folder, ZIP, and Sims package warehouse panels scrolls the visible preview content instead of scrolling Explorer behind it.
- Added slim in-preview scrollbars for long text/list panels.
- Raised bounded panel caps to 2,000 text lines and 2,000 list rows per stable hover target; reads remain bounded and active-hover-only.

Resource impact:
- No external media backend is resident while idle.
- No mouse hook is installed while hidden or Dormant; the hook is active only for visible image/text/list previews.
- Folder, ZIP, and package enumeration still happen only after stable hover and remain capped.
- Video window maintenance now runs at attach/steady intervals instead of every 33 ms.

Tests and validation:
- `cargo fmt` passed.
- `cargo test --workspace` passed: 22 Rust tests passed, 0 failed.
- VS Code diagnostics found no errors in `crates/hover-win/src/platform.rs`.
- `cargo build --release` passed after stopping the older `hover.exe` and any orphaned media backend processes.
- Fresh release app launched:
	- Process ID: 28532
	- CPU seconds: 0
	- Working set: 7,983,104 bytes
	- Private memory: 1,572,864 bytes
	- Handles: 139
	- Threads: 4
- Confirmed no `ffplay.exe`, `mpv.exe`, or `vlc.exe` process is resident while Hover is idle.

Checklist status after this pass:
- Completed checklist entries: 321
- Blocked checklist entries: 99
- Still-open checklist entries: 582

Remaining blockers:
- [blocked] Direct2D/DirectComposition final renderer and benchmark are not implemented; GDI remains the interim working backend.
- [blocked] Browser remote image fetch/decode/overlay handoff is not implemented.
- [blocked] Animated WebP playback is not implemented.
- [blocked] Installer execution/native messaging registration tests are not complete.
- [blocked] Full zero idle work certification still requires interactive Windows profiling measurements.

## 2026-04-27 - Active wheel zoom, image quality, and video controls

What changed:
- Replaced unreliable no-activate overlay-only wheel handling with a temporary low-level mouse hook installed only while an image preview is visible.
- The wheel hook filters to `WM_MOUSEWHEEL` only when the cursor is inside the preview bounds, then reuses the tested cursor-centered zoom model.
- Kept previews alive while the cursor is over the overlay or ffplay video surface so moving into the preview no longer causes immediate target-loss dismissal.
- Switched static image decode to an 8192 px bounded high-quality path and made GDI painting use full-client clears plus `HALFTONE` scaling after zoom settles.
- Reworked video playback startup to discover `ffplay.exe` through `HOVER_FFPLAY_PATH`, `PATH`, the app folder, and common FFmpeg install locations instead of only relying on PATH.
- Added ffplay window discovery/repositioning/topmost/no-activate styling based on the spawned process ID.
- Added active-video-only keyboard forwarding: Space pause/play, left/right seek, and up/down native player controls.
- Installed FFmpeg Gyan build through winget so this machine now has working `ffplay.exe` and `ffprobe.exe` aliases.

Resource impact:
- No mouse or keyboard hook is installed while previews are hidden or Hover is Dormant.
- The mouse hook is active only for visible image previews and does not call COM or WIC.
- The keyboard hook is active only for ffplay-backed video previews and is removed when video playback stops or the preview hides.
- `ffplay.exe` is spawned only for active hovered video files and killed when the preview hides or target changes.

Tests and validation:
- `cargo fmt` passed.
- `cargo test --workspace` passed: 22 Rust tests passed, 0 failed.
- VS Code diagnostics found no errors in `crates/hover-win/src/platform.rs`.
- `ffplay -version` passed after installing FFmpeg: `ffplay version 8.1-full_build-www.gyan.dev`.
- `cargo build --release` passed after stopping the older running `hover.exe`.
- Fresh release app launched:
	- Process ID: 51292
	- CPU seconds: 0
	- Working set: 11,616,256 bytes
	- Private memory: 2,088,960 bytes
	- Handles: 192
	- Threads: 3
- Confirmed no `ffplay.exe` process is resident while Hover is idle.

Checklist status after this pass:
- Completed checklist entries: 319
- Blocked checklist entries: 99
- Still-open checklist entries: 584

Remaining blockers:
- [blocked] Direct2D/DirectComposition final renderer and benchmark are not implemented; GDI remains the interim working backend.
- [blocked] Browser remote image fetch/decode/overlay handoff is not implemented.
- [blocked] Animated WebP playback is not implemented.
- [blocked] Installer execution/native messaging registration tests are not complete.
- [blocked] Full zero idle work certification still requires interactive Windows profiling measurements.
- Initial `cargo build --release` was blocked by the previously running `hover.exe`; stopped the old process and reran the build successfully.
- `cargo build --release` passed.
- VS Code diagnostics found no errors.
- Fresh release app launched:
	- Process ID: 46788
	- CPU seconds: 0
	- Working set: 7,917,568 bytes
	- Private memory: 1,560,576 bytes
	- Handles: 139
	- Threads: 4

Checklist status after this pass:
- Completed checklist entries: 311
- Blocked checklist entries: 99
- Still-open checklist entries: 592
- `cargo build --release` passed.
- `cargo tree --workspace --depth 2` reviewed after adding `gif` and `zip`.
- Release GUI dormant smoke snapshot after `WaitForInputIdle(2000)`:
	- CPU seconds: 0
	- Working set: 7,766,016 bytes
	- Private memory: 1,527,808 bytes
	- Handles: 136
	- Threads: 4

Blockers:
- [blocked] Direct2D/DirectComposition final renderer is not implemented; GDI is an interim backend.
- [blocked] Interactive Explorer/Desktop hover behavior and ffplay window behavior still require manual desktop validation.
- [blocked] Wheel zoom capture is still not wired to the overlay.
- [blocked] Animated WebP playback is not implemented.
- [blocked] Full zero idle work certification still requires interactive Task Manager/WPR/Process Explorer measurements.

Checklist status after this pass:
- Completed checklist entries: 230
- Blocked checklist entries: 101
- Still-open checklist entries: 671

## 2026-04-27 - Wheel zoom, cancel controls, startup registration, and release launch prep

What changed:
- Added default-on image wheel zoom through normal `WM_MOUSEWHEEL` messages on the visible no-activate overlay.
- Reused the tested core cursor-centered zoom model for plain, Shift fine, and Ctrl coarse wheel zoom.
- Added 100 ms zoom quality settling through an active-preview-only Win32 timer.
- Added Escape dismissal and left-click-outside dismissal while a preview is active.
- Added native HKCU Run startup registration behind the tray `Run at Startup` toggle.
- Updated checklist and input docs for the completed wheel/cancel/startup work.

Resource impact:
- No global wheel hook or Raw Input capture is installed.
- Wheel handling exists only as normal overlay window messages while the overlay is visible.
- Escape/click cancellation is checked only while a preview path is active and the foreground-gated cursor timer is already running.
- Startup registration performs a registry read at app startup and registry writes only when the user toggles startup from the tray.

Tests run:
- `cargo fmt` passed.
- `cargo test --workspace` passed: 22 Rust tests passed, 0 failed.
- `cargo build --release` passed.
- VS Code diagnostics found no errors.
- Fresh release app launched after stopping any older `hover.exe` process:
	- Process ID: 39368
	- CPU seconds: 0
	- Working set: 7,929,856 bytes
	- Private memory: 1,564,672 bytes
	- Handles: 139
	- Threads: 4

Checklist status after this pass:
- Completed checklist entries: 311
- Blocked checklist entries: 99
- Still-open checklist entries: 592

Remaining blockers:
- [blocked] Direct2D/DirectComposition final renderer and benchmark are not implemented; GDI remains the interim working backend.
- [blocked] Browser remote image fetch/decode/overlay handoff is not implemented.
- [blocked] Animated WebP playback is not implemented.
- [blocked] Installer execution/native messaging registration tests are not complete.
- [blocked] Full zero idle work certification still requires interactive Windows profiling measurements.

## 2026-04-27 - Text previews and icon-rich folder previews

What changed:
- Added bounded previews for README/LICENSE/CHANGELOG-style files and common text extensions including `.md`, `.markdown`, `.txt`, `.nfo`, `.log`, `.ini`, `.json`, `.yaml`, `.toml`, source files, scripts, and config files.
- Added UTF-8, UTF-8 BOM, UTF-16 LE, and UTF-16 BE text decoding with binary-file rejection.
- Split overlay content into image, document, and rich list surfaces.
- Reworked folder hover previews into an XYplorer-style list with title, counts, alternating rows, file sizes, and native small shell icons for real filesystem entries.
- Reworked ZIP previews into the same richer list surface with archive entry icons and bounded row display.
- Enabled the `Win32_Storage_FileSystem` `windows-sys` feature required for `SHGetFileInfoW` shell icons.

Resource impact:
- Text files read at most 384 KiB per stable hover target and never in Dormant.
- Folder and ZIP previews still enumerate only the hovered target and cap visible rows.
- Shell icons are requested only while painting visible folder/list previews; no icon cache or background icon scan was added.

Tests run:
- `cargo fmt` passed.
- `cargo test --workspace` passed: 22 Rust tests passed, 0 failed.

## 2026-04-27 - Latest release: video audio, seeking, and tray icon

What changed:
- Kept active video previews alive while the spawned media player is in startup grace or owns the foreground, fixing the remaining flicker when the cursor is not directly over the player surface.
- Added video settings with audio enabled by default and a configurable seek step, saved in `settings.ini` under `[video]`.
- Extended the native Options window with a video audio toggle and plus/minus seek interval controls.
- Video launch commands now honor the audio toggle for ffplay, mpv, and VLC; ffplay receives the configured `-seek_interval` value.
- Wheel up/down over the active video sends forward/backward seek taps, and left/right arrow keys continue to seek through the active player.
- Replaced the generic tray icon with a generated high-resolution Hover icon held for the app lifetime.

Resource impact:
- Video wheel and keyboard hooks remain active-preview-only and are removed when the preview hides.
- No external media backend is resident while Hover is idle.
- The custom tray icon is generated once at startup and does not add polling or background work.

Tests and validation:
- `cargo fmt` passed.
- `cargo test --workspace` passed: 22 Rust tests passed, 0 failed.
- `cargo build --release` passed after stopping the older running `hover.exe`.
- VS Code diagnostics found no errors.
- Fresh release app launched:
	- Process ID: 5000
	- CPU seconds: 0
	- Working set: 7,974,912 bytes
	- Private memory: 1,556,480 bytes
	- Handles: 139
	- Threads: 4
- Confirmed no `ffplay.exe`, `mpv.exe`, or `vlc.exe` process is resident while Hover is idle.

Checklist status after this pass:
- Completed checklist entries: 340
- Blocked checklist entries: 98
- Still-open checklist entries: 564