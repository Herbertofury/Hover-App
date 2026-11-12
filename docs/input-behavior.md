# Input Behavior

## Hover

- Default hover delay: 250 ms.
- Cursor timer: normal Win32 timer at 30 Hz maximum, armed only in eligible foreground contexts.
- Dormant state has zero polling.
- Target changes cancel pending decode work through generation replacement.

## Wheel Zoom

Required model:

- Plain wheel zooms by default over the visible preview.
- Shift plus wheel uses fine zoom.
- Ctrl plus wheel uses coarse zoom.
- Cursor-centered zoom preserves the image coordinate under the cursor.
- Wheel outside the preview passes through.
- Wheel capture is active only while preview is visible and the cursor is inside the preview rectangle.

Current status:

- Core zoom math is implemented and tested.
- Native shell wheel zoom first handles normal `WM_MOUSEWHEEL` messages on the visible no-activate overlay.
- Because Windows often routes wheel input to Explorer instead of a no-activate overlay, Hover now installs a temporary low-level mouse hook only while an image, text, folder, ZIP, or package preview is visible.
- The hook filters to cursor-inside-preview `WM_MOUSEWHEEL`, does not call COM or WIC, and is removed immediately when the preview hides.
- No global wheel hook or Raw Input capture is installed while hidden or Dormant.
- `Ctrl` plus wheel uses coarse zoom and `Shift` plus wheel uses fine zoom.

## Panel Scrolling

- Text, Markdown, README, NFO, folder, ZIP, and Sims package warehouse previews keep a scroll offset in the overlay model.
- Wheel up/down over the visible panel scrolls the preview content instead of scrolling File Explorer behind it.
- Folder, ZIP, and package list previews are bounded to 2,000 rendered rows per stable hover target.
- Text previews are bounded to 384 KiB and 2,000 lines per stable hover target.
- Long panels draw a slim in-preview scroll indicator.

## Video Playback

- Video preview uses installed external media backends in order: ffplay, mpv, then VLC when available.
- Startup foreground changes from a media player get a short grace period so Hover does not instantly enter Dormant and kill playback.
- While the media player has startup focus or active foreground, Hover keeps the current video preview alive instead of treating transient target loss as a hide signal.
- Player window styling and topmost positioning are throttled after attachment instead of reasserted every cursor timer tick.
- Video audio is enabled by default and can be toggled in the native Options window.
- ffplay is launched with the configured seek step; default is 5 seconds.
- Wheel up/down over the active video sends forward/backward seek taps and is captured only while video preview is active and the cursor is inside the video bounds.
- If a player exits or errors, Hover shows a stable status panel instead of repeatedly restarting and flickering.

## Keyboard

Required controls include Escape, L, F, W, H, 1, 0, plus/minus, and arrow-key pan in locked preview.

Current status:

- Escape hides the active preview through the foreground-gated timer path.
- Unpinned video previews install a temporary keyboard hook only while external video playback is active; forwarding requires that player's foreground process.
- Space is forwarded to the video window for pause/play.
- Left/right arrows are forwarded for backward/forward seek, using the configured ffplay seek step when ffplay is the active backend; up/down arrows are forwarded for native player controls.
- Pinned previews use window-scoped messages: Escape closes, F5/Ctrl+R reloads,
  Ctrl+P toggles pin and Tab reaches native controls. With content focused, 0
  fits an image, 1 uses native scale, plus/minus zooms and arrows pan. Text/list
  panels support arrows, Page Up/Down and Home/End. Enter/Space explicitly
  starts prepared media. L/F/W/H fit shortcuts remain unimplemented.
- These paths require physical keyboard and accessibility verification; native
  message tests do not establish those acceptance gates. See [pin evidence](pinned-preview.md).

## Mouse Cancel

- Left click outside the active preview hides the preview.
- Target changes and foreground changes cancel pending decode work and hide active previews.
