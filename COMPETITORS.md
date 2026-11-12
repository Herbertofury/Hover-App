# Competing apps and specialist references

## What users should gain
Hover should reveal the right information without opening a heavyweight application, blocking a file operation, stealing focus or consuming resources while idle. Full game-format support and breathtaking rendering are goals requiring file-specific evidence, not blanket claims.

## Direct alternatives
| Reference | Verified documentation | Comparison focus |
|---|---|---|
| [PowerToys Peek](https://learn.microsoft.com/en-us/windows/powertoys/peek) | Documents Space activation, selected-file navigation, keyboard controls and read-only SQLite preview. | Activation-to-useful-content, focus behavior, keyboard navigation and explicit truncation. Navigation among selected files is not simultaneous comparison. |
| [QuickLook](https://github.com/QL-Win/QuickLook) | Official project brings Quick Look-style preview to Windows. | Supported viewer behavior, extension compatibility, large-file isolation and plugin failure recovery. |
| [XYplorer Hover Box](https://www.xyplorer.com/release_24.40.php) | Historical 24.40 release, April 2023, documents keyboard movement between hover previews. | Mouse-to-keyboard continuity, traversal and discoverability without focus theft. This is not a claim about its latest release. |

## Specialists are fidelity references, not all direct competitors
[Sims 4 Tray Importer](https://luniversims.fr/files/file/73-sims-4-tray-importer/) is a reference for Tray information, images and CC workflows. Its published changelog includes image reading, CC loading and image-cache fixes. It does not establish a complete live 3D household or lot renderer. Keep specialist research linked through the [game support index](https://github.com/Herbertofury/Hover-App/wiki/Game-support-index) and [format comparison](https://github.com/Herbertofury/Hover-App/wiki/Format-research-and-specialist-comparison).

## Acceptance experiments
- Compare multi-selected files side by side, with optional synchronized zoom/cameras and clear per-file identity.
- Keep previews nonlocking: rename, move, replace and delete while a preview exists; safely recover from disappearance or replacement.
- Preserve compatible zoom, pan and scroll after reload; pinned previews, shelf and copy actions must not leak stale content.
- Show immediate bounded metadata while expensive decoding proceeds asynchronously. Measure cold preview, not just warm-cache render time.
- Test malformed archives, decompression bombs, missing dependencies, remote placeholders and cancelled work under explicit resource budgets.
- Verify every game adapter against real authorized examples and specialist output: embedded image, parsed metadata, reconstructed geometry and faithful rendering are separate completeness levels.
- Keep autoplay off and require user intent for media. Avoid recursive whole-drive indexing as a shortcut to perceived speed.

These are comparison targets. The README and current-status evidence remain authoritative for implemented behavior.

## Evidence rules and recurring review
Last source review: **2026-11-12**. This is a living comparison, not a claim that our app beats these tools. Source-described behavior is not hands-on verification. No comparative runtime measurements were performed in this pass.

For each daily review, inspect official release notes, documentation, repositories and relevant issue resolutions. Record publication date separately from review date, version/commit, affected workflow, source link, and what actually changed. An old release page remains historical evidence, not today's release. If unchanged, retain the previous evidence and say no material change. If blocked, mark unavailable; never infer feature absence from a failed fetch.

Maintain each entry as: **reference | workflow | source/version/date | observed strength | reported grievance and resolution | our measured baseline | hypothesis | acceptance evidence | decision/issue**. A complaint is a report, not proof of a defect or its prevalence. Check whether it was fixed before proposing work. Preserve failures and last-good evidence.

Before adopting an idea, compare representative real inputs on the same machine and record cold/warm p50/p95 latency, memory, CPU, correctness and accessibility. Preserve feature parity, crash/restart recovery and durable data. A faster microbenchmark does not establish a faster product. Reuse compatible maintained upstream code only after license/dependency review. New feature proposals require owner review; this research does not silently authorize implementation.

Daily upkeep is a requested cadence, not a claim that a scheduler or uninterrupted worker is active. Add dated results only after an actual review.
