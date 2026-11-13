# Pinned preview implementation and evidence

Development feature. Visual polish, actual Explorer input, accessibility and
mixed-DPI behavior still require supported desktop testing before release.

The native preview has a file-identity label and standard Pin, Reload and Close
controls. Pin explicitly activates a resizable window; ordinary glance previews
remain non-activating. Pin stops cursor polling/target replacement and retains
the current preview across app switches. The tray also exposes these commands
for keyboard access. Close dismisses only the preview; the resident app remains.
Unpin returns to a bounded glance window when the originating shell is eligible.

## Input and continuity

- In a pinned window, Tab reaches the native controls. Escape closes, Ctrl+P
  toggles pin, and F5/Ctrl+R reloads. These commands are scoped to Hover's window.
- With image content focused, arrows pan; plus/minus zoom; 0 fits; 1 selects
  native scale. Text/list content supports arrows, Page Up/Down and Home/End.
  Enter/Space on pinned media content explicitly starts playback.
- Explicit reload cancels obsolete work and keeps the old same-file content
  labelled as loading until replacement is ready. The saved view is keyed to
  the requested file and guarded by the existing generation/path checks.
- Same-size image reload preserves zoom/pan and clamps the animation frame
  index. Text/list scroll is preserved within the new content bounds. A changed
  content kind or image dimensions resets the view with an explanatory caption.
  This is not a full animation-time or 3D camera/swatch restoration system.
- Missing or corrupt files replace old content with a named error. A same-file
  reload keeps only the small last-successful view snapshot through repeated
  failures. A successful retry restores that view when compatible; closing or
  opening another target clears it. Error panels cannot overwrite saved scroll
  or image zoom/pan. A retained view is explained in the retry hint.
- Pinned static content has no cursor/media poll. GIF timing is visible-only;
  minimization suspends it without shrinking the saved image viewport. A running
  external player has a separate bounded-frequency maintenance timer. Pinned
  video occupies the content region while the native controls remain available.
- Video forwarding no longer consumes keys typed in another foreground process;
  pinned controls use ordinary window messages instead of a global key hook.

## Validation boundaries

Local source checks include view-model regressions for custom zoom/pan,
dimension/type changes, frame clamping and shrinking text/list content. These
tests do not prove native control usability or visual correctness.

The real Windows app regression test first reproduced loss of text scroll after
repeated missing-source errors. After the fix, the same CLI-driven suite passed
60 assertions locally, including successful text and image recovery after two
failed reloads, byte preservation and the decoder process checks. See
[retained recovery results](evidence/reload-recovery-local-2026-11-12.json).
This exercises production windows/worker code without physical input injection.

Source revision `71b38a7960b84ca92b700f1f319afa0208b8776b` passed formatting,
strict Clippy, 39 workspace parent tests plus the source-sharing subprocess,
the dedicated native lifecycle test and release builds on Windows with both
Rust 1.95.0 and 1.99.0 in [CI run 43315323528](https://github.com/Herbertofury/Hover-App/actions/runs/43315323528).
Each toolchain actually ran one lifecycle test; its logged duration measures
test execution, not preview latency. Local verification output (workspace paths
redacted) is in [pin verification](evidence/pin-verification-2026-11-12.txt);
hosted evidence is in [CI summary](evidence/pin-ci-2026-11-12.json) and
[lifecycle log](evidence/pin-native-lifecycle-2026-11-12.txt).
The displayed CI log has trailing whitespace stripped; substantive output is
retained, with the original log available from the linked run.

The first CI attempt stopped at a Rust 1.95 Clippy warning before native tests.
The match was corrected without relaxing warnings; the succeeding run above
passed both toolchains. External media behavior still needs actual player
fixtures and visual interaction tests; the lifecycle fixture is plain text.

An explicitly ignored native lifecycle test is invoked only by the Windows
GitHub Actions workflow. It creates Hover's own window on that dedicated runner,
checks actual Win32 styles/control labels, routes Pin and F5 through production
message handling, verifies pinned retention and same-file scroll restoration,
checks unchanged fixture bytes, then checks preview close and resident cleanup.
It also minimizes its own window and checks that stale video geometry cannot
capture wheel input and that reload completion leaves the preview minimized.
The wheel guard uses the test's own window as a player stand-in, not a real
media backend or injected wheel input.
It refuses to run locally without the CI environment guard. It neither injects
physical user input nor operates Explorer; it does not replace release UI tests.

The supported computer-use runtime on the user's desktop still fails before
app input with a kernel-assets/path error. No screenshot, physical-input,
screen-reader or real Explorer success is asserted by this document.
