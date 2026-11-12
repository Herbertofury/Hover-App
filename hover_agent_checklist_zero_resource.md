# Hover Agent Checklist

A detailed, agent-ready checklist for building an ultra-lightweight Windows 11 hover image preview utility in Rust.

Project intent: create a native Windows 11 app that previews images on hover, feels like Hover Zoom+ for the whole OS, borrows the best image preview behavior from XYplorer, and treats **zero resource cost** as the product requirement, not a nice-to-have optimization.

Zero-cost doctrine for the agent: do **not** design for "low usage" or "close enough." Design for **zero observable work** whenever the user is not actively previewing an image. Any CPU tick, timer, disk read, network call, GPU present, decode job, polling loop, cache churn, allocation churn, or background task outside an explicit active preview state is a defect until proven unavoidable. If Windows, hardware, or measurement noise prevents a literal zero reading, the agent must document the measured baseline, explain the unavoidable source, and continue reducing it. The requirement language throughout this project is **zero first**, with measured exceptions only after proof.

---

## 0A. Zero-Language Policy for the Coding Agent

The agent must use strong zero-cost language in planning, code review, test names, milestone reports, and README text.

- [x] Say **zero idle work**, not "basically free."
- [x] Say **zero hidden decode**, not "light decode."
- [x] Say **zero hidden render**, not "low render overhead."
- [x] Say **zero polling in Dormant**, not "infrequent polling."
- [x] Say **zero background scanning**, not "efficient scanning."
- [x] Say **zero network by default**, not "minimal network."
- [x] Say **zero global hook while Dormant**, not "cheap hook."
- [x] If a metric is non-zero, label it a defect, exception, or active-state cost. Do not label it success.
- [x] Use `docs/zero-resource-exceptions.md` for every unavoidable non-zero measurement.
- [x] CI/performance reports must include a `ZERO_IDLE_STATUS` field with values: `PASS`, `DEFECT`, or `EXCEPTION_DOCUMENTED`.

## 0. Non-Negotiable Requirements

### 0.1 Product requirements

- [blocked] Build a Windows 11 native hover image preview utility. Blocked: local image/GIF/text/folder/ZIP/video/audio/Sims package preview path, active-only wheel zoom and panel scrolling, external-player video controls with audio toggle, seek interval, wheel/arrow seeking, startup registration, generated tray icon, and cancellation controls are wired, but browser handoff, Direct2D/DirectComposition renderer, installer execution, animated WebP, and runtime certification remain incomplete.
- [x] Use Rust as the primary implementation language.
- [x] Keep the tech stack aggressively lightweight.
- [x] Optimize for **zero idle work**. Treat every non-zero idle cost as a bug until measured and justified.
- [x] Preview images when hovering local image files.
- [blocked] Preview images when hovering supported browser images through an optional tiny browser extension. Blocked: extension and host skeleton exist, but remote fetch/decode/overlay handoff is not implemented.
- [blocked] Make the preview appear fast enough to feel instant. Blocked: runtime preview path is implemented but not interactively benchmarked.
- [x] Make the preview disappear instantly when the hover target changes or the user cancels.
- [x] Support mouse-wheel zoom on the preview by default.
- [x] Make wheel zoom cursor-centered by default, meaning the pixel under the cursor remains visually anchored during zoom.
- [blocked] Support a smooth XYplorer-like image preview experience. Blocked: folder/ZIP/package list previews with icons and scrolling, text previews with scrolling, active-only wheel zoom, higher-quality GDI scaling, stabilized video controls with audio/seek options, and a custom tray icon are wired, but final renderer polish and interactive certification remain incomplete.
- [ ] Support all common image formats, with graceful fallback for uncommon formats.
- [x] Ship with strict cache, memory, decode, and network limits.
- [x] Never do background folder scanning.
- [x] Never do image indexing.
- [x] Never add telemetry.
- [x] Never require Electron, Chromium, WebView, Node, Python, Java, .NET, or a browser runtime for the core app.

### 0.2 Forbidden architecture choices

- [x] Do not use Electron.
- [x] Do not use Tauri.
- [x] Do not use WebView2 for the core app or preview overlay.
- [x] Do not use a web UI for the overlay.
- [x] Do not use React, Vue, Svelte, or HTML for the native preview.
- [x] Do not run a global low-level mouse hook while idle.
- [x] Do not scan folders looking for images.
- [x] Do not generate thumbnails in advance.
- [x] Do not decode images before hover confirmation.
- [x] Do not block the UI thread.
- [x] Do not let decode jobs pile up after the cursor moves away.
- [x] Do not fetch remote images unless browser preview mode is enabled.
- [x] Do not store browser browsing history.
- [x] Do not store remote image URLs beyond the configured memory cache.
- [x] Do not add dependencies without documenting size, CPU, memory, and startup impact.


### 0.3 Absolute zero-resource rules

These rules exist to force the app toward actual zero observable cost when the user is not actively previewing an image. The agent must not soften this to "low usage", "tiny cost", or "close enough" in design docs, code comments, milestone definitions, or acceptance criteria. Use the word **zero** as the target. Use measurement notes only to explain unavoidable operating-system noise after benchmarking.

- [ ] The product requirement is **zero idle work**, not "low idle work."
- [ ] Any non-zero idle CPU, disk, network, GPU, timer, hook, polling, allocation, or background activity must be opened as a performance defect.
- [ ] A measured exception is allowed only when the agent proves the cost comes from Windows baseline behavior, Task Manager sampling noise, hardware driver behavior, or an explicitly enabled user feature.
- [ ] Measured exceptions must be documented in `docs/zero-resource-exceptions.md` with repro steps, ETW/Profiler evidence, and the mitigation attempted.
- [ ] Marketing and README language must say **zero idle work target**, **zero background scanning**, **zero idle decoding**, and **zero idle rendering**.
- [ ] Default app state must be `Dormant`.
- [ ] `Dormant` means no timers, no decode workers awake, no render loop, no GPU surface updates, no disk reads, no network, no hot-path logging, and no global low-level mouse hook.
- [ ] Every timer must be conditionally armed and conditionally disarmed.
- [ ] Every hook must be conditionally installed and conditionally removed.
- [ ] Every worker must sleep on a wait primitive, not spin.
- [ ] Every loop must have a blocking wait, explicit wake condition, and explicit shutdown condition.
- [ ] Every cache must have a hard byte cap.
- [ ] Every queue must have a hard length cap.
- [ ] Every decode request must be cancelable.
- [ ] Every render must be dirty-flag driven.
- [ ] No continuous render loop is allowed unless an animated image is visibly playing.
- [ ] No heap allocation is allowed in the mouse input hot path after initialization.
- [ ] No string formatting is allowed in the mouse input hot path in release builds.
- [ ] No debug logging is allowed in the mouse input hot path in release builds.
- [ ] No async runtime is allowed in the core resident app.
- [ ] No Tokio, async-std, reqwest, hyper, curl, OpenSSL, or bundled browser runtime in the core resident app.
- [ ] Browser remote fetching, if added later, must live in the optional browser host and must not affect local-preview idle cost.
- [ ] The agent must reject any library, framework, or feature that increases idle work unless the user explicitly enables it and the performance budget is updated.
- [ ] The final app must be benchmarked in the six resource states listed in section 3.4.

---

## 1. Product Name and Working Terms

### 1.1 Temporary project name

- [x] Use `Hover` as the working name unless replaced later.

### 1.2 Terms the agent must use consistently

- [ ] `Hover target`: the item under the cursor that may produce a preview.
- [ ] `Preview source`: the resolved local path, URL, stream, shell item, or embedded thumbnail used for preview.
- [ ] `Overlay`: the native topmost window that renders the image preview.
- [ ] `Transient preview`: a preview that follows hover and closes automatically.
- [ ] `Locked preview`: a preview that remains open for zoom, pan, copy, or inspection.
- [ ] `Fit scale`: the scale that fits the image within the preview bounds.
- [ ] `Native scale`: 100 percent image scale, one image pixel to one logical pixel after DPI handling.
- [ ] `Zoom anchor`: the cursor-relative point that remains visually stable while zooming.
- [ ] `Eligible foreground context`: File Explorer, Desktop, Open/Save dialog, or a supported browser extension signal.

---

## 2. Core User Experience

### 2.1 Default hover behavior

- [ ] When the cursor rests over an image file, wait for the configured hover delay.
- [ ] Default hover delay: 250 ms.
- [ ] Do not decode until the cursor is stable over the same target for the full delay.
- [ ] Show the preview near the cursor without covering the target when possible.
- [ ] Keep the preview fully inside the current monitor work area.
- [ ] Use the monitor containing the cursor, not the monitor containing the foreground window, for placement.
- [ ] Preserve image aspect ratio.
- [ ] Start in `Fit All` mode by default.
- [x] Do not steal focus.
- [x] Do not appear in Alt-Tab.
- [x] Do not appear in the taskbar.
- [x] Hide immediately when the cursor leaves the target and the preview is not locked.
- [x] Hide immediately on Escape.
- [x] Hide immediately on left click outside a locked preview.
- [x] Hide immediately when the foreground app changes.
- [x] Hide immediately when a fullscreen app becomes active.

### 2.2 Default wheel zoom behavior, required

This is mandatory. The agent must implement this early, not as a late optional feature.

- [x] Mouse wheel over the visible preview zooms the image in and out by default.
- [x] Wheel up zooms in.
- [x] Wheel down zooms out.
- [x] Zoom must be cursor-centered by default.
- [x] The zoom anchor must be the image coordinate under the cursor.
- [x] During zoom, keep the anchored image coordinate under the cursor as closely as possible.
- [x] Do not use the mouse wheel to browse next or previous images by default.
- [x] Do not use the mouse wheel to scroll the underlying File Explorer window while the cursor is inside the preview bounds and the preview is visible.
- [x] If the cursor is outside the preview bounds, pass wheel input through normally.
- [x] If the preview is hidden, do not intercept wheel input.
- [x] If the preview is visible but not under the cursor, do not intercept wheel input.
- [x] Wheel zoom must work in transient preview mode.
- [ ] Wheel zoom must also work in locked preview mode.
- [x] Wheel zoom must not require Ctrl, Shift, or Alt.
- [x] Ctrl plus wheel may be used as an alternate zoom shortcut, but plain wheel remains the default.
- [x] Shift plus wheel performs fine zoom by default.
- [x] Alt plus wheel may be assigned to browse previous or next image later, but this must be disabled in MVP unless explicitly implemented.

### 2.3 Required zoom model

- [x] Initial scale mode: `Fit All`.
- [x] Minimum zoom: fit scale or 5 percent, whichever is smaller, but never allow invisible or negative scale.
- [x] Maximum zoom: 800 percent by default.
- [ ] Hard maximum zoom: 3200 percent behind an advanced setting.
- [x] Wheel zoom step: 12.5 percent relative scale per wheel notch.
- [x] Fine zoom step with Shift plus wheel: 2.5 percent relative scale per wheel notch.
- [x] Coarse zoom step with Ctrl plus wheel: 25 percent relative scale per wheel notch.
- [x] Include a snap zone around 100 percent.
- [x] Snap to 100 percent when zoom crosses 100 percent within a small epsilon.
- [x] Include a snap zone around fit scale.
- [x] Snap to fit scale when zoom crosses fit scale within a small epsilon.
- [x] Never crop by changing aspect ratio.
- [ ] When zoomed larger than the preview viewport, allow panning.
- [x] When zoomed smaller than the viewport, center the image.
- [x] When zoomed near fit, avoid subpixel jitter.
- [x] Use high-quality scaling when preview is static.
- [x] Use fast scaling while actively wheel-zooming.
- [x] Switch to high-quality scaling after 100 ms of zoom inactivity.

### 2.4 Required pan model

- [ ] Panning is enabled when the zoomed image is larger than the preview viewport.
- [ ] Middle-button drag pans the image in locked preview mode.
- [ ] Space plus left-button drag pans the image in locked preview mode.
- [ ] Right-button drag may pan only if it does not conflict with the context menu behavior.
- [x] In transient preview mode, plain wheel zooms only.
- [ ] In transient preview mode, panning requires holding Space, middle mouse, or switching to locked mode.
- [x] Clamp pan so empty space is not shown beyond the image edges unless the image is smaller than the viewport.
- [x] Preserve pan offset when zooming in and out.
- [x] Reset pan when the source image changes.
- [ ] Reset pan when the user presses `F` for Fit All.

### 2.5 Required keyboard controls

- [x] Escape hides the preview.
- [ ] Ctrl toggles lock while held, if this is technically reliable.
- [ ] Pressing `L` toggles locked preview mode.
- [ ] Pressing `F` switches to Fit All.
- [ ] Pressing `W` switches to Fit Width.
- [ ] Pressing `H` switches to Fit Height.
- [ ] Pressing `1` switches to 100 percent native scale.
- [ ] Pressing `0` resets to Fit All.
- [ ] Pressing `+` zooms in.
- [ ] Pressing `-` zooms out.
- [ ] Pressing arrow keys pans when locked and zoomed in.
- [ ] Pressing PageUp/PageDown may browse previous or next sibling image later, but not in MVP unless implemented correctly.

### 2.6 Preview placement rules

- [ ] Prefer placing preview to the right and below the cursor.
- [ ] If it would overflow the monitor, flip to the left or above.
- [ ] Apply a minimum cursor gap of 16 logical pixels.
- [ ] Never place the preview so it covers more than 80 percent of the current monitor work area by default.
- [ ] Default maximum preview size: 80 percent of monitor width and 80 percent of monitor height.
- [ ] Default minimum preview size: 160 x 160 logical pixels when possible.
- [ ] Small images may be shown at native scale by default if they fit comfortably.
- [ ] Large images start in Fit All.
- [ ] The preview should not jump during small cursor movements over the same target.
- [ ] Reposition only when needed to avoid edge overflow.

---

## 3. Resource Usage Targets

### 3.1 Idle target: zero means zero

- [ ] CPU: **0.0 percent** in Task Manager while Dormant. Do not accept "low" as passing.
- [ ] GPU: **0.0 percent** when preview is hidden. No presents, no animation clock, no composition churn caused by this app.
- [ ] Disk: **0 reads after startup settles** when preview is hidden.
- [ ] Network: **0 connections and 0 bytes** when browser remote preview is disabled.
- [ ] Timers: **0 active timers** while Dormant.
- [ ] Hooks: **0 global mouse hooks** while Dormant.
- [ ] Decode work: **0 decode workers awake** while Dormant.
- [ ] Render work: **0 render passes** while Dormant.
- [ ] Background scanning: **0 folder scans ever**.
- [ ] Memory: target under 10 MB private working set for core idle app, but do not trade zero CPU/disk/GPU for lower memory.
- [ ] Handle count: stable, no leaks, no repeating open/close churn.
- [ ] Threads: minimum viable count. Prefer no worker thread until the first decode request unless measurement proves a sleeping worker is cheaper.
- [ ] Startup impact: instant-feeling startup, no indexing, no scan, no prewarming unless benchmarked as strictly cheaper than lazy initialization.

### 3.2 Active but hidden target

When File Explorer or Desktop is foreground but no preview is visible:

- [ ] CPU average under 0.1 percent.
- [ ] No image decoding.
- [ ] No image loading.
- [ ] No folder enumeration.
- [ ] Only lightweight target detection is allowed.
- [ ] Any timer must stop when the foreground context becomes ineligible.

### 3.3 Preview visible target

- [ ] First visible preview for small local JPG/PNG: target under 50 ms after hover delay.
- [ ] Normal local image preview: target under 150 ms after hover delay.
- [ ] Huge local image preview: target under 300 ms after hover delay when downscale-on-decode is available.
- [ ] UI thread must stay responsive.
- [ ] Decode cancellation must trigger immediately on target change.
- [ ] Decode queue must not grow without bound.
- [ ] GPU usage allowed only while preview is visible or animating.


### 3.4 Required no-work state machine

The implementation must use this state machine. Do not let features bypass it.

#### State A: `Dormant`

Allowed activity:

- [ ] Main Win32 message loop only.
- [ ] Foreground-window WinEvent callback may be registered.
- [x] Tray icon messages may be handled if tray icon is enabled.
- [ ] No cursor timer.
- [ ] No decode worker awake.
- [ ] No render loop.
- [ ] No low-level mouse hook.
- [ ] No Raw Input wheel capture.
- [ ] No filesystem reads.
- [ ] No network reads.
- [ ] No GPU updates.

Enter `Dormant` when:

- [ ] App starts and foreground context is ineligible.
- [ ] Foreground app becomes ineligible.
- [ ] Fullscreen app is detected.
- [ ] User disables Hover.
- [ ] Session locks.
- [ ] System enters sleep or connected standby.
- [ ] Preview hides and no eligible context remains.

#### State B: `ForegroundEligible`

Allowed activity:

- [ ] Low-frequency cursor stability timer, 30 Hz maximum.
- [ ] Lightweight foreground window validation.
- [ ] Lightweight point-under-cursor target probing.
- [ ] No image decode.
- [ ] No image file read except tiny metadata or header read after stable hover is likely.
- [ ] No GPU work.
- [ ] No low-level mouse hook.
- [ ] No wheel capture.

Enter `ForegroundEligible` when:

- [ ] Foreground window is File Explorer, Desktop, supported Open/Save dialog, or another explicitly allowed file manager.
- [ ] App is enabled.
- [ ] No fullscreen app is active.
- [ ] Session is interactive.

Exit `ForegroundEligible` when:

- [ ] Foreground window changes to ineligible.
- [ ] User disables app.
- [ ] Fullscreen app appears.
- [ ] Session locks.

#### State C: `HoverCandidate`

Allowed activity:

- [ ] Continue cursor stability timer.
- [ ] Resolve target identity.
- [ ] Read only the minimum data needed to confirm image-likeness.
- [ ] Start hover delay countdown.
- [ ] No full decode yet.
- [ ] No GPU work yet.
- [ ] No wheel capture yet.

Enter `HoverCandidate` when:

- [ ] Cursor is stable over the same likely image target.
- [ ] Target path or shell item can be resolved confidently.

Exit `HoverCandidate` when:

- [ ] Cursor moves beyond threshold.
- [ ] Target changes.
- [ ] Hover delay expires and target is valid.
- [ ] Foreground context changes.

#### State D: `DecodePending`

Allowed activity:

- [ ] Wake one decode worker.
- [ ] Use one cancelable decode job.
- [ ] Use WIC downscale-on-decode or Shell cached thumbnail before full decode.
- [ ] Keep UI thread decode-free.
- [ ] Keep render hidden until a valid bitmap or placeholder is ready.
- [ ] No continuous render loop.
- [ ] No wheel capture unless an older preview is already visible.

Enter `DecodePending` when:

- [ ] Hover delay has completed.
- [ ] Target is still valid.
- [ ] Source passes file size and format checks.

Exit `DecodePending` when:

- [ ] Decode completes.
- [ ] Decode is canceled.
- [ ] Target changes.
- [ ] Foreground context changes.
- [ ] Limits are exceeded.

#### State E: `PreviewVisible`

Allowed activity:

- [x] Overlay visible.
- [ ] GPU surface updates only when source, zoom, pan, size, or frame changes.
- [x] Wheel capture active only while cursor is inside preview bounds.
- [x] Optional low-level mouse hook only if Raw Input or other lighter wheel capture fails.
- [x] Render only on dirty state.
- [ ] Decode worker may sleep or run one higher-resolution replacement decode if zoom needs it.

Enter `PreviewVisible` when:

- [x] Valid preview bitmap is ready.
- [x] Foreground context is still eligible.
- [x] Target is still current.

Exit `PreviewVisible` when:

- [x] Cursor leaves target and preview is not locked.
- [x] Escape is pressed.
- [x] Foreground app changes.
- [x] Fullscreen app appears.
- [x] User disables app.
- [x] Target disappears.

#### State F: `PreviewLocked`

Allowed activity:

- [ ] Same as `PreviewVisible`.
- [ ] Keyboard controls active.
- [ ] Panning active.
- [ ] Higher-resolution decode allowed only when zoom demands it.
- [ ] No folder browsing unless user explicitly invokes it.

Exit `PreviewLocked` when:

- [ ] Escape is pressed.
- [ ] User unlocks.
- [ ] Foreground app changes.
- [ ] Fullscreen app appears.
- [ ] Source becomes unavailable.

### 3.5 Best 2026 Windows event architecture for invisible idle cost

Use Windows events to enter and leave work states. Do not discover state by constantly polling.

- [x] Use `SetWinEventHook` for `EVENT_SYSTEM_FOREGROUND` to learn when the foreground window changes.
- [x] Register the foreground event hook out-of-context.
- [x] Use `WINEVENT_SKIPOWNPROCESS` where applicable.
- [x] The WinEvent callback must only enqueue a tiny state update.
- [x] The WinEvent callback must not decode, allocate large memory, enumerate folders, or log heavily.
- [x] Use the main message loop as the core wait point.
- [x] Use normal Win32 timers only while in `ForegroundEligible`, `HoverCandidate`, `PreviewVisible`, or `PreviewLocked`.
- [x] Do not use high-resolution multimedia timers.
- [x] Do not use a permanent global mouse hook to detect foreground or cursor changes.
- [x] Use `WM_DISPLAYCHANGE`, `WM_DPICHANGED`, and monitor APIs to refresh monitor geometry only when needed.
- [x] Use `WM_POWERBROADCAST` or equivalent power/session notifications to shut down active work before sleep and resume cleanly.
- [ ] Use session lock/unlock notifications to force `Dormant` on lock.
- [x] Use fullscreen detection before showing overlays.
- [x] If there is uncertainty about foreground eligibility, prefer `Dormant`.

### 3.6 Best wheel-input architecture for default-on zoom with zero hidden cost

Wheel zoom is required, but the capture mechanism must not exist while hidden.

Preferred order:

- [x] First try normal window messages if overlay can receive wheel input without stealing focus or blocking clicks.
- [ ] Then try Raw Input while preview is visible.
- [x] Then try a temporary low-level mouse hook only while preview is visible.
- [x] Remove wheel capture immediately when preview hides.
- [x] Never install wheel capture while in `Dormant`, `ForegroundEligible`, `HoverCandidate`, or hidden `DecodePending`.
- [blocked] Intercept only `WM_MOUSEWHEEL` and `WM_MOUSEHWHEEL` equivalents. Blocked: `WM_MOUSEWHEEL` is implemented; horizontal wheel is not wired yet.
- [x] Intercept only when cursor is inside preview rectangle.
- [x] Pass through all other wheel messages.
- [x] Pass through wheel when the preview is hidden.
- [x] Pass through wheel when cursor is outside preview bounds.
- [x] Pass through wheel when app is disabled.
- [x] Pass through wheel in fullscreen apps.
- [ ] The hook path must use preallocated state and atomic reads where practical.
- [x] The hook path must not call COM.
- [x] The hook path must not call WIC.
- [ ] The hook path must not touch disk.
- [ ] The hook path must not allocate strings.
- [ ] The hook path must return immediately.

### 3.7 Best rendering architecture for invisible idle cost

The renderer must be completely dormant while hidden.

Primary recommendation:

- [ ] Use a hidden borderless Win32 overlay window.
- [ ] Use DirectComposition plus Direct2D where it benchmarks lower for zoom/pan and composition.
- [ ] Use Direct2D directly if it benchmarks simpler and equally idle.
- [ ] Do not use GDI+.
- [ ] Do not use WebView.
- [ ] Do not use an immediate-mode GUI for the preview.
- [ ] Do not animate open or close by default.
- [ ] Do not run a frame pump.
- [ ] Maintain a dirty flag for source, zoom, pan, viewport, DPI, and animation frame.
- [ ] Present only when dirty.
- [ ] Upload bitmap once per decoded bitmap.
- [ ] On wheel zoom, transform the existing GPU bitmap first.
- [ ] Request a higher-resolution decode only after zoom settles or quality threshold is crossed.
- [ ] Release optional GPU resources when hidden under memory pressure.
- [ ] Keep device creation lazy unless startup measurements prove it is cheaper to initialize once.

### 3.8 Best decode architecture for invisible resource cost

Use the smallest useful image data, not the full image.

- [ ] Try cached Shell thumbnail only if it is already in cache or available cheaply.
- [ ] Use `IShellItemImageFactory::GetImage` with cache-only behavior where possible on UI-sensitive paths.
- [ ] Never force thumbnail extraction on the UI thread.
- [ ] Never call thumbnail extraction flags that force regeneration during hover unless the work is on a cancelable background thread and within budget.
- [ ] Use WIC first for local decode.
- [ ] Use `IWICBitmapSourceTransform` or decoder-native scaling where available.
- [ ] Convert pixel format during decode or transform when possible.
- [ ] Decode to the current preview need, not original image size.
- [ ] For Fit All, decode near the display size.
- [ ] For wheel zoom, initially scale the already-decoded bitmap on GPU.
- [ ] If zoom crosses quality threshold, schedule a higher-resolution decode.
- [ ] Cancel higher-resolution decode immediately if zoom changes back, target changes, or preview hides.
- [ ] Prefer embedded thumbnails for RAW and huge formats.
- [ ] Use fallback Rust decoders only when WIC and Shell cannot produce a usable image.
- [ ] Feature-gate every non-WIC decoder.
- [ ] Keep animation decode lazy and capped.

### 3.9 Build, binary, and runtime hardening for zero idle cost

- [x] Use `#![windows_subsystem = "windows"]` for the release GUI binary.
- [x] Use the default Windows system allocator unless benchmarks prove another allocator improves total memory and CPU.
- [x] Avoid custom allocators in MVP.
- [x] Avoid async runtimes in the core app.
- [x] Avoid background thread pools in the core app.
- [blocked] Use one worker thread by default. Blocked: decode worker is not wired into the native shell yet.
- [x] Allow two workers only when measuring proves it improves latency without hurting idle.
- [x] Do not create worker threads until first decode request unless startup benchmarks prove one sleeping worker is cheaper overall.
- [x] Prefer static dispatch and small enums for hot paths.
- [x] Avoid trait objects in mouse input hot paths unless measured.
- [x] Avoid heap allocation in zoom math.
- [blocked] Avoid `PathBuf` cloning in hot target-probe loops. Blocked: native target-probe loop is not implemented yet.
- [x] Keep target identity as small stable IDs plus borrowed or interned path data where practical.
- [x] Use bounded channels.
- [x] Use `try_send` or replacement semantics for hover targets so old work is dropped instead of queued.
- [x] Use release profile with LTO, `panic = "abort"`, stripped symbols, and one codegen unit for final builds.
- [blocked] Benchmark `opt-level = 3` against size-focused builds before choosing the shipped profile. Blocked: final runtime benchmark not run.
- [x] Keep diagnostics behind a compile-time or explicit runtime flag.
- [x] Make diagnostics off by default.
- [x] Run `cargo tree` before every dependency addition and record why the dependency exists.

Suggested release profile:

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
strip = "symbols"
debug = false
incremental = false
```

For a size-optimized experimental build, benchmark this separately instead of assuming it is faster:

```toml
[profile.release-small]
inherits = "release"
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
debug = false
incremental = false
```

### 3.10 Resource budget gates

The agent must not mark a milestone complete unless these gates pass. These are **zero gates**, not aspirational guidelines. A milestone with unexplained non-zero idle work is incomplete.

- [ ] Dormant state: 0 active timers.
- [ ] Dormant state: 0 decode jobs.
- [ ] Dormant state: 0 render presents.
- [ ] Dormant state: 0 wheel hooks.
- [ ] Dormant state: 0 Raw Input wheel capture.
- [ ] Dormant state: 0 disk reads after startup settles.
- [ ] Dormant state: 0 network connections.
- [ ] Dormant state: worker thread count stable.
- [ ] Hidden eligible state: 30 Hz maximum cursor checks.
- [ ] Hidden eligible state: no image file full reads.
- [ ] Hidden eligible state: no decoder initialization unless a stable hover starts.
- [ ] Preview visible state: render only when dirty.
- [ ] Preview visible state: no re-upload of same bitmap.
- [x] Preview visible state: wheel input capture removed immediately after hide.
- [ ] Stress test: stale decode count may increase, but stale result display count must be 0.
- [ ] Stress test: queue length must never grow unbounded.
- [ ] Stress test: memory must return to cache cap.
- [ ] Any failure to hit zero in Dormant must create a named issue with owner, repro, measurements, and next mitigation.
- [ ] The agent must not close a zero-resource issue by saying "good enough."
- [ ] The agent must not rename a zero-resource failure as "close-enough success."


---

## 4. Recommended Lightweight 2026 Rust Stack

### 4.1 Rust workspace

- [x] Use a Rust workspace.
- [x] Use stable Rust.
- [x] Use the newest stable Rust compiler available to the agent at implementation time.
- [x] Use `windows-sys` for hot-path Win32 calls where zero abstraction overhead and compile-time cost matter most.
- [x] Use `windows` for COM-heavy or WinRT-heavy code only when it materially reduces risk or boilerplate.
- [blocked] Keep Windows API bindings local to `hover-win`. Blocked: WIC COM bindings currently live in `hover-decode` to isolate decoding and need final architecture review.
- [x] Keep unsafe Windows calls wrapped behind small audited functions.
- [x] Use `parking_lot` only if benchmarks or code simplicity justify it.
- [x] Use `crossbeam-channel` or a custom bounded channel for small internal event channels.
- [x] Do not use unbounded channels for hover targets or decode jobs.
- [x] Use `smallvec` for small hot-path collections.
- [x] Use `lru` or a custom small LRU for decoded bitmap cache.
- [x] Use `thiserror` for errors outside hot paths.
- [x] Avoid formatted errors in hot paths.
- [x] Use `tracing` only behind a debug or diagnostics feature.
- [x] Keep release logging disabled by default.
- [x] Avoid async runtimes in the core resident app.
- [x] Avoid large dependency trees.

### 4.2 Cargo feature strategy

- [blocked] Core build should include only Windows integration, WIC decoding, and local preview. Blocked: local preview overlay rendering is incomplete.
- [x] Add non-WIC decoders behind feature flags.
- [x] Add browser native messaging host behind a feature flag or separate binary.
- [x] Add SVG support behind a feature flag.
- [x] Add JPEG XL support behind a feature flag.
- [x] Add AVIF support behind a feature flag.
- [x] Add RAW support behind a feature flag or rely on WIC/system codecs.

### 4.3 Suggested workspace layout

```txt
hover/
  Cargo.toml
  crates/
    hover-core/
    hover-win/
    hover-decode/
    hover-render/
    hover-browser-host/
    hover-settings/
  browser-extension/
    manifest.json
    background.js
    content.js
  installer/
    wix/
    msix/
  tests/
    fixtures/
    integration/
  benches/
  docs/
    architecture.md
    performance-budget.md
    zero-resource-exceptions.md
    image-format-matrix.md
    input-behavior.md
    decoder-fallback-order.md
```

### 4.4 Initial dependencies

```toml
[workspace.dependencies]
windows = "0.62"
windows-sys = "0.61"
parking_lot = "0.12"
crossbeam-channel = "0.5"
smallvec = "1"
lru = "0.13"
thiserror = "2"
tracing = "0.1"
```


### 4.5 2026 best-tech decision matrix

The agent must use this matrix before choosing a method.

| Area | First choice | Second choice | Avoid by default |
| --- | --- | --- | --- |
| Windows API binding | `windows-sys` on hot paths | `windows` for COM-heavy code | wrapper crates that hide cost |
| Foreground changes | `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` | Shell/window messages where reliable | polling foreground window forever |
| Cursor stability | timer only while eligible | adaptive lower rate after no movement | global mouse hook while idle |
| Wheel zoom | normal messages or Raw Input while preview visible | temporary low-level hook while visible | permanent low-level hook |
| Overlay | Win32 focusless topmost window | DirectComposition visual tree | WebView/Electron/Tauri |
| Rendering | Direct2D plus DirectComposition if fastest | Direct2D HWND path if simpler | GDI+, software loops, web canvas |
| Decode | WIC with decoder-native downscale | Shell cached thumbnail or embedded thumbnail | full decode first |
| Fallback formats | optional feature-gated Rust decoders | installed WIC codecs | always-bundled codec pack |
| Remote browser images | optional native messaging host | browser extension prefetch handoff | network stack in resident core |
| Settings UI | tray menu/native dialog | separate optional tiny UI process | resident web UI |
| Diagnostics | explicit debug flag | ETW/WPR manual profiling | always-on logging |
| Caching | hard-capped LRU | memory-pressure-aware eviction | unbounded maps |

### 4.6 Source-grounded API notes for the agent

- [ ] `windows` and `windows-sys` are the official Rust for Windows crates. Use them instead of large abstraction frameworks.
- [ ] DirectComposition is appropriate for high-performance bitmap composition and can operate through DWM independently of the app UI thread.
- [ ] WIC is the first decoder choice because it is built into Windows, extensible through installed codecs, and gives one image-processing interface across formats.
- [ ] WIC decoder-native transforms are preferred because scaling during decode is usually more efficient than full decode followed by resize.
- [ ] `IShellItemImageFactory::GetImage` should not perform extraction on the UI thread. Cache-only use on UI-sensitive paths is acceptable.
- [ ] Microsoft recommends Raw Input over low-level hooks in most cases where low-level hooks would otherwise monitor input.
- [ ] If a low-level mouse hook is used, it must run on a dedicated hook thread, pass work off immediately, and return immediately.
- [ ] `SetWinEventHook` callbacks require a message loop on the calling thread, so keep the main loop simple and reliable.


### 4.7 Dependency approval checklist

Every new dependency must answer:

- [ ] What feature requires this dependency?
- [ ] Is there a Windows-native API that avoids it?
- [ ] How much binary size does it add?
- [ ] Does it allocate during idle?
- [ ] Does it spawn threads?
- [ ] Does it perform I/O?
- [ ] Can it be feature-gated?
- [ ] Can it be replaced by a smaller crate?
- [ ] Does it increase startup time?
- [ ] Does it increase attack surface for remote images?

---

## 5. Windows Native Integration Checklist

### 5.1 App process

- [blocked] Build as a native Windows GUI app. Blocked: native shell builds, but full preview behavior is incomplete.
- [x] Avoid console window in release builds.
- [x] Add tray icon only after MVP preview works. Hover now has a generated high-resolution runtime tray icon plus enable/startup/options/exit tray controls.
- [x] Do not open a settings window at startup.
- [x] Do not initialize optional decoders until needed.
- [x] Do not initialize browser host unless browser mode is installed or requested.

### 5.2 Overlay window

- [x] Create a borderless Win32 window.
- [blocked] Use topmost behavior only while preview is visible. Blocked: overlay is created topmost then hidden; final visible-only topmost behavior needs runtime verification.
- [x] Use no taskbar entry.
- [x] Use no Alt-Tab entry.
- [x] Do not steal focus.
- [blocked] Support per-monitor DPI. Blocked: DPI messages force Dormant, but layout handling is not complete.
- [blocked] Support multi-monitor positioning. Blocked: placement math exists, but native monitor selection is not wired.
- [blocked] Support fractional DPI scaling. Blocked: native DPI conversion is not wired.
- [blocked] Use DWM-friendly composition. Blocked: Direct2D/DirectComposition backend is not implemented.
- [blocked] Render through Direct2D, DirectComposition, or another native low-overhead path. Blocked: current shell paints a neutral GDI test surface only.
- [x] Keep the preview window hidden when not needed.
- [blocked] Destroy or release heavyweight surfaces when hidden if memory pressure is detected. Blocked: no heavyweight surfaces are allocated yet; memory-pressure path not implemented.

### 5.3 Click-through and wheel input tension

Important: a fully click-through overlay may not receive mouse wheel messages. The product still requires wheel zoom by default. Implement this carefully.

Preferred approach:

- [ ] Keep overlay click-through for normal clicks.
- [x] Capture wheel input only while the preview is visible.
- [x] Only intercept wheel input when the cursor is inside the preview rectangle.
- [x] Pass all wheel input through when cursor is outside the preview rectangle.
- [x] Do not run the wheel capture mechanism while preview is hidden.
- [x] Do not run the wheel capture mechanism while app is disabled.
- [x] Do not run the wheel capture mechanism in fullscreen apps.

Acceptable technical options:

- [x] Option A: temporary low-level mouse hook only while preview is visible.
- [ ] Option B: Raw Input with input sink if it can reliably receive wheel deltas without focus.
- [ ] Option C: temporarily disable click-through only for wheel interaction, but verify it does not break clicks or steal focus.

Strict rule for any hook:

- [ ] The hook callback must do almost nothing.
- [ ] The hook callback may only inspect wheel delta, cursor point, and preview rectangle.
- [ ] The hook callback must immediately pass events through unless the preview is visible and cursor is inside preview bounds.
- [ ] The hook callback must never decode, allocate large memory, do I/O, log heavily, or block.
- [x] The hook must be installed only while preview is visible.
- [x] The hook must be removed immediately when preview hides.

### 5.4 Fullscreen and game safety

- [ ] Detect fullscreen foreground windows.
- [ ] Disable hover detection in fullscreen apps by default.
- [ ] Disable overlay in exclusive fullscreen contexts.
- [ ] Maintain a per-app blocklist.
- [ ] Add default blocklist entries for common games only after testing.
- [ ] If detection is uncertain, prefer not showing the preview.

---

## 6. Hover Detection Checklist

### 6.1 Foreground context gate

- [x] Do no hover work when the foreground context is ineligible.
- [ ] Eligible local contexts for MVP:
  - [x] File Explorer
  - [x] Desktop shell
  - [x] Common Open dialog
  - [x] Common Save dialog
- [ ] Optional later contexts:
  - [ ] XYplorer
  - [ ] Directory Opus
  - [ ] Total Commander
  - [ ] Other file managers by user rules
- [x] Browser contexts must use the companion extension instead of native DOM guessing.

### 6.2 Cursor stability

- [blocked] Track the current cursor point only while an eligible context is active. Blocked: eligible timer exists; cursor/target resolver is not wired.
- [x] Use a low-frequency timer only while eligible.
- [x] Suggested eligible polling rate: 30 Hz maximum.
- [x] Stop timer when foreground context becomes ineligible.
- [x] Stop timer when app is disabled.
- [blocked] Stop timer when system is locked or session is unavailable. Blocked: power/session messages force Dormant, but session lock notification is not registered yet.
- [x] Trigger hover only after cursor remains over same target for hover delay.
- [x] Reset hover timer if target changes.
- [blocked] Reset hover timer if cursor moves more than a small threshold. Blocked: path identity stability is wired, but pixel movement threshold is not yet applied.
- [x] Suggested movement threshold before hover: 3 logical pixels.

### 6.3 Target resolution order for File Explorer and Desktop

Try in this order:

- [ ] Shell item under cursor through Shell APIs if available.
- [ ] Explorer view COM interfaces if reliable.
- [blocked] UI Automation element under cursor as fallback. Blocked: MSAA `AccessibleObjectFromPoint` is wired; a separate UI Automation fallback is not yet wired.
- [x] Foreground Explorer folder plus item text as fallback.
- [x] Reject target if path cannot be confidently resolved.
- [x] Reject target if target is a folder unless folder preview is added later.
- [ ] Reject target if target is not local or allowed remote.
- [x] Sniff file header before trusting extension.

### 6.4 Target validation

- [ ] Confirm target is stable at decode start.
- [x] Confirm target still exists.
- [x] Confirm file size is within configured limits.
- [x] Confirm file is readable.
- [x] Confirm file header looks image-like or Shell/WIC reports image-like.
- [x] Reject dangerous or unsupported sources gracefully.
- [x] Do not display error popups for normal unsupported files.
- [ ] Optional tiny error badge only if user enables diagnostics.

---

## 7. Image Format Support Checklist

### 7.1 Mandatory common formats

- [ ] JPEG
- [ ] Progressive JPEG
- [ ] PNG
- [x] GIF first frame
- [x] Animated GIF playback later, optional for MVP
- [ ] BMP
- [ ] TIFF
- [ ] ICO
- [ ] WebP
- [ ] Animated WebP later, optional after static WebP
- [ ] AVIF
- [ ] HEIC and HEIF through WIC/system codec where available
- [ ] SVG through rasterization feature
- [ ] JPEG XL through optional feature

### 7.2 Windows-native first strategy

- [x] Try WIC first for local file decoding.
- [x] Use WIC downscale-on-decode when possible.
- [x] Use installed system codecs when available.
- [blocked] Use Shell thumbnail provider fallback for formats with thumbnail support but no full decode support. Blocked: Shell thumbnail fallback is documented but not implemented.
- [blocked] Use embedded thumbnails for RAW and large formats when available. Blocked: embedded thumbnail extraction is not implemented.
- [x] Use Rust fallback decoder only when WIC cannot decode or lacks required support.

### 7.3 Rust fallback strategy

- [x] Use Rust fallback decoders behind feature flags.
- [x] Keep fallback decoders lazy-loaded where possible.
- [x] Keep SVG rasterization isolated.
- [x] Keep AVIF/JXL optional if binary size is too high.
- [x] Document fallback order per format.
- [x] Document which formats rely on installed Windows codecs.

### 7.4 Format fallback order

For each preview source:

- [ ] Try Shell thumbnail cache when it is cheap and sufficient.
- [ ] Try embedded preview or embedded thumbnail.
- [ ] Try WIC full decode with downscale.
- [ ] Try Rust decoder fallback.
- [ ] Try generic file icon only if user enables non-image fallback.
- [ ] Otherwise show nothing.

### 7.5 Image correctness

- [ ] Apply EXIF orientation.
- [ ] Preserve transparency.
- [ ] Correctly composite transparent images over checkerboard or neutral background.
- [ ] Support color profiles later, but do not block MVP on advanced color management.
- [ ] Handle Unicode paths.
- [ ] Handle long Windows paths.
- [ ] Handle locked files gracefully.
- [ ] Handle corrupt images gracefully.
- [ ] Handle truncated images gracefully.
- [ ] Handle zero-byte files gracefully.
- [ ] Handle massive dimensions gracefully.
- [ ] Handle files renamed with wrong extensions through header sniffing.

---

## 8. Decoder and Memory Safety Checklist

### 8.1 Decode job lifecycle

- [blocked] Decode jobs run on a worker thread. Blocked: worker is not wired into native shell yet.
- [x] UI thread never decodes.
- [x] Each decode job has a cancellation token.
- [blocked] Cancel current decode when hover target changes. Blocked: target-change path is not wired.
- [blocked] Cancel current decode when preview hides. Blocked: native decode worker is not wired.
- [x] Cancel current decode when source is replaced by newer hover target.
- [blocked] Do not deliver stale decode results to the overlay. Blocked: overlay result delivery is not wired.
- [x] Every decode result must include a generation ID.
- [blocked] Overlay must ignore decode result if generation ID is stale. Blocked: overlay result delivery is not wired.

### 8.2 Decode limits

- [x] Maximum source file size default: 512 MB for local files.
- [x] Maximum decoded pixels default: 200 megapixels.
- [x] Maximum output preview bitmap dimension default: based on monitor size and max zoom.
- [blocked] Maximum animation frames decoded ahead: 2. Blocked: animated GIF playback is implemented with capped full-animation decode; streaming two-frame lookahead is not implemented yet.
- [x] Maximum remote fetch size default: 50 MB.
- [x] Maximum remote fetch time default: 10 seconds.
- [blocked] Maximum decode time before cancellation warning: 2 seconds. Blocked: worker timeout diagnostics are not implemented.
- [x] Hard fail gracefully when limits are exceeded.

### 8.3 Downscale strategy

- [ ] Decode to the smallest useful size for current preview scale.
- [ ] For initial Fit All preview, do not decode full 100 MP image if a smaller decode is possible.
- [ ] If user zooms beyond current bitmap quality, schedule a higher-resolution decode.
- [ ] Show existing lower-resolution bitmap while higher-resolution decode is pending.
- [ ] Cancel higher-resolution decode if user zooms back out or target changes.
- [ ] Never decode full resolution just because the file is hovered.

### 8.4 Cache strategy

- [x] Default decoded bitmap cache: 64 MB.
- [blocked] Default local metadata cache: small, path plus mtime plus dimensions only. Blocked: metadata cache is not wired.
- [x] Default remote cache: disabled unless browser mode is enabled.
- [x] Default remote decoded cache: 128 MB when browser mode is enabled.
- [x] Use LRU eviction.
- [blocked] Evict on memory pressure. Blocked: Windows memory-pressure notifications are not wired.
- [blocked] Key cache by source ID, file modified time, target decode size, and orientation. Blocked: decoded bitmap cache is not wired to preview path.
- [x] Never let cache exceed hard limit.
- [blocked] Provide debug counters for cache hits and misses. Blocked: diagnostics panel/counters are not exposed.

---

## 9. Rendering Checklist

### 9.1 Renderer requirements

- [x] Use native rendering.
- [blocked] Prefer Direct2D or DirectComposition. Blocked: backend benchmarking/implementation not done.
- [blocked] Avoid software rendering except fallback. Blocked: current working overlay uses GDI as an interim fallback while Direct2D/DirectComposition remains unimplemented.
- [x] Avoid GPU work while hidden.
- [x] Release or idle render resources while hidden.
- [x] Upload bitmap only when source or decode size changes.
- [x] Do not re-upload bitmap every frame.
- [x] Render only when visible and dirty.
- [x] Use dirty flags.
- [x] No continuous render loop unless animation is playing.

### 9.2 Visual style

- [ ] Default background: neutral dark or adaptive.
- [ ] Rounded corners optional, but must be cheap.
- [ ] Shadow optional, but must be cheap.
- [ ] No blur by default.
- [ ] No animated open/close by default.
- [ ] Fade animation optional and disabled by default.
- [ ] Metadata footer disabled by default.
- [ ] Transparent image checkerboard optional.

### 9.3 Animation

- [blocked] Animated GIF and WebP playback is optional after MVP. Blocked: GIF playback is wired; animated WebP remains unimplemented.
- [blocked] If animation is enabled, decode frames lazily. Blocked: GIF uses capped full-animation decode rather than streaming/lazy decode.
- [x] Respect frame timing.
- [x] Stop animation when preview hidden.
- [x] Stop animation when preview is not foreground-relevant.
- [x] Cap animation memory.
- [x] Cap animation frame rate.
- [ ] Allow user to disable animation.

---

## 10. Browser Companion Extension Checklist

Browser support must be separate from the core native app. The native app should not try to inspect browser internals directly.

### 10.1 Browser extension scope

- [ ] Build a minimal Manifest V3 extension.
- [ ] Support Chromium browsers first: Chrome, Edge, Brave, Vivaldi.
- [ ] Support Firefox later with a separate manifest if needed.
- [ ] Extension must not render the preview in normal mode.
- [ ] Extension only detects hover targets and sends source data to native host.
- [ ] Extension has no analytics.
- [ ] Extension stores no browsing history.

### 10.2 Browser hover target detection

- [ ] Detect hovered `img` elements.
- [ ] Detect `picture` and `srcset` best source.
- [ ] Detect links directly pointing to image files.
- [ ] Detect CSS `background-image` where practical.
- [ ] Detect video poster images where practical.
- [ ] Detect thumbnails linking to larger images where practical.
- [ ] Prefer highest-quality source that is reasonable.
- [ ] Respect user blocklist.
- [ ] Respect site allowlist if privacy mode is enabled.

### 10.3 Native messaging host

- [ ] Implement native messaging host in Rust.
- [ ] Validate every incoming URL.
- [ ] Enforce remote size limits.
- [ ] Enforce remote timeout limits.
- [ ] Enforce allowed schemes: `http` and `https` only by default.
- [ ] Reject `file`, `chrome`, `edge`, `about`, and extension URLs unless specifically supported.
- [ ] Strip credentials from URLs before diagnostics.
- [ ] Do not persist URL history.
- [ ] Cache remote images only in memory by default.

### 10.4 Browser preview behavior

- [ ] Use the same overlay as local file preview.
- [ ] Use same wheel zoom behavior.
- [ ] Use same cursor-centered zoom behavior.
- [ ] Hide preview when browser sends hover exit.
- [ ] Hide preview when tab changes.
- [ ] Hide preview when browser window loses focus.
- [ ] Cancel remote fetch if hover exits.
- [ ] Cancel decode if hover exits.

---

## 11. Settings Defaults Checklist

Create a settings file only after MVP behavior exists. Defaults must work without the user configuring anything.

### 11.1 Suggested settings file

```toml
[general]
enabled = true
start_with_windows = false
show_tray_icon = true
telemetry = false

[hover]
delay_ms = 250
movement_threshold_px = 3
eligible_poll_hz = 30
idle_poll_hz = 0
foreground_detection = "winevent"
hide_on_escape = true
hide_on_click = true
hide_in_fullscreen = true

[preview]
initial_mode = "fit_all"
max_monitor_width_ratio = 0.80
max_monitor_height_ratio = 0.80
min_width_px = 160
min_height_px = 160
cursor_gap_px = 16
metadata_footer = false
rounded_corners = true
shadow = true
fade_animation = false

[zoom]
wheel_zoom_enabled = true
plain_wheel_zooms = true
cursor_centered_zoom = true
wheel_zoom_step = 0.125
shift_wheel_zoom_step = 0.025
ctrl_wheel_zoom_step = 0.25
min_zoom_percent = 5
max_zoom_percent = 800
advanced_max_zoom_percent = 3200
snap_to_fit = true
snap_to_100 = true
wheel_browse_enabled = false
alt_wheel_browse_enabled = false

[pan]
enabled = true
middle_drag_pan = true
space_left_drag_pan = true
arrow_key_pan = true

[decode]
wic_first = true
wic_native_downscale = true
shell_thumbnail_fallback = true
shell_thumbnail_cache_only_first = true
rust_fallback_decoders = true
lazy_initialize_decoders = true
max_local_file_size_mb = 512
max_decoded_megapixels = 200
max_worker_threads = 1
allow_second_worker_when_visible = true
cancel_stale_decodes = true

[cache]
decoded_cache_mb = 64
remote_cache_enabled = false
remote_decoded_cache_mb = 128
evict_on_memory_pressure = true
hard_fail_when_cache_limit_exceeded = true

[power]
force_dormant_on_session_lock = true
force_dormant_on_sleep = true
disable_on_battery_saver = true

[diagnostics]
enabled = false
hot_path_logging = false
perf_counters = false

[browser]
enabled = false
native_messaging = true
remote_fetch_timeout_seconds = 10
remote_max_size_mb = 50
memory_only_remote_cache = true

[preview_types]
images = true
videos = true
audio = true
folders = true
zips = true
text = true
sims_packages = true

[video]
audio_enabled = true
seek_seconds = 5
```

### 11.2 Settings UI requirements

- [x] Settings UI is optional for MVP.
- [x] Settings UI must not run at startup.
- [x] Settings UI must not require WebView.
- [x] Settings UI may be a small native dialog.
- [x] Settings UI may be a simple tray menu first.
- [x] All settings must have safe defaults.
- [x] Invalid settings must fall back to defaults.
- [x] Settings changes must not require restart when practical.

---

## 12. Implementation Milestones

### Milestone 1: Native overlay spike

Deliver:

- [x] Minimal Rust Windows GUI process.
- [x] Hidden overlay window.
- [blocked] Show overlay at test coordinates. Blocked: overlay show helper is not exposed/tested yet.
- [x] Render a test bitmap.
- [x] Hide overlay.
- [x] Avoid taskbar entry.
- [x] Avoid Alt-Tab entry.
- [x] Avoid focus steal.
- [blocked] Correct DPI scaling. Blocked: DPI messages are handled by entering Dormant, but scaling is not implemented.
- [blocked] Correct multi-monitor placement. Blocked: placement math exists, but monitor API integration is not wired.

Acceptance:

- [blocked] Overlay appears under 16 ms after internal show command. Blocked: runtime measurement not performed.
- [blocked] Overlay hides under 16 ms after internal hide command. Blocked: runtime measurement not performed.
- [blocked] Overlay does not appear in taskbar. Blocked: style is set, but runtime verification not performed.
- [blocked] Overlay does not appear in Alt-Tab. Blocked: style is set, but runtime verification not performed.
- [blocked] Overlay does not steal focus from File Explorer. Blocked: style is set, but runtime verification not performed.
- [blocked] Overlay remains inside monitor work area. Blocked: native monitor placement is not wired.

### Milestone 2: WIC local image decode

Deliver:

- [x] Decode JPEG through WIC.
- [x] Decode PNG through WIC.
- [x] Decode BMP through WIC.
- [x] Decode TIFF through WIC.
- [x] Decode GIF first frame through WIC.
- [blocked] Apply EXIF orientation where available. Blocked: WIC metadata orientation handling is not implemented.
- [x] Downscale before render where possible.
- [blocked] Render decoded bitmap in overlay. Blocked: decoded bitmap handoff to overlay is not wired.

Acceptance:

- [blocked] 100 mixed images preview correctly. Blocked: fixture integration suite not run.
- [x] Wrong-extension images are handled by header or decoder detection.
- [blocked] Corrupt images do not crash. Blocked: fixture integration suite not run.
- [blocked] Huge images do not freeze UI. Blocked: fixture integration suite not run.

### Milestone 3: Explorer and Desktop hover MVP

Deliver:

- [x] Detect eligible foreground context.
- [blocked] Detect image file under cursor in File Explorer. Blocked: Shell/UI Automation resolver not wired.
- [blocked] Detect image file under cursor on Desktop. Blocked: Shell/UI Automation resolver not wired.
- [blocked] Apply hover delay. Blocked: hover candidate tracking not wired.
- [blocked] Cancel hover on movement. Blocked: hover candidate tracking not wired.
- [blocked] Show preview for valid image. Blocked: preview path not wired.
- [blocked] Hide preview on target exit. Blocked: target tracking not wired.

Acceptance:

- [ ] Hover over JPG in Explorer shows preview.
- [ ] Hover over PNG on Desktop shows preview.
- [ ] Moving away hides preview.
- [ ] Rapid movement across files does not create stale previews.
- [ ] CPU is near zero when Explorer is not foreground.

### Milestone 4: Wheel zoom default-on

Deliver:

- [blocked] Implement plain wheel zoom over visible preview. Blocked: native wheel capture is not wired.
- [x] Implement cursor-centered zoom.
- [x] Implement Shift plus wheel fine zoom.
- [x] Implement Ctrl plus wheel coarse zoom.
- [x] Implement snap to fit scale.
- [x] Implement snap to 100 percent.
- [x] Implement maximum zoom cap.
- [x] Implement minimum zoom cap.
- [x] Implement pan clamp.
- [blocked] Implement Fit All shortcut. Blocked: keyboard controls are not wired.
- [blocked] Implement 100 percent shortcut. Blocked: keyboard controls are not wired.

Acceptance:

- [ ] Wheel up over preview zooms in.
- [ ] Wheel down over preview zooms out.
- [ ] The image point under the cursor remains anchored during zoom.
- [ ] Wheel outside preview passes through.
- [ ] Wheel does nothing when preview is hidden.
- [x] Wheel hook or capture mechanism is active only while preview is visible.
- [ ] Zoom never makes image vanish.
- [ ] Zoom never produces NaN, infinity, negative scale, or pan runaway.
- [ ] Zoom works in transient preview mode.
- [ ] Zoom works in locked preview mode.

### Milestone 5: Format expansion

Deliver:

- [ ] WebP static decode.
- [ ] AVIF decode.
- [ ] HEIC/HEIF through WIC/system codec.
- [ ] SVG rasterization.
- [ ] ICO decode.
- [ ] JPEG XL optional decode.
- [x] Animated GIF optional playback.
- [ ] Animated WebP optional playback.

Acceptance:

- [ ] Format matrix is updated.
- [ ] Missing codec paths fail gracefully.
- [ ] Each decoder has size and memory notes.
- [ ] Optional decoders can be disabled at compile time where practical.

### Milestone 6: Performance hardening and invisible-cost certification

Deliver:

- [ ] Decode cancellation.
- [ ] Hard decode limits.
- [ ] Cache hard limits.
- [ ] Stale result generation checks.
- [ ] No idle logging.
- [ ] No idle hooks.
- [ ] No idle Raw Input wheel capture.
- [ ] No background scanning.
- [ ] Foreground changes detected by event, not permanent foreground polling.
- [ ] `Dormant` state implemented exactly as section 3.4 describes.
- [ ] Worker thread sleeps or does not exist while idle.
- [ ] Renderer has dirty-flag present model.
- [ ] Debug performance counters behind feature flag.
- [ ] Release build profile added.
- [ ] `cargo tree` reviewed and dependency notes updated.

Acceptance:

- [ ] Idle CPU is **0.0 percent** in Dormant. Any non-zero sample must be investigated, classified, and either fixed or documented as an exception.
- [ ] Dormant state has zero active timers.
- [ ] Dormant state has zero wheel hooks.
- [ ] Dormant state has zero render presents.
- [ ] Dormant state has zero decode jobs.
- [ ] RAM returns to cache cap after stress test.
- [ ] No unbounded queue growth during rapid cursor movement.
- [ ] No stale preview after rapid hover changes.
- [ ] No GPU activity while preview hidden.
- [ ] No repeated disk reads after startup settles.
- [ ] No network activity unless browser mode is enabled and actively previewing.

### Milestone 7: Browser companion extension

Deliver:

- [x] Manifest V3 extension.
- [x] Content script hover detection.
- [x] Native messaging host.
- [blocked] Remote image fetch in Rust host. Blocked: no HTTP stack is selected; core remains zero network by default.
- [blocked] Remote decode and overlay preview. Blocked: remote fetch and native overlay handoff are not implemented.
- [blocked] Remote cancellation. Blocked: remote fetch is not implemented.
- [blocked] Same wheel zoom behavior. Blocked: native wheel capture is not wired.

Acceptance:

- [ ] Hovering a normal web image previews it.
- [ ] Hovering a `picture` source previews best source.
- [ ] Hovering an image link previews image.
- [ ] Remote fetch cancels when hover exits.
- [ ] No remote URL history persisted.
- [ ] Browser extension performs no heavy rendering.

### Milestone 8: Installer and startup behavior

Deliver:

- [x] WiX or MSIX installer.
- [x] Optional startup registration.
- [x] Tray icon.
- [x] Enable/disable toggle.
- [blocked] Browser native messaging registration. Blocked: example manifest exists; installer registration is not implemented.
- [blocked] Clean uninstall. Blocked: installer not built/tested.

Acceptance:

- [ ] Install works on Windows 11.
- [ ] Uninstall removes app files.
- [ ] Uninstall removes native messaging registration.
- [x] Startup setting can be enabled and disabled.
- [x] App runs without admin rights.

---

## 13. Testing Checklist

### 13.1 Unit tests

- [x] Zoom math preserves cursor anchor.
- [x] Zoom clamps correctly.
- [blocked] Zoom snaps correctly. Blocked: snap model is implemented, but no dedicated crossing assertion has been added yet.
- [x] Pan clamps correctly.
- [x] Fit All scale calculation is correct.
- [x] Fit Width scale calculation is correct.
- [x] Fit Height scale calculation is correct.
- [x] Placement flips correctly near monitor edges.
- [x] Cache evicts correctly.
- [x] Stale decode results are ignored.
- [x] Settings parse correctly.
- [x] Invalid settings fall back correctly.

### 13.2 Integration tests

- [blocked] Load and preview local JPEG. Blocked: runtime preview overlay is not wired.
- [blocked] Load and preview local PNG. Blocked: runtime preview overlay is not wired.
- [blocked] Load and preview local WebP. Blocked: runtime preview overlay is not wired and codec availability depends on WIC/system codec.
- [blocked] Load and preview local AVIF. Blocked: runtime preview overlay is not wired and codec availability depends on WIC/system codec.
- [blocked] Load and preview local SVG. Blocked: SVG rasterizer not selected.
- [blocked] Load and preview corrupt file without crash. Blocked: no fixture integration suite yet.
- [blocked] Load and preview huge file without UI freeze. Blocked: no fixture integration suite yet.
- [blocked] Decode cancellation works. Blocked: core cancellation exists; worker integration test not wired.
- [blocked] Preview hides on Escape. Blocked: keyboard controls not wired.
- [blocked] Preview hides on foreground change. Blocked: foreground change enters Dormant in shell; runtime preview hide not verified.
- [blocked] Preview disables in fullscreen app. Blocked: fullscreen detection exists; runtime preview suppression not verified.

### 13.3 Manual UX tests

- [ ] Hover slowly across a folder of images.
- [ ] Move quickly across a folder of images.
- [ ] Hover the same image repeatedly.
- [ ] Hover a very small image.
- [ ] Hover a very large image.
- [ ] Wheel zoom in and out repeatedly.
- [ ] Zoom at image center.
- [ ] Zoom near image corners.
- [ ] Zoom near preview edges.
- [ ] Shift plus wheel fine zoom.
- [ ] Ctrl plus wheel coarse zoom.
- [ ] Fit All shortcut.
- [ ] 100 percent shortcut.
- [ ] Pan after zooming in.
- [ ] Multi-monitor placement.
- [ ] Different DPI monitors.
- [ ] Windows scaling at 100 percent.
- [ ] Windows scaling at 125 percent.
- [ ] Windows scaling at 150 percent.
- [ ] Windows scaling at 200 percent.

### 13.4 Performance tests

Idle test:

- [ ] Launch app.
- [ ] Do nothing for 10 minutes.
- [ ] Confirm CPU is **0.0 percent** in Dormant. Any non-zero sample must be investigated, classified, and either fixed or documented as an exception.
- [ ] Confirm GPU is 0.0 percent.
- [ ] Confirm disk reads do not repeat.
- [ ] Confirm memory stays stable.
- [ ] Confirm handle count stays stable.

Foreground Explorer test:

- [ ] Open Explorer to folder with no hovering.
- [ ] Leave cursor still.
- [ ] Confirm CPU under 0.1 percent average.
- [ ] Confirm no image decoding.
- [ ] Confirm no folder scan.

Rapid hover test:

- [ ] Create folder with 10,000 files.
- [ ] Include at least 1,000 images.
- [ ] Move cursor rapidly across list and thumbnails.
- [ ] Confirm decode queue does not grow unbounded.
- [ ] Confirm stale previews do not appear.
- [ ] Confirm app remains responsive.

Huge image test:

- [ ] Test 100 megapixel JPEG.
- [ ] Test 500 MB TIFF.
- [ ] Test corrupt PNG.
- [ ] Test animated WebP.
- [ ] Test SVG with huge canvas.
- [ ] Confirm no crash.
- [ ] Confirm no UI freeze.
- [ ] Confirm limits are enforced.

Wheel zoom stress test:

- [ ] Show preview.
- [ ] Spin wheel rapidly upward.
- [ ] Spin wheel rapidly downward.
- [ ] Hold Shift and spin wheel.
- [ ] Hold Ctrl and spin wheel.
- [ ] Move cursor while zooming.
- [ ] Confirm no jitter explosion.
- [ ] Confirm no NaN scale.
- [ ] Confirm no pan runaway.
- [ ] Confirm cursor-centered anchor remains stable.


### 13.5 Invisible-cost profiling checklist

Use these tools or equivalent measurements before claiming the app is lightweight.

- [blocked] Task Manager for quick CPU, memory, disk, network, and GPU sanity checks. Blocked: requires running the finished native GUI interactively.
- [blocked] Resource Monitor for disk and network confirmation. Blocked: requires running the finished native GUI interactively.
- [blocked] Windows Performance Recorder or Windows Performance Analyzer for deeper CPU wakeup analysis. Blocked: requires running the finished native GUI interactively.
- [blocked] Process Explorer for threads, handles, and loaded modules. Blocked: requires running the finished native GUI interactively.
- [blocked] GPUView or equivalent only if GPU usage is suspicious. Blocked: requires running the finished native GUI interactively.
- [blocked] `cargo bloat` or equivalent for binary-size regressions. Blocked: tool not installed/used in this pass.
- [x] `cargo tree` for dependency creep.
- [blocked] A custom debug-only internal counter panel for: Blocked: internal counters exist, but no panel is exposed.
  - [ ] Current state.
  - [ ] Active timers.
  - [ ] Decode queue length.
  - [ ] Render presents.
  - [ ] Bitmap uploads.
  - [ ] Cache bytes.
  - [ ] Hook installed true or false.
  - [ ] Raw Input capture active true or false.
  - [ ] Last transition reason.

Pass criteria:

- [ ] In `Dormant`, all internal counters that represent active work must be zero.
- [ ] In `ForegroundEligible`, only the cursor timer may be active.
- [ ] In `PreviewVisible`, active work must stop immediately after hide.
- [ ] In stress tests, counters must return to baseline without restarting the app.


---

## 14. Acceptance Definition for MVP

The MVP is complete only when all of these are true:

- [ ] Rust native Windows app launches without console window.
- [ ] App has no heavy runtime dependency.
- [ ] App shows local image previews in File Explorer.
- [ ] App shows local image previews on Desktop.
- [ ] App starts preview only after hover delay.
- [ ] App cancels stale preview work.
- [ ] App uses WIC first.
- [ ] App supports JPEG and PNG.
- [ ] App applies fit-to-screen preview placement.
- [ ] App does not steal focus.
- [ ] App does not appear in taskbar.
- [ ] App does not appear in Alt-Tab.
- [ ] App has mouse-wheel zoom enabled by default.
- [ ] App has cursor-centered zoom enabled by default.
- [ ] App has Shift plus wheel fine zoom.
- [ ] App has Ctrl plus wheel coarse zoom.
- [ ] App has Fit All and 100 percent shortcuts.
- [ ] App uses no idle global hook.
- [ ] App uses no background image indexing.
- [ ] App uses no telemetry.
- [ ] App hits the **zero idle work target**: 0.0 percent CPU, 0.0 percent GPU, 0 disk reads after startup settles, 0 network, 0 hidden decode jobs, 0 hidden render passes, and 0 global mouse hooks while Dormant.
- [ ] App does no GPU work while hidden.
- [ ] App does not leak memory after repeated previews.

---

## 15. Agent Notes for Ambiguous Choices

### 15.1 If a feature conflicts with lightweight goals

- [ ] Prefer lightweight behavior.
- [ ] Put heavy behavior behind a setting.
- [ ] Disable heavy behavior by default.
- [ ] Document the tradeoff.
- [ ] Add a performance test.

### 15.2 If a format requires a large dependency

- [ ] First try WIC/system codec.
- [ ] Then try Shell thumbnail provider.
- [ ] Then consider optional Rust decoder.
- [ ] Feature-gate it.
- [ ] Document binary size impact.
- [ ] Keep MVP unaffected.

### 15.3 If wheel zoom conflicts with click-through behavior

- [ ] Keep click-through for normal clicks.
- [ ] Capture wheel only while visible.
- [ ] Capture wheel only inside preview bounds.
- [ ] Pass everything else through.
- [ ] Keep hook inactive while preview is hidden.
- [ ] Add a test that underlying Explorer still receives wheel when cursor is outside preview.

### 15.4 If Explorer hover detection is unreliable

- [ ] Do not guess dangerously.
- [ ] Use Shell APIs first.
- [ ] Use UI Automation fallback.
- [ ] Add diagnostics mode to show resolved path.
- [ ] Prefer no preview over wrong preview.
- [ ] Keep detection isolated so better methods can replace it later.

---

## 16. Future Features After MVP

Do not implement these until MVP is fast and stable.

- [ ] Browser extension support.
- [x] Animated GIF playback.
- [ ] Animated WebP playback.
- [ ] RAW embedded preview support.
- [ ] Video poster preview.
- [ ] Pinned comparison mode.
- [ ] Metadata footer.
- [ ] Color picker.
- [ ] Copy image to clipboard.
- [ ] Open containing folder.
- [ ] Site-specific browser rules.
- [ ] App-specific hover rules.
- [ ] Optional plugin system.
- [ ] Optional AI upscaling plugin, disabled by default and separate from core app.

---

## 17. Reference Behavior Notes

These are inspiration notes, not dependencies.

- XYplorer-style image preview behavior includes mouse-wheel interaction for previews, where the wheel can scroll previews or scale images.
- XYplorer added a Zoom to Cursor style image preview behavior in 2026, where mouse-wheel zoom focuses on the area under the cursor.
- Hover must implement the image-scaling version by default: plain wheel over preview zooms in and out, cursor-centered.
- Hover must not make plain wheel browse next or previous image by default.

---



## 18. Required Reference Implementation Starting Point

The agent must study this repository before implementing Explorer hover detection, tray behavior, preview-window behavior, and animated image handling:

- Reference repository: https://github.com/cosmokud/rust-hover-preview
- Repository role: excellent starting point and prior-art implementation, not the final architecture ceiling.
- Project summary from the repository: a Windows 11 Rust system tray app that previews image and video files in File Explorer on mouse hover or keyboard navigation, inspired by QTTabBar hover preview.

### 18.1 Mandatory first pass through the reference repository

- [x] Clone or inspect `https://github.com/cosmokud/rust-hover-preview` before starting Hover implementation.
- [x] Read `README.md`.
- [x] Read `Cargo.toml`.
- [x] Read `CHANGELOG.md`.
- [x] Inspect the `src/` modules for Explorer integration, preview-window creation, tray integration, config handling, image decoding, animation handling, and video-preview handling.
- [x] Build the reference project locally on Windows 11 if possible.
- [blocked] Run the reference project in File Explorer and record observed behavior. Blocked: this tool session cannot drive a real File Explorer hover/navigation session or inspect tray behavior interactively.
- [x] Create `docs/reference-rust-hover-preview-audit.md` before writing major Hover code.
- [x] In the audit, list what should be reused conceptually, what should be replaced, and what should be avoided for the zero-resource Hover goal.

### 18.2 Things to learn from the reference repository

- [x] Study how it resolves hovered Explorer items.
- [x] Study how it resolves keyboard-focused Explorer items.
- [x] Study how it uses Windows accessibility APIs, including MSAA and UI Automation, for Explorer item detection.
- [x] Study how it uses Shell COM APIs to identify active Explorer windows and folders.
- [x] Study how it creates a topmost, non-activating preview window that avoids focus stealing.
- [x] Study how it exposes tray controls for enabling/disabling preview, delay, position behavior, startup, and media-related settings.
- [x] Study how its config file is stored and loaded.
- [x] Study how it handles animated GIF and animated WebP previews.
- [x] Study its changelog entries related to reducing animated GIF/WebP CPU spikes, queue-based frame transfer, redraw throttling, decoder contention, and rapid-hover behavior.
- [x] Study its fixes around stale Explorer folder state, false preview triggering, and preview flashes when switching targets.
- [x] Treat these fixes as known traps that Hover must avoid from the beginning.

### 18.3 Things not to copy directly without benchmarking

- [blocked] Do not automatically copy GDI rendering. Benchmark Direct2D and DirectComposition first. Blocked: Direct2D/DirectComposition benchmark not performed; current overlay is a GDI test surface.
- [x] Do not automatically copy polling behavior. Hover must prefer the Dormant state and event-driven foreground activation.
- [x] Do not automatically copy folder indexing or folder caching if it creates idle disk, CPU, or memory cost.
- [x] Do not automatically copy FFmpeg process-based video preview into the core app unless explicitly requested. The user explicitly requested video playback, so Hover now starts ffplay only for active video previews and stops it when the preview hides.
- [x] Do not automatically copy global hooks if a lighter event-driven method works.
- [x] Do not automatically copy dependencies unless each dependency passes the dependency budget checklist.
- [x] Do not let compatibility behavior from the reference app weaken Hover's stricter zero-resource goal.

### 18.4 Required comparison document

Create `docs/reference-rust-hover-preview-audit.md` with this exact checklist:

```md
# rust-hover-preview Reference Audit

Reference: https://github.com/cosmokud/rust-hover-preview

## Useful patterns to keep conceptually

- [ ] Explorer hover target resolution:
- [ ] Keyboard-focused item preview:
- [ ] Tray controls:
- [ ] Non-activating topmost preview window:
- [ ] Config format and storage:
- [ ] Animated GIF/WebP handling lessons:
- [ ] Rapid-hover cancellation lessons:

## Patterns to replace for Hover

- [ ] Rendering path:
- [ ] Decoder path:
- [ ] Foreground/window activation path:
- [ ] Timer and polling behavior:
- [ ] Cache behavior:
- [x] Video preview behavior: external backends are discovered in order, startup foreground churn gets a grace period, player window attachment is throttled, audio is controlled by settings, wheel/arrow seeking is wired, and player exits show a stable status panel instead of flickering restarts.

## Known traps from the reference changelog

- [ ] Animated WebP CPU spikes:
- [ ] Duplicate frame copying and memory churn:
- [ ] Redraw conditions during animation:
- [ ] Decoder contention during long streams:
- [ ] Stale folder state after navigation:
- [ ] False preview triggering after folder changes:
- [ ] Previous image flashing during rapid target switches:
- [ ] Mislabeled file extensions:

## Hover decisions

- [ ] Keep:
- [ ] Replace:
- [ ] Defer:
- [ ] Reject:

## Performance implications

- [ ] Expected idle cost:
- [ ] Expected eligible-foreground cost:
- [ ] Expected hover-confirmation cost:
- [ ] Expected preview-visible cost:
- [ ] Expected animation-visible cost:
````

### 18.5 How the reference affects the Hover build order

- [x] Build Milestone 3, Explorer hover detection MVP, only after reviewing the reference Explorer detection approach.
- [x] Build Milestone 1, overlay spike, while comparing against the reference topmost no-focus preview behavior.
- [blocked] Build Milestone 4, animated image support, only after reviewing the reference queue-based animation improvements. Blocked: animated GIF support is implemented, but animated WebP and streaming queue playback are not implemented.
- [x] Build Milestone 5, performance hardening, using the reference changelog as a checklist of problems to prevent.
- [blocked] Keep Hover's final architecture stricter than the reference: WIC-first decoding, Direct2D/DirectComposition rendering, Dormant no-work state, feature-gated fallbacks, and no process-based video in the core app unless explicitly enabled/requested. Blocked: Direct2D/DirectComposition rendering is not implemented yet; the requested process-based video path is now active-hover-only and gated by preview/video settings.

---

## 19. Absolute Best-Tech Summary for the Agent

Use this as the final architecture decision unless benchmarks disprove it.

- [x] Core app: Rust native Win32 GUI.
- [x] Hot-path Windows calls: `windows-sys`.
- [x] COM-heavy imaging and rendering: `windows` only where safer and still lightweight.
- [x] Foreground detection: `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`.
- [x] Idle state: message loop only, no polling.
- [x] Eligible state: low-frequency cursor timer only.
- [blocked] Hover confirmation: stable target plus delay before decode. Blocked: core defaults exist, but native target resolver is not wired yet.
- [blocked] Decode: WIC first, Shell cached thumbnail or embedded thumbnail where cheaper, Rust fallback decoders feature-gated. Blocked: WIC path exists; Shell thumbnail and embedded thumbnail paths are not implemented.
- [blocked] Downscale: decoder-native WIC transform first. Blocked: WIC scaler path exists, but runtime fixture verification is not complete.
- [blocked] Render: DirectComposition plus Direct2D if benchmark winner, otherwise Direct2D direct path. Blocked: dirty render abstraction exists, native shell currently uses a GDI test paint surface.
- [blocked] Wheel zoom: normal message or Raw Input while visible, low-level hook only as preview-visible fallback. Blocked: native wheel capture is not wired yet.
- [x] Network: absent from core resident app.
- [x] Browser support: optional native messaging host plus tiny extension.
- [x] Caches: hard-capped LRU only.
- [blocked] Workers: one lazy worker by default. Blocked: replacement model exists, but native decode worker is not wired yet.
- [x] Diagnostics: off by default.
- [x] Heavy features: disabled by default and feature-gated.
- [blocked] Success definition: **zero visible cost in `Dormant`**; all measurable cost must appear only during an intentional preview, or be filed as a defect/exception. Blocked: runtime profiling is not complete.


## 20. Final One-Sentence Build Brief

Build Hover as a Rust-native, event-driven Windows 11 hover image preview utility with a focusless Direct2D/DirectComposition overlay, WIC-first decoder-native downscaling, optional feature-gated Rust codec fallbacks, strict cache limits, a strict zero-work Dormant state, no indexing, zero idle hooks, zero idle rendering, zero idle decoding, zero idle polling, and default-on XYplorer-like mouse-wheel cursor-centered zoom for previews.
