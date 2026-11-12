# Dependency Notes

Last reviewed: 2026-04-27

Command: `cargo tree --workspace --depth 2`

## Approved Dependencies

- `windows-sys 0.61`: used by `hover-win` for hot-path Win32 calls, tray/menu/registry, GDI overlay drawing, and active folder-preview shell icons through `SHGetFileInfoW`. Windows-native API binding, no runtime, no spawned threads, no idle I/O.
- `windows 0.62`: used by `hover-decode` for WIC COM calls and by `hover-win` for Shell/MSAA COM target resolution where type safety lowers risk. Windows-native API binding, no bundled codec pack.
- `gif 0.13`: used by `hover-decode` only after stable hover over a GIF target. Frames are composited into BGRA, capped by frame count and memory, and no decoder runs while Dormant.
- `zip 2`: used by `hover-win` only after stable hover over a ZIP target. Listing is bounded to the first preview entries and no archive scan occurs while Dormant.
- `thiserror 2`: error enums outside hot paths. No idle work.
- `crossbeam-channel 0.5`: reserved for bounded internal event channels. No unbounded queues are allowed.

## Workspace Dependencies Not Currently Used In Code

- `smallvec`, `lru`, `parking_lot`, and `tracing` were listed in the checklist as approved candidates, but the implementation currently avoids them where the standard library is sufficient.

## Rejected For Core Resident App

- Async runtimes: Tokio, async-std.
- HTTP stacks: reqwest, hyper, curl, OpenSSL.
- Browser/UI runtimes: Electron, Tauri, WebView2, React, Vue, Svelte.
- Resident video process dependency: FFmpeg/ffplay must never run while idle. After explicit user request, `hover-win` may launch `ffplay` only for an active hovered video file and must kill it when the preview hides. Runtime discovery checks `HOVER_FFPLAY_PATH`, `PATH`, the app folder, and common FFmpeg install locations.
- Always-bundled codec packs.

## Local Runtime Tooling

- FFmpeg Gyan build was installed with `winget install --id Gyan.FFmpeg -e --accept-source-agreements --accept-package-agreements` on 2026-04-27 so local video hover previews can use `ffplay.exe`.