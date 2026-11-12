# Hover Architecture

Hover is a native Windows 11 Rust app with a zero idle work first architecture.

## Process Model

- Core resident binary: `hover` from `crates/hover-win`.
- Browser native messaging host: `hover-browser-host`, separate from the core app.
- Browser extension: Manifest V3 companion that only detects hover targets and sends messages.

## State Machine

The app uses the checklist states exactly as architectural gates:

- `Dormant`: blocking Win32 message loop, foreground WinEvent hook, optional tray messages. Zero cursor timer, zero decode worker awake, zero render loop, zero wheel hook, zero Raw Input capture, zero disk reads, zero network, zero GPU updates.
- `ForegroundEligible`: normal Win32 cursor timer at 30 Hz maximum. No image decode, no GPU work, no wheel capture.
- `HoverCandidate`: target identity and header validation only after stable hover.
- `DecodePending`: one cancelable replacement decode job.
- `PreviewVisible`: overlay visible, dirty render only, wheel capture only while cursor is inside preview bounds.
- `PreviewLocked`: same as visible plus keyboard/pan controls.

## Implemented

- Blocking Win32 message loop.
- `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` foreground activation.
- Conditional eligible-state cursor timer.
- Hidden no-activate topmost overlay window.
- Native tray icon and menu.
- WIC-first decode module.
- Dirty render state and render plan model.
- Cursor-centered zoom model and pan clamp.
- Replacement decode generation/cancellation model.
- Browser companion extension and native messaging host skeleton.

## Blocked Or Incomplete

- [blocked] Full Shell/UI Automation hover target resolver is not wired into the `hover-win` timer path yet.
- [blocked] Overlay does not yet render decoded image pixels through Direct2D/DirectComposition.
- [blocked] Default-on wheel capture is not yet implemented in the native shell.
- [blocked] Runtime zero idle work certification requires interactive Windows profiling.

## Reference Influence

The reference audit in `docs/reference-rust-hover-preview-audit.md` records which `rust-hover-preview` files informed each decision.