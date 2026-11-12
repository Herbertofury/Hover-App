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
- Explicit reload cancels obsolete work and keeps the old same-file content
  labelled as loading until replacement is ready. The saved view is keyed to
  the requested file and guarded by the existing generation/path checks.
- Same-size image reload preserves zoom/pan and clamps the animation frame
  index. Text/list scroll is preserved within the new content bounds. A changed
  content kind or image dimensions resets the view with an explanatory caption.
  This is not a full animation-time or 3D camera/swatch restoration system.
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

An explicitly ignored native lifecycle test is invoked only by the Windows
GitHub Actions workflow. It creates Hover's own window on that dedicated runner,
checks actual Win32 styles/control labels, routes Pin and F5 through production
message handling, verifies pinned retention and same-file scroll restoration,
checks unchanged fixture bytes, then checks preview close and resident cleanup.
It refuses to run locally without the CI environment guard. It neither injects
physical user input nor operates Explorer; it does not replace release UI tests.

The supported computer-use runtime on the user's desktop still fails before
app input with a kernel-assets/path error. No screenshot, physical-input,
screen-reader or real Explorer success is asserted by this document.
