# Performance Budget

Hover treats **zero idle work** as the product requirement.

## CI Report Fields

Every performance report must include:

```text
ZERO_IDLE_STATUS=PASS|DEFECT|EXCEPTION_DOCUMENTED
state=Dormant|ForegroundEligible|HoverCandidate|DecodePending|PreviewVisible|PreviewLocked
active_timers=0
decode_jobs=0
render_presents=0
wheel_hooks=0
raw_input_captures=0
disk_reads_after_settle=0
network_connections=0
worker_threads_awake=0
```

## Dormant Gates

- CPU: 0.0 percent target.
- GPU: 0.0 percent target.
- Disk: 0 reads after startup settles.
- Network: 0 connections and 0 bytes.
- Timers: 0 active timers.
- Hooks: 0 global mouse hooks.
- Decode work: 0 decode workers awake.
- Render work: 0 render passes.

Any non-zero Dormant gate is a defect unless documented in `docs/zero-resource-exceptions.md`.

## Current Measurement Status

- [blocked] Final Task Manager, Resource Monitor, WPR/WPA, Process Explorer, and GPUView checks require running the finished GUI app interactively on Windows 11.
- `cargo test --workspace` currently validates internal zero-counter behavior but is not a runtime resource measurement.