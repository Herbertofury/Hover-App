# rust-hover-preview Reference Audit

Reference: https://github.com/cosmokud/rust-hover-preview

Audit date: 2026-04-27

Local clone: `reference/rust-hover-preview`

Reference build: `cargo build` passed on Windows with `rustc 1.95.0`.

Runtime observation: [blocked] The reference executable can be built locally, but this tool session cannot drive a real File Explorer hover/navigation session or inspect the tray UI interactively. Source, README, Cargo manifest, and changelog behavior notes were audited before Hover implementation.

## Files inspected

- [x] `reference/rust-hover-preview/README.md` informed the feature inventory, supported formats, tray behavior, FFmpeg split, and topmost no-focus preview goals.
- [x] `reference/rust-hover-preview/Cargo.toml` informed dependency decisions: use Windows-native APIs first, keep non-core decoders explicit, and keep FFmpeg/ffplay as an active-preview-only external process path after the user's follow-up request for video playback.
- [x] `reference/rust-hover-preview/CHANGELOG.md` informed rapid-hover, animation, stale-folder, mislabeled-extension, and CPU-spike traps.
- [x] `reference/rust-hover-preview/src/main.rs` informed process/thread layout risks: permanent preview/hook threads are simple but not zero idle work.
- [x] `reference/rust-hover-preview/src/explorer_hook.rs` informed Explorer folder discovery, MSAA/UI Automation fallback behavior, and traps around folder navigation state.
- [x] `reference/rust-hover-preview/src/preview_window.rs` informed no-activate topmost preview behavior, generation checks, queued animation frame transfer, and video-process risks.
- [x] `reference/rust-hover-preview/src/tray.rs` informed tray menu command handling and startup toggle ergonomics.
- [x] `reference/rust-hover-preview/src/config.rs` informed config defaults and the risk of writing config files during startup.
- [x] `reference/rust-hover-preview/src/startup.rs` informed HKCU startup registration.

## Useful patterns to keep conceptually

- [x] Explorer hover target resolution: Keep the layered strategy of resolving the hovered UI element, validating it is actually under the cursor, then resolving a filename against the current Explorer folder. Hover must prefer Shell COM and direct point-under-cursor identity before UI Automation fallbacks. Informed by `src/explorer_hook.rs` functions around `AccessibleObjectFromPoint`, `IUIAutomation`, `IShellWindows`, and cursor-rectangle validation.
- [x] Keyboard-focused item preview: Keep the concept of UI Automation focused-item lookup after explicit navigation input, but keep it outside MVP unless it can be gated without idle polling. Informed by `get_focused_explorer_item` and the changelog notes about stale `GetAsyncKeyState` triggers.
- [x] Tray controls: Keep a tiny native tray menu for enable/disable, startup, settings, and diagnostics toggles. Informed by `src/tray.rs`.
- [x] Non-activating topmost preview window: Keep the `WS_EX_NOACTIVATE`, `WS_EX_TOOLWINDOW`, and topmost-on-visible pattern. Hover adds stricter hide-time cleanup and no hidden render work. Informed by `src/preview_window.rs`.
- [x] Config format and storage: Keep a simple user-editable config file with safe defaults, but Hover must not save-on-load or perform startup disk writes unless needed. Informed by `src/config.rs`.
- [x] Animated GIF/WebP handling lessons: Keep frame limits, cancellation, queue-based frame transfer, and redraw throttling concepts. Informed by `CHANGELOG.md` 0.1.6 and `src/preview_window.rs` streaming code.
- [x] Rapid-hover cancellation lessons: Keep generation IDs, cancel flags, single newest-request replacement, and stale-result rejection. Informed by `CHANGELOG.md` and `LoadResult { generation, media }` patterns in `src/preview_window.rs`.

## Patterns to replace for Hover

- [x] Rendering path: Replace GDI `StretchDIBits` as the final renderer with a renderer boundary designed for Direct2D/DirectComposition benchmarking. Current Hover uses GDI `StretchDIBits` as an interim working backend so the product can display pixels while the final backend remains blocked.
- [x] Decoder path: Replace always-bundled Rust image decoding in the resident core with WIC-first decoding and feature-gated fallback decoders.
- [x] Foreground/window activation path: Replace permanent Explorer polling with `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` and a Dormant state that arms cursor timers only in eligible foreground contexts.
- [x] Timer and polling behavior: Replace `std::thread::sleep` polling loops with Win32 message waits, conditionally armed timers, condition variables, and explicit shutdown wakeups.
- [x] Cache behavior: Replace folder media indexing and broad Explorer-folder caching with target-only resolution and bounded metadata/bitmap caches.
- [x] Video preview behavior: Do not copy FFmpeg as a resident/background dependency. After the user explicitly requested video playback, adapt the reference ffplay approach only as an active-preview process that starts for hovered video files and is killed when the preview hides.

## Follow-up adaptation after full-product request

- [x] Adapted the reference MSAA/Shell folder idea into `crates/hover-win/src/resolver.rs`: `AccessibleObjectFromPoint` resolves the item text/value under the cursor, then `IShellWindows` resolves the active Explorer folder so Hover can build a local path without permanent polling.
- [x] Adapted the reference generation/cancellation lesson into `crates/hover-win/src/platform.rs`: each worker result carries a generation, stale results are ignored, and target changes cancel the active token.
- [x] Adapted the reference GDI paint path as a temporary working renderer: decoded BGRA frames are drawn with `StretchDIBits`, and folder/ZIP previews are painted with native text drawing.
- [x] Adapted the reference GIF compositing lesson into `crates/hover-decode/src/lib.rs`: GIF subframes are composited onto a canvas, disposal modes are handled, memory is capped, and animation timers run only while visible.
- [x] Adapted the reference ffplay command concept for video preview: Hover starts `ffplay` only for an active video target and stops the process when the preview hides or the target changes.
- [x] Rejected the reference folder media index/cache again: folder and ZIP previews enumerate only the hovered target, cap visible entries, and perform no background indexing.

## Known traps from the reference changelog

- [x] Animated WebP CPU spikes: Avoid eager long-stream decode, cap frame count and bytes, stop animation timers immediately when hidden, and redraw only when the visible frame changes.
- [x] Duplicate frame copying and memory churn: Use bounded queues and move frames once into the render-owned cache. Avoid duplicate full-frame copies in animation hot paths.
- [x] Redraw conditions during animation: No continuous redraw loop for static previews. Animated previews use frame due-time only while visible.
- [x] Decoder contention during long streams: One lazy worker by default, replacement semantics for new targets, and cooperative cancellation before each decode step.
- [x] Stale folder state after navigation: Treat folder changes as target invalidation and suppress previews until the target is resolved freshly.
- [x] False preview triggering after folder changes: Prefer no preview over a guessed preview; require hover stability and target identity confirmation after navigation.
- [x] Previous image flashing during rapid target switches: Every decode result carries a generation ID and is ignored unless it matches the current Hover target.
- [x] Mislabeled file extensions: Sniff headers before full decode and let WIC/system codec identify the source where possible.

## Hover decisions

- [x] Keep: MSAA/UI Automation as fallback concepts, Shell COM folder discovery, no-activate overlay style, native tray ergonomics, startup registry concept, animation caps, generation IDs, cancellation tokens, and stale-result rejection.
- [x] Replace: GDI-first drawing, broad Rust decoder dependency tree in the resident core, permanent polling loops, folder media indexing, save-on-load config writes, and video-in-core behavior.
- [x] Defer: Keyboard-focused preview, animated WebP playback, rich settings UI, browser support beyond the companion-extension shell, Direct2D/DirectComposition renderer completion, and installer execution tests until the native image path is stable.
- [x] Reject: Electron, Tauri, WebView, Node, Python runtime dependency, .NET runtime dependency, telemetry, background scanning, permanent global hooks, resident FFmpeg work while idle, and unbounded decode/cache queues.

## Performance implications

- [x] Expected idle cost: Dormant message loop only, foreground WinEvent hook registered, zero cursor timer, zero decode worker awake, zero render presents, zero wheel hook, zero Raw Input capture, zero network.
- [x] Expected eligible-foreground cost: One normal Win32 cursor timer at 30 Hz maximum, no decode, no full file reads, no GPU work, no wheel capture.
- [x] Expected hover-confirmation cost: Resolve one target identity, perform bounded metadata/header validation only after stable hover, start a replacement decode job only after the hover delay completes.
- [x] Expected preview-visible cost: Overlay visible, render only when dirty, wheel capture active only while the cursor is inside preview bounds, one decode worker may run one current job.
- [x] Expected animation-visible cost: Timer or due-time wake only for the visible animated frame, frame count/bytes capped, stopped immediately when hidden.

## How Hover improves on the reference architecture

- [x] `Dormant` is an explicit state with zero idle polling, zero idle decoding, zero idle rendering, and zero global hook while Dormant.
- [x] Foreground eligibility is event-driven through `SetWinEventHook`, not discovered by a permanent polling loop.
- [x] Decode jobs use replacement semantics so old hover work cannot pile up.
- [x] The resident core uses WIC first and keeps non-WIC formats feature-gated.
- [x] The browser path is isolated from the core app; zero network by default remains true for local preview.
- [x] Every unavoidable non-zero measurement must be documented in `docs/zero-resource-exceptions.md` with evidence before it can be called an exception.