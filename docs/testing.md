# Testing Hover on Windows

Use the current source and retained evidence in [current status](current-status.md).
Historical imported checklists are not acceptance proof.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --workspace --locked
./scripts/test-agent.ps1
./scripts/test-agent.ps1 -Executable target/release/hover-agent.exe -AgentArguments '--stdio'
./scripts/test-resident.ps1
./scripts/measure-agent.ps1 -Json
./scripts/test-development-installation.ps1
```

The preview suite controls an owned real Win32 app, reads generated files,
checks file identity/content, bounded errors/cancellation, view continuity,
file locking, native focus/styles and actual client painter exports. It has
56 generated-fixture checks or 61 with the pinned public DBPF metadata fixture.
The resident suite has 17 checks covering normal launch, duplicate launch,
restart, same-user/same-installation lifecycle control, independent folders
and settings preservation. See [agent protocol](agent-control.md).

The native MSI suite separately tests exact packaged installation, Start Menu
launch/icon/AppUserModelID, file hashes, both installed app entry points,
upgrade, failed incoming payload, downgrade guards, explicit local rollback
and uninstall. It uses same-source versioned development fixtures and does
not establish authenticated delivery. See [reproduction and evidence](native-installation.md).
Both Rust 1.95 and 1.99 Windows CI matrices run the native application and
installer workflows. The CPU-limit and window-lifecycle probes marked ignored
in the general Rust suite run explicitly on dedicated hosted Windows jobs.

The resource probe retains command/state timing and CPU/I/O/memory/handle
samples with cache and timing limits stated in its output. Those are actual
native processes, not storage-cold, screen-paint, Explorer or GPU benchmarks.
See [measurement scope](native-performance.md).

Required remaining acceptance includes physical Explorer targeting and safe
activation during typing/rename/IME/dragging, physical taskbar pinning,
screen readers/high contrast, mixed DPI/multiple monitors, actual game/artist
fixtures and specialist/visual comparisons, 3D/dependency fidelity, broader
malformed/permission/GPU/profile/parent-I/O boundaries, and authenticated
update/interruption/power-loss recovery. The CLI suites do not substitute
for those gates. No game milestone or verified release is complete.
