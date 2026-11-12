# Hover CLI and agent control

`hover-agent.exe --stdio` launches and owns a real native Hover session. It
uses the production App, Win32 windows/message loop, preview worker, local-read
policy, decoders, view model and painter. No Computer Use runtime is required.
The normal notification-area entry point remains `hover.exe`.
That same GUI executable also accepts `--agent-stdio` when launched with
redirected stdin/stdout. The end-to-end script tests this actual `hover.exe`
release binary; `hover-agent.exe` is the convenient console entry point.

The interface does not attach to another Hover process, listen on a network
port, run arbitrary programs, evaluate embedded scripts or inject input into
other applications. Each session uses ephemeral default settings and refuses
startup-registry mutation. EOF or `quit` closes this session's windows, hooks
and resident process. This is a development interface, not release certification.

## Run and test

```powershell
cargo build --release --workspace --locked
./target/release/hover-agent.exe --help
./scripts/test-agent.ps1
```

The test generates its own on-disk files and keeps evidence in
`target/agent-evidence`. It starts real Windows processes and windows, checking
identity, decode output, native styles/focus, reload continuity, rename and byte
preservation, cancellation, malformed requests, no autoplay, minimize/restore,
render capture and owned shutdown. The local release build passed 45 generated
fixture assertions, or 49 with the optional pinned DBPF fixture below.
CI invokes the same script after each toolchain's build; check its actual result
before claiming hosted success. The test does not send physical input or operate
Explorer.

Use `-DbpfFixture <path>` with the reviewed LlamaLogic TestPackage.package from
[DBPF evidence](dbpf-evidence.md) to add native metadata/capture/hash checks.
The script requires its exact SHA-256. CI explicitly downloads this single file
at the pinned upstream revision and verifies it; Hover never downloads fixtures
on hover. This checks 20 package index resources, not real-game rendering or a
known Sims game/build. The earlier 40-check CI run passed both toolchains;
expanded 49-check hosted results are pending.

Example batch (replace the local input/output paths):

```powershell
@(
  '{"id":1,"command":"capabilities"}',
  '{"id":2,"command":"open","path":"C:\\Fixtures\\image.png","pin":true}',
  '{"id":3,"command":"wait","timeout_ms":10000}',
  '{"id":4,"command":"zoom","mode":"native"}',
  '{"id":5,"command":"capture","path":"C:\\Fixtures\\preview.bmp"}',
  '{"id":6,"command":"quit"}'
) | ./target/release/hover-agent.exe --stdio
```

## Protocol version 1

Send one UTF-8 JSON object per line, at most 16 KiB including newline. Optional
`id` is echoed unchanged. Replies contain `protocol: 1`, `id`, `ok`, and either
`result` or `error`. Stdout contains JSON replies; diagnostics use stderr.
Unknown fields/commands and invalid ranges fail explicitly. Oversized requests
stop this session with exit code 2. EOF/accepted quit normally exit 0.

Commands are serialized onto the actual UI thread through an eight-slot queue
with a ten-second reply timeout. Decode is asynchronous: `open` and `reload`
acknowledge pending state; `wait` obtains a settled snapshot. A settled snapshot
can have `phase: error`; inspect that field as well as RPC `ok`.

- `capabilities`, `support`, `settings`, `status`: discover commands, retrieve
  the bundled game-support matrix, inspect effective ephemeral settings, or
  inspect native/application state.
- `open`: absolute `path`; `pin` defaults true; `activate` defaults false.
  Local/cloud/link read policy still applies. `pin` sets `value` (default true),
  optionally `activate`. Explicit activation can focus Hover.
- `reload`, `close`, `quit`: production reload, preview dismissal/cancellation,
  or owned resident shutdown. Close leaves the session running.
- `wait`: `timeout_ms` 1–30000, default 10000. Status polling every 20 ms happens
  in the controller; the UI thread keeps servicing completion messages.
- `inspect`: decoded text/list pages with `offset` 0–20000 and `limit` 1–128.
  Long list names/details are explicitly bounded. Images return decoded
  dimensions/frame information and an FNV-1a pixel checksum for regression
  comparison. This checksum is not a cryptographic integrity hash.
- `zoom`: `mode` is `fit`, `fit_width`, `fit_height` or `native`. `pan` accepts
  finite `dx`/`dy` within +/-1000000; the production model clamps the result.
  `scroll` sets row `offset`, clamped to actual content and viewport.
- `resize`: outer `width` 96–4096 and `height` 96–2160; requires an open,
  restored preview. `minimize` and `restore` operate on a pinned native window.
- `focus`: `target` is `content`, `next` or `previous`, using production native
  focus traversal. This is not physical keyboard input.
- `play`: explicit playback of prepared media through an installed backend.
  Recognition/paused state does not prove valid audio/video decoding.
- `activation`: `automatic` defaults false. Controlled mode prevents pointer
  movement/app switches from replacing a test preview. True enables production
  foreground/cursor/shell targeting, whose real Explorer and safe typing,
  rename, drag and IME behavior still require verification.
- `capture`: absolute new `.bmp` `path`. The real painter and its own native
  controls render a client-area bitmap up to 4096 x 2160. Existing files,
  remote/linked output directories and hidden/minimized previews are refused.
  This is an app render export, not a desktop/compositor screenshot; external
  video, GPU scenes and other apps are not captured.

## Evidence boundaries

Generated image captures are checked against known quadrant colors and were
visually inspected locally. Evidence retains responses, executable SHA-256,
OS/architecture, assertions and raw command round-trip durations. Timings
include protocol/UI scheduling and wait polling; they are not Explorer hover
latency, storage-cold benchmarks or 3D frame times. Fixtures were just generated;
the OS cache was not flushed. Actual game fidelity, accessibility, mixed DPI,
long-run resource budgets and exact install/update/rollback gates remain open.

Responses can contain caller-requested local paths and content. Evidence stays
local and is never automatically uploaded. Review/redact paths and private
content before publishing any output.
