# Repeatable Windows app measurements

`scripts/measure-agent.ps1` starts and owns real `hover.exe --agent-stdio`
processes. It generates an 800 x 600 BMP, verifies every settled preview's file
identity/dimensions/decoder completion, checks real pan changes and successful
shutdown, and verifies unchanged fixture bytes. No Computer Use runtime, input
injection, game assets, network service or normal user settings are required.
All generated files and raw measurements are retained locally.

```powershell
cargo build --release --workspace --locked
./scripts/measure-agent.ps1
./scripts/measure-agent.ps1 -Sessions 3 -WarmSamples 16 -IdleMilliseconds 3000 -Json
```

`-Json` emits parseable JSON as well as saving it under a fresh
`target/performance-evidence/session-*/measurements.json`. It contains the
executable SHA-256, generated fixture hash, OS/build/CPU, individual timings,
actual decoder job accounting and resource counter endpoints. It contains no
input/output paths or account/machine names. The script terminates only its own
processes and never deletes files. CI exercises a smaller one-session/two-reload
probe on each Windows toolchain, with no timing threshold on shared runners.

## What is measured

- Process startup ends when the real native UI thread answers `capabilities`.
- First `open` and subsequent same-process `reload` record the acknowledgment
  containing the correct file identity/phase, then the settled preview response.
  The WIC child is new for every decode, including warm-process reloads.
- Pan round trips verify the requested state change. They measure command/UI
  response, not screen/compositor frame presentation.
- Dormant, pinned static and minimized intervals send no protocol/input requests
  while Windows process counters advance. Parent CPU, private bytes, working set,
  handle count and I/O deltas are separate from child job accounting. CPU percent
  is labelled both per one logical processor and normalized across the machine.

The wall clock includes JSON/pipe work, scheduling and `wait`'s 20 ms polling.
Acknowledged state and published pixels do not prove when a physical screen
painted. New-process startup does not mean storage-cold: the generated file was
just written and OS caches are uncontrolled. Endpoint memory is not a concurrent
peak; lifetime peak working set is labelled separately. Process I/O includes
pipes and cached operations, not physical disk activity. See Microsoft's
[GetProcessIoCounters](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getprocessiocounters)
and [IO_COUNTERS](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-io_counters).

## Retained local run

The [raw run](evidence/native-performance-2026-11-12.json) has 3 new-process
first opens, 48 warm-process reloads, 16 verified pan responses and three
approximately 3-second idle intervals on one shared Windows machine. The binary
is identified by its hash; it is the locally verified reload-recovery build.

Warm-process settled responses had p50 59.78 ms and p95 90.65 ms; acknowledgments
had p50 6.30 ms and p95 16.33 ms. Percentiles use nearest rank. Three first opens
are too few for a useful tail claim; their median settled response was 99.59 ms.
This run is not a comparison against another build or a guaranteed latency.

The idle intervals recorded no CPU-time ticks or process I/O deltas, stable
parent memory and unchanged handle counts. CPU counter resolution and these
short intervals preclude a literal zero-CPU claim. The pinned-image working set
was about 12.63 MiB and private bytes about 4.21 MiB at the endpoints; these omit
the already-exited decoder and are not total-system, VRAM or peak figures.
Every settled sample in each process had 179 parent handles. This small run
does not prove absence of long-run leaks.

Real-file/game fidelity, actual Explorer targeting, storage-cold p50/p95/p99,
physical input/frame latency, large libraries, animation/media/3D resources,
GPU/VRAM, accessibility/DPI and packaged install/update/rollback remain open.
The [decoder component comparison](decoder-containment.md) has a different scope
and must not be presented as the same measurement.
