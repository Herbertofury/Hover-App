# Current delivery evidence

Checkpoint: 2026-11-12. Development source; no verified release.

The original desktop workspace was not a Git checkout. Remote main contained
only the scaffold README and no open PRs. It was cloned separately, then source,
manifests and historical docs were copied without binaries, reference checkouts
or private assets. The original desktop project remains unchanged. No other
active chat was doing Hover work. All 17 existing milestones and 21 wiki pages
were inspected; their scopes remain required and open.

## Implemented foundation

- Preserved native Win32 tray/overlay, WIC, GIF, text, folder, ZIP, DBPF metadata
  and optional external media paths. Media now requires a click to play.
- One lazy sleeping worker, one newest queued request and one result slot
  replace per-hover thread spawning. Completion posts a Win32 event. Stale
  generations are discarded; old content clears when the target changes.
- Loading/error panels identify the file. Escape/click dismissal cancels pending
  work and suppresses reopening until the target changes.
- Directory enumeration stops after 2,000 rows plus one indication of more;
  remaining entries are not scanned to invent a total count.
- ZIP directory allocation is guarded at 20,000 entries / 8 MiB, with 2,000
  displayed rows and a checked ZIP64 footer path. This is bounded indexing,
  not streaming every entry. Prefixed/multi-disk/over-budget files report an
  explicit fallback. No extraction occurs.
- Local readers share read/write/delete access. WIC and GIF decode bounded
  snapshots after closing the source handle. Automatic reads refuse remote and
  device paths, reparse links and cloud-recall attributes.
- Unicode Explorer URLs decode as UTF-8 bytes; malformed escapes/NUL are
  rejected. COM initialization is balanced.
- ZIP upgraded from 2.4.2 to 9.5.1 after four RustSec advisories. Full reference
  and Cargo license notices are retained. Advisory scans do not prove safety.

## Current evidence

The imported baseline passed 22 tests on Windows with Rust 1.99.0. Added native
tests verify exact WIC BMP pixels, unchanged input bytes, second-process
rename/replace/delete with a held shared reader, byte limits, cancellation,
corrupt input, Unicode paths, ZIP guards and a 10,000-request replacement burst.
These are generated synthetic fixtures, not real game assets or end-to-end
Explorer performance. Exact verification output is recorded separately.

Supported Windows UI automation is blocked at startup: `failed to write kernel
assets: The system cannot find the path specified. (os error 3)`. Both node and
CUA failed before app input. No hover/visual/accessibility/install test passed
through that runtime. Unsupported UI injection is not a substitute.

## Required remaining gates

- Actual shell targeting and safe activation during typing, rename, IME, drag
  and modal flows; pinning, keyboard, accessibility/high contrast, DPI/monitors,
  reload continuity, shelf, copying and synchronized multi-file comparison.
- Hard decoder isolation and time/memory/process limits. Cooperative worker
  cancellation cannot terminate a stuck native codec. Native codec allocation,
  concurrent changes, external media security and parser fuzz/stress coverage
  remain incomplete. Existing settings/checklists do not prove these features.
- Each game milestone with pinned licensed specialists, versioned permitted
  fixtures and retained differential/visual failures. DBPF is unverified partial
  metadata, not CC resolution, image extraction or 3D. Bethesda, Minecraft,
  Hytale, Unreal, Unity, Cyberpunk, Witcher, Terraria, Stardew and BG3 adapters
  remain unimplemented. Engine and community research queues remain in scope.
- Artist formats, actual 2D/3D/animation/material/texture inspection and faithful
  dependency resolution. Embedded images, interactive geometry and evaluated
  scenes must be separate support states.
- Real-file cold/warm p50/p95/p99 first useful preview/3D frame, input/frame
  stalls, idle CPU/I/O, RAM/VRAM, cache and file-handle evidence on Windows.
- Stable-path installation, taskbar launch, source-authenticated atomic updates,
  restart, interruption recovery and rollback against exact packaged artifacts.
  The WiX/MSIX files are skeletons. No release or milestone is complete.

The full user objective and existing [issues and milestones](https://github.com/Herbertofury/Hover-App/milestones)
remain authoritative; this checkpoint does not narrow them. Other imported
docs and checked boxes describe historical April work and are not current
completion proof. This checkpoint supersedes their status claims.
