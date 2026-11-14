---
name: hover-native-preview-qa
description: Repair and qualify Hover-App native previews, linked comparison and Windows installation with real lifecycle evidence, bounded file handling and optional feature independence.
---

# Hover native preview QA

Read repository AGENTS.md, Cargo.toml and docs/current-status.md first. The product remains a native Windows Rust app; a browser mock or source-only build does not establish delivery.

## Locate the affected ownership boundary

Trace core request/generation handling, format reading, decoding, rendering, settings and Windows host separately. Preserve the lazy worker with bounded queued/result state; never introduce one thread or continuous polling per hover. Preview, comparison, browser host and external media are separable capabilities. Inspect dependencies before a change that would make an optional component mandatory.

Read docs/pinned-preview.md for Pin/Reload/Close behavior and docs/native-comparison.md for linked panes. Test same-file reload state restoration only when kinds/dimensions remain compatible; resets should explain themselves. Preserve separate displayed-text, decoded-pixel and full-file-byte comparisons rather than presenting different evidence as equivalent.

## Native interruption matrix

Use the actual Windows app or hover-agent private stdio driver, following the repository README and scripts/test-agent.ps1. Record exact executable/source identity. Exercise target A to B while A loads; Escape/Close during decode; missing/corrupt input then retry; Pin/Unpin across app switches; reload after scroll/zoom changes; and two-to-four-file comparison with a slow/failing pane. Old work must not repaint or reopen a dismissed/new target. Independent panes must remain useful when one cannot decode.

Check input bounds and explicit fallback states for folder/ZIP/DBPF/text/image files. Do not extract archives or change source files just to preview them. Automatic reads must retain the project's remote/device/reparse/cloud-recall restrictions. Decoder isolation or effective LPAC authorization failures are not reasons to silently fall back to unsafe in-process decoding.

Media starts on user action. ffplay/mpv/VLC are optional external programs, not bundled dependencies or evidence that all media formats work. Keep fixture permissions and third-party notices intact.

## Verification routes

Read the current .github/workflows Windows matrix for pinned toolchains, formatting, Clippy and test commands. Build with cargo build --release --workspace --locked and follow scripts/verify.ps1. Use the matching scripts for affected behavior:
- scripts/test-agent.ps1 for native preview automation;
- scripts/test-resident.ps1 for resident lifecycle/isolation;
- scripts/test-comparison.ps1 -ReferenceChecks for real comparison and independent controls;
- scripts/test-development-installation.ps1 or scripts/test-installation.ps1 for the documented development/package path;
- scripts/check-public-metadata.ps1 before public commits; scripts/collect-licenses.ps1 for dependency/license changes.

Inspect the scripts' parameters before running them. Installer tests can change the installed app: use the intended disposable or authorized environment, preserve settings and rollback, and do not treat a source-check task as permission to replace someone's installation. Do not automatically change security settings to get decoder tests green.

## Acceptance record

Separate fixture automation, real app painter output, actual Explorer activation, physical keyboard/accessibility, mixed DPI/monitors, packaged install/upgrade/rollback, and authenticated network updates. Only mark exercised layers complete. Benchmark equivalent file sets with latency, idle activity and memory; bounded fallback is not full-format support. Retain raw failures, package/source identity and remaining gates. A verified development build is not a verified release.
