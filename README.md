# Hover

Native Rust Windows previews for players, modders and artists.

The app is called **Hover**. This public repository is named **Hover-App**.

## Documentation

See the [Hover wiki](https://github.com/Herbertofury/Hover-App/wiki) for goals, the delivery checklist, and verification standards.

## Status

**In development. No verified installer or release is available.** The existing desktop source has been recovered here, preserving its development features. See [current evidence and remaining gates](docs/current-status.md).

The goal is responsive desktop previews with minimal idle CPU, memory, and disk activity, including useful mod-file metadata. Performance and file-format support must be demonstrated with repeatable tests, not assumed.

## Build and verify

Use Windows, Visual Studio C++ build tools/Windows SDK and Rust (declared minimum 1.95; locally verified with 1.99.0).

```powershell
cargo build --release --workspace --locked
./target/release/hover.exe
./scripts/verify.ps1
cargo audit
```

Hover starts in the notification area. Use its tray menu for options/exit.
Explorer hover is the intended activation path. Image zoom, scrolling panels,
GIF, folder/ZIP lists, bounded text, DBPF metadata and optional external media
paths exist in source. Media waits for a click. Separately installed ffplay,
mpv or VLC are optional and not bundled. Full game support, 3D rendering,
dependency resolution and comparisons remain unfinished.

The development preview now has Pin, Reload and Close controls. Pin opens an
activated, resizable inspection window that survives app switches. F5/Ctrl+R
reloads; Ctrl+P toggles pin; Escape closes the preview. Image zoom/pan and
text/list scroll restore after compatible same-file reloads. See
[pinned-preview behavior and test limits](docs/pinned-preview.md); actual
Explorer, physical-keyboard and accessibility verification remains pending.

Native automated tests use generated fixtures; they do not prove Explorer,
visual, accessibility, real-game fidelity or installation behavior. Every game
milestone remains an acceptance requirement, not a supported-games list.

Source is MIT licensed. Retain [third-party notices](THIRD-PARTY-NOTICES.md)
with distributions. The original desktop project is preserved separately;
build output, reference checkouts, private saves and game/creator assets are
excluded. Publish only after exact packaged install, hover, update, restart
and rollback gates pass with representative Windows evidence.
