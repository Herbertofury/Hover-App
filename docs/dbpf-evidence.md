# DBPF 2.1 index metadata checkpoint

This is package-index inspection only. It does not resolve CC, render meshes,
extract thumbnails, identify required packs or reconstruct a Sim/lot.

The old reader always consumed a committed field, even for entries without an
extended compression record. That misaligned subsequent uncompressed entries.
It also selected the larger of the legacy and 64-bit index pointers, instead
of using the nonzero 64-bit pointer first.

The bounded Rust reader in `hover-formats` corrects both cases. It validates the
96-byte header and exact 2.1 version; checks index byte range/count/flags and
resource ranges; reads at most 4 MiB of index bytes; checks cancellation per
read chunk and entry; retains at most 5,000 entries. A capped prefix is labelled
partial, with a declared total. Uninspected entries are not claimed validated.
No resource data is decoded or executed. Package-category labels use parsed
resource type identifiers; gender/age and merged-package claims are no longer
inferred from filenames.

## Specialist reference and comparison

Format reference: [LlamaLogic source revision 4a3d576f0afb776aba783db561760afb5b5de8b7](https://github.com/Llama-Logic/LlamaLogic/blob/4a3d576f0afb776aba783db561760afb5b5de8b7/LlamaLogic.Packages/DataBasePackedFile.cs),
particularly ParseHeader/ParseIndex. Its MIT notice is retained separately.

Actual comparator: LlamaLogic.Packages NuGet 5.3.0, run in a temporary external
.NET 9 harness. This is verification tooling; Hover has no .NET dependency.
Comparator assembly SHA-256:
`AAC9327B4A9F25479432A35C0C7A406E8A1C4859D996CA6CF114C5195654B2AB`.
The package version and source-research revision are pinned separately; they
are not asserted to be the same build.

Fixture: upstream `LlamaLogic.Packages.UnitTests/TestPackage.package` at the
pinned source revision; 3,186 bytes, DBPF 2.1, 20 index entries. SHA-256 before
and after both readers:
`63466BD02E9FEBFD6B02D823D2BEF4FF301167D94A1A9B1657AF7DCCCB0251C2`.
All 20 ordered resource keys and decompressed sizes match. The fixture remains
outside this repository; no private or third-party game/creator asset is bundled.

The Rust developer comparator is reproducible with:

```powershell
cargo run -p hover-formats --example dbpf_index -- <authorized-package>
```

The external reference harness opens the fixture with DataBasePackedFile and
prints Count, Keys (Type, Group, InstanceEx/Instance) and GetSize(key). No Set,
Save or CopyTo method is called. This checks index interpretation, not resource
decompression, visual fidelity or CC dependency resolution.

## Coverage and limits

Generated regression fixtures exercise all eight constant-key flag layouts,
with and without extended compression fields, a stale legacy pointer, each
truncated prefix, unsupported versions, invalid flags/counts, byte limits,
partial-prefix reporting and cancellation. The tests also run 4,096
deterministic mutations through the parser and retain
the one-entry output cap. This is a small regression corpus, not a fuzz campaign.
Broader game/build fixtures,
duplicates/deleted resources, concurrent changes, fuzzing, specialist visual
comparisons and packaged Explorer interaction remain required. Neither this
checkpoint nor a successful parse completes the Sims milestone.
