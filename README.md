# Hover

Native Rust Windows previews for players, modders and artists.

The app is called **Hover**. This public repository is named **Hover-App**.

## Documentation

See the [Hover wiki](https://github.com/Herbertofury/Hover-App/wiki) for goals, the delivery checklist, and verification standards.

## Status

**In development; no verified release.** Native development MSI packages now have Windows install, preview, restart, upgrade, failure and local rollback tests. The existing desktop source has been preserved. See [current evidence and remaining gates](docs/current-status.md).

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
dependency resolution and game/3D comparisons remain unfinished.

Native two-to-four-file comparison now supports progressive independent panes,
linked image zoom/pan, paused animation with linked controls, decoded-pixel masks,
displayed-line alignment and separate file-byte differences. Run
`./scripts/test-comparison.ps1 -ReferenceChecks` to test the actual Windows app;
see [comparison behavior, budgets and evidence limits](docs/native-comparison.md).

The development preview now has Pin, Reload and Close controls. Pin opens an
activated, resizable inspection window that survives app switches. F5/Ctrl+R
reloads; Ctrl+P toggles pin; Escape closes the preview. Image zoom/pan and
text/list scroll restore after compatible same-file reloads. See
[pinned-preview behavior and test limits](docs/pinned-preview.md); actual
Explorer, physical-keyboard and accessibility verification remains pending.

For repeatable tests without Computer Use, build `hover-agent.exe --stdio` and
use its private JSON command interface. It owns a real native Hover session and
can inspect/control previews and export the actual painter's output. Run
`./scripts/test-agent.ps1`; see the [agent protocol and evidence limits](docs/agent-control.md).
Use `./scripts/measure-agent.ps1 -Json` for repeatable startup, preview/state
response and idle CPU/I/O/memory measurements; [scope and raw evidence](docs/native-performance.md).

`hover-agent.exe --version-json`, `--resident-status`, `--show` and `--shutdown`
inspect and control the normal tray app in the same installation under the
current user. Normal launches reuse that resident; independent installations
and private stdio sessions remain separate. Run `./scripts/test-resident.ps1`.
For real Windows installation tests, use `./scripts/test-development-installation.ps1`;
see [development MSI workflow and evidence limits](docs/native-installation.md).

WIC/GIF decoding uses a separate native process with Windows job resource limits
and bounded byte pipes, now inside a token-verified Windows LPAC. See
[permission isolation and actual denial tests](docs/decoder-permissions.md) and
[decoder containment and measured cost](docs/decoder-containment.md) for evidence
and the remaining security boundaries.

Native automated tests use generated fixtures; they do not prove Explorer,
visual, accessibility or real-game fidelity. Separate native MSI tests cover
development installation behavior; release qualification remains open. Every game
milestone remains an acceptance requirement, not a supported-games list.

Source is MIT licensed. Retain [third-party notices](THIRD-PARTY-NOTICES.md)
with distributions. The original desktop project is preserved separately;
build output, reference checkouts, private saves and game/creator assets are
excluded. Publish only after exact packaged install, hover, update, restart
and rollback gates pass with representative Windows evidence.
