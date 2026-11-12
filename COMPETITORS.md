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

## Foundations to build on
| Foundation | Specific reuse or study | License evidence and boundary | Improvement acceptance |
|---|---|---|---|
| Existing Hover Rust source | Extend current shell activation, bounded decoding and pinned-preview behavior rather than replacing working paths for novelty. | Retain project notices and inspect the license of every imported component. | No idle-resource regression; rename/delete remains possible while previewing. |
| [CUE4Parse](https://github.com/FabianFG/CUE4Parse) | Unreal parsing/reference candidate for package/container and typed-resource interpretation; inspect actual supported versions before selecting an adapter. | [LICENSE](https://github.com/FabianFG/CUE4Parse/blob/master/LICENSE) reviewed: Apache-2.0. This does not license game assets or all dependencies. Non-Rust upstream is a reference/adapter candidate, not an already-native Rust implementation. | Differential tests against authorized supported samples; unsupported/encrypted content must be explicit, without bypassing access restrictions. |
| [UnityPy](https://github.com/K0lb3/UnityPy) | Unity parsing reference and differential-test candidate; evaluate individual object/resource support instead of claiming every Unity game works. | [LICENSE](https://github.com/K0lb3/UnityPy/blob/master/LICENSE) reviewed: MIT. Python upstream is not proof of a Rust integration or permission to redistribute games. | Compare metadata, textures and dependencies against known samples; isolate corrupt inputs and bound decode resources. |
| [wgpu](https://github.com/gfx-rs/wgpu/blob/trunk/Cargo.toml) | Evaluate reusable GPU rendering infrastructure for interactive meshes/material previews instead of inventing a graphics backend. | Reviewed workspace manifest declares MIT OR Apache-2.0; audit actual chosen dependency versions. | Measure cold GPU initialization, sustained preview latency, device-loss recovery and idle power. No migration justified by a microbenchmark alone. |

## User-project interoperability opportunities
- **External CAS:** investigate a shared read-only Sims resource/dependency model and preview request contract. Its existing [native source notices](https://github.com/Herbertofury/External-CAS/blob/main/native/THIRD_PARTY_NOTICES.md) identify GPL-derived components. Hover's MIT label does not permit silently copying these into an MIT-only distribution; resolve compatible licensing before reuse. Separate processes are not an automatic legal exemption.
- **Mod managers and game-library apps:** accept an explicitly selected local file/resource for preview and return useful metadata/completeness without taking ownership of installs or profiles.
- **Engineering Atlas:** reuse permissible fixtures and test methodology, not private game assets or unqualified performance conclusions.

## Reuse qualification and interoperability contract
**Status: design/reuse candidates, not integrated or benchmark-qualified by this review.** Before adopting code, pin the exact revision; inspect file-level license headers, dependency licenses, notices, assets and redistribution obligations. A repository badge is not a full license audit. Preserve attribution and corresponding-source obligations where applicable. A language port does not erase upstream obligations.

Cross-project integration should use a versioned, documented contract with stable resource IDs, source hashes, schema version, completeness/access state, cancellation and explicit errors. Read-only preview and metadata exchange is the default. Never copy credentials, grant filesystem-wide access or let a preview trigger installation, game mutation or background downloads. Mutations require the owning application's authorization, revision check, journal and rollback. Negotiate capabilities rather than assuming every consumer supports every format. Test disconnected peers, stale revisions, malformed messages and incompatible schema upgrades. These contracts are proposed; no working cross-app link is claimed here.

No private repository contents, personal catalogs, source assets or credentials belong in public examples. Use redistributable fixtures and synthetic IDs; retain real-input evidence privately.
