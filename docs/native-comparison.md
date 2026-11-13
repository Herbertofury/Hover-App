# Native file comparison

Development feature; no verified release. Compare two to four local files in a
retained, resizable native Windows window. Each pane keeps its source identity,
loading/error state and preview stage. Decoded panes arrive before bounded
differences finish. File 1 is the reference; choose another pane for its summary
and difference visualization. Open alone returns to the existing single viewer.

The Explorer resolver reads the foreground Shell view's selected items when
the hovered file belongs to that selection. It reads at most four paths and
shows the selection's total separately. This source integration has **not** been
verified with physical Explorer interaction, hidden extensions or Explorer tabs.
The first-party CLI provides a tested way to open the actual comparison window.

Images support wheel zoom, drag pan, linked actual pixel scale and normalized
image center, or independent views. Different image/window bounds can clamp
each center differently. Initial fits are independent until a linked gesture.
Transparent WIC frames composite their premultiplied pixels over a checkerboard.
Animations start paused. Explicit Play uses a shared elapsed timeline and each
file's decoded delays; frame stepping also links when enabled. Minimize stops
the animation timer; paused/static windows have no animation timer.

F5 reloads; Escape closes; Ctrl+L links views; Ctrl+1–4 selects a pane. Tab reaches
the native buttons, pane headers and interactive content. Arrows, paging,
Home/End and +/- navigate focused content. Compatible reloads restore views,
including retries after repeated missing/corrupt input. Changed dimensions or
preview kinds explain a reset. Pane-header tooltips expose the actual path,
preview stage and limitations. Native focus/message tests do not establish
physical keyboard, screen-reader or mixed-monitor DPI correctness.

## What differences mean

- Bytes: count differing positions, including extra trailing bytes, after an
  independent full-file scan. This differs from decoded-pixel equality.
- Images: exact decoded preview PBGRA channel differences at equal coordinates,
  with separate RGB/alpha counts. Pink marks changed pixels. Decoder scaling
  limits still apply. Different dimensions/frame counts report a fallback;
  pixels are not resampled to suggest alignment. Timing equality is separate.
  Masks display only when the reference and other frame indices align.
- Text: exact LCS alignment of displayed lines, preserving insertion/deletion
  gaps and changed rows. Omitted source content is outside that comparison.
- Folder/ZIP/DBPF: bounded displayed metadata only. DBPF index rows are not CAS,
  meshes, swatches, used CC or dependency-resolved resources. Directory entries
  are not recursively scanned. Media is recognized without starting a player.

No dependency, 3D, perceptual/colorimetric or atomic two-file equality claim is
made. Files may be concurrently edited: before/after length, modification time
and type observations detect observed changes, but cannot prove atomic snapshots
or detect every same-timestamp replacement. Reload after edits.

## Safety and resource limits

Comparison reuses production read policy, bounded readers and the verified LPAC
image decoder. Remote/device paths, reparse links and cloud-recall attributes
are refused. Local source handles share write/delete access and close promptly.
One replacement worker and a 16-event channel bound background work. Replacement
clears old payload immediately; generations discard old results. Close cancels
work and releases retained data. The window/worker is reused, avoiding a thread
per opening. No game scripts, DLLs or embedded project code execute.

Retained preview payload is limited to 64 MiB per pane and 128 MiB shared. Pixel
masks share 8 Mi-pixels (32 MiB). Full byte scans allow 128 MiB per file, read in
64 KiB chunks. Displayed-line alignment allows 4,100,000 u16 cells and 768 KiB
of line text. Over-budget work reports a fallback. These are specific payload
budgets, **not** total process or GPU caps: temporary decoding, GDI surfaces,
metadata/allocator overhead and decoder-child limits are separate. One temporary
alpha surface is freed before the next pane paints. Parent storage/startup hard
deadlines and complete real-file/GPU evidence remain open.

## Repeatable real Windows checks

```powershell
./scripts/test-comparison.ps1 -ReferenceChecks
./scripts/test-comparison.ps1 -Executable target/release/hover-agent.exe -AgentArguments '--stdio'
```

With the pinned public DBPF fixture, both local entry points passed 66 checks;
without it, 64 checks. Evidence is retained under ignored
`target/comparison-evidence/session-*`. Tests exercise actual windows, buttons,
focus, native key messages, tooltips, painter output, linked/different views,
animation timers, small four-pane layouts, missing/corrupt/mixed inputs, reload,
24 rapid replacements, cancellation, rename and unchanged source hashes.
Three one-second idle intervals record parent CPU/memory/handle endpoints, not
long-run idle, GPU/VRAM or concurrent process peaks. RPC timings measure command
responses with uncontrolled caches, not physical presentation or storage cold.

`-ReferenceChecks` independently compares generated decoded pixels with Windows
GDI+ and displayed text edits with external Git. It matched 400 changed pixels,
zero changes for equal pixels/different bytes, premultiplied alpha 128, and one
inserted line. ImageMagick was not installed; no fair cross-tool speed or real
game/specialist fidelity comparison is claimed. No reference code is embedded.

The earlier private prototype passed 97 Windows installer assertions, including
installed GUI/console comparison suites, incoming-payload failure, upgrade,
local rollback and uninstall. A native tooltip test then caught an incorrect
common-controls structure size; the v5-compatible TOOLINFO layout and explicit
Unicode notification contract fixed it. Retain failed logs alongside successes.
Clean packaged artifacts and hosted results require their own subsequent checks.

Primary references: [Shell selected items](https://learn.microsoft.com/en-us/windows/win32/shell/ishellfolderviewdual-selecteditems),
[premultiplied alpha](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-blendfunction),
[tool information](https://learn.microsoft.com/en-us/windows/win32/api/commctrl/ns-commctrl-tttoolinfow),
[Unicode notifications](https://learn.microsoft.com/en-us/windows/win32/controls/wm-notifyformat),
and [Git diff](https://git-scm.com/docs/git-diff).
