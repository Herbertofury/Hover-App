# Repository instructions

Read `.agents/foundry/AGENTS-adapter.md` for the pinned shared engineering core.
Keep this file and nested instructions project-owned: setup, commands, constraints and acceptance belong here.

## Hover-local contract

- This public workspace is Hover-App. Read docs/current-status.md and docs/native-comparison.md for evidence and remaining gates. Preserve the original desktop source and private prototypes; never copy their account details or private metadata into this repository.
- Maintain crate responsibilities: core, formats, settings, decode, render, Windows host and optional browser host. Browser integration, comparison and media helpers must remain optional; disabling any must not disable ordinary native previews.
- Preserve lazy bounded work, stale-generation rejection and cancellation. Close/Escape must not reopen a dismissed preview from an old request. Keep source files unmodified and closed promptly, preserve reader sharing, safe input limits and explicit unsupported/fallback states.
- Use .agents/skills/hover-native-preview-qa/SKILL.md for native preview, comparison, decoder and installer acceptance. Linux compile/static evidence is not Windows Explorer/WebView/native acceptance.
- Check Cargo.toml and .github/workflows for exact supported Rust/toolchain commands. Keep custom settings and installed-data rollback intact. Do not claim release qualification from development MSI or fixture checks.
