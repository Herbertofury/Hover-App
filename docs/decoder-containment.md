# Native image decoder containment

Current source adds [LPAC permission isolation and its native evidence](decoder-permissions.md)
to the original PR #42 resource worker described here. Retained PR #42 results
describe that earlier revision; they do not assert its old worker was sandboxed.

WIC images and animated GIFs in the actual Hover Windows app now decode in a
short-lived copy of its own executable. The preview worker reads one authorized
local snapshot, closes the source file, then sends bytes over private anonymous
pipes. It sends no source filename or file handle. The existing WIC/GIF decoders
and pixel representation remain in use. There is no in-process fallback when
containment cannot be established.

Before sending any bytes, the parent assigns the waiting child to a Windows
job with these enforced limits:

- One active process; no breakaway and termination when the job handle closes.
- 2 GiB process commit limit, 5 seconds of user CPU time, and a 50% machine CPU
  rate ceiling. CPU-time enforcement is periodically checked by Windows.
- An 8-second supervised transaction deadline. Cancellation or timeout stops
  the child independently of cooperation from the codec.

Cleanup allows up to one second for process exit and one second to drain/cancel
the owned synchronous pipe thread. If cleanup fails, further image decoding is
disabled until restart, rather than accumulating abandoned workers. The parent
accepts only bounded, complete output from a successful child: requested image
dimensions, at most 1,000 animation frames and 256 MiB total pixels. Truncated,
oversized and unexpected trailing output fail explicitly. Input remains capped
at 512 MiB. CLI `status.decoder_process` exposes the actual active PID, last
outcome, job accounting and configured limits; unavailable accounting is explicit.

PR #42 provided crash/resource containment alone. Its child still used the
user's normal token/environment; the job did not restrict filesystem, network,
registry, broker or GPU access. Current source replaces that launch with LPAC;
see the permission document above for actual tests and remaining boundaries.
Source snapshot I/O and parent startup APIs still need hard stall boundaries.
External media is separate. These remain release gates.
Host job restrictions are preserved; Hover never requests breakaway. UI limits
are not set because Windows job nesting does not support them. See Microsoft's
[job objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects),
[nested jobs](https://learn.microsoft.com/en-us/windows/win32/procthread/nested-jobs)
and [CPU-time limit semantics](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_basic_limit_information).

## Verification and cost

The local release executable passed 55 agent assertions with the pinned public
DBPF fixture; see [retained local results](evidence/decoder-agent-local-2026-11-12.json).
These include generated BMP colors, generated two-frame GIF
decoding, a distinct decoder PID, real job accounting, no surviving child,
source hashes, rename, reload, cancellation and the existing native app checks.
Six additional source tests cover actual stalled children, cancellation, abrupt
exit and recovery, denied native allocation, ordinary descendant creation,
non-spinning supervision and hostile/truncated output packets. Fault helpers
exist only in the test executable. A dedicated hosted Windows test additionally
checked actual CPU-time termination on both Rust 1.95 and 1.99. Both complete
jobs passed the 55-check real-app suite at source `13f309f0250467d5c110c31e16f8289f2cea2114`
in [Windows run 43334578860](https://github.com/Herbertofury/Hover-App/actions/runs/43334578860).
See [retained CI summary](evidence/decoder-ci-2026-11-12.json) and
[native CPU/app log](evidence/decoder-native-ci-2026-11-12.txt). Log trailing
whitespace is removed; the linked run retains the original. PR #42 was published
to protected main by fast-forwarding to that exact tested commit. The later
reload-recovery suite adds five checks; its local evidence is documented in
[pinned-preview evidence](pinned-preview.md).

The [raw component samples](evidence/decoder-benchmark-2026-11-12.json) compare
the same source-read/decoder path in-process and through the isolated process.
On the generated BMP, p50 was 6.73 ms in-process and 40.82 ms isolated; on the
two-frame GIF, 1.07 ms and 30.17 ms. Geometry, pixels and GIF delays matched
across both sets. Isolation costs time on these small files; this change makes
no speed improvement claim. Each mode has 16 samples, run in sequence with
uncontrolled OS cache state. These are synthetic component measurements, not
Explorer hover, storage-cold, UI frame, real-game or 3D results.

Reproduce against a caller-authorized absolute local fixture path:

```powershell
cargo run --release -p hover-win --example decoder_bench -- C:\Fixtures\quadrants.bmp 16
cargo run -p hover-decode --example gif_fixture
./scripts/test-agent.ps1 -DbpfFixture C:\Fixtures\TestPackage.package
```

The GIF example prints the generated fixture bytes; the agent test creates the
same two-frame file itself. No private game assets or new third-party decoder
code were imported. Packaged installation, updates, rollback, Explorer input,
accessibility and all game-specific acceptance gates remain open.
