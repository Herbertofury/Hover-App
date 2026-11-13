# Native decoder permissions and evidence

WIC/GIF decoding now uses Windows Less Privileged AppContainer (LPAC), in
addition to the existing resource-limited job and bounded byte protocol. This
is native app code, independent of Computer Use. No uncontained fallback runs
when startup, token verification or decoding fails.

The parent creates the worker suspended with its job, LPAC policy and an
explicit handle list already applied. It verifies the actual AppContainer
identity, low integrity and exactly two system capabilities: `lpacCom` and
`registryRead`. These permit WIC's COM/registry initialization; no network,
document/library or broad filesystem capability is requested.

Before resuming the child, supported Windows `AccessCheck` calls verify that
its token permits an exact-package grant and rejects an `ALL APPLICATION
PACKAGES`-only grant. An ordinary AppContainer negative control is rejected
before any child code or input runs. This avoids relying on the non-portable
`TokenIsLessPrivilegedAppContainer` query, which returned error 87 here. The
worker rechecks its token before parsing input. See Microsoft's
[LPAC definition](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ne-winnt-token_information_class),
[AccessCheck](https://learn.microsoft.com/en-us/windows/win32/api/securitybaseapi/nf-securitybaseapi-accesscheck)
and [launch attributes](https://learn.microsoft.com/en-us/windows/win32/secauthz/implementing-an-appcontainer).

Only the input and output pipes are inherited; stderr shares the output pipe.
Unexpected output still fails the strict protocol. The source file is closed
before launch and its path/handle is never sent. The worker starts in the
Windows system directory, with only required OS/profile environment paths.
Arbitrary parent environment variables and credentials are not inherited.
Windows requires `LOCALAPPDATA` to locate/remap the AppContainer profile; the
[Chromium launcher](https://chromium.googlesource.com/chromium/src/+/04d774d3827c8532b1b7d3966629f9193a35dd0e/sandbox/win/src/target_process.cc)
also documents that requirement. No Chromium code was imported.

Windows creates/reuses one per-user `Herbertofury.Hover.ImageDecoder.v1`
profile. The parent grants this exact package SID read/execute access to its
own executable if needed, preserving existing ACL entries and ownership.
This manual ACL adjustment grants no directory, preview-file or parent-process
permissions; Windows separately provisions its own AppContainer profile. A system
installation must provision that narrow executable grant; insufficient rights
produce an explicit repair error without elevation or fallback. Executable
bytes remain unchanged. No firewall settings or loopback exemptions are changed.

## Actual verification

The [local native permission log](evidence/lpac-native-local-2026-11-12.txt)
records real Windows denial of private-file reads/writes, writes to a package-
readable sentinel, reads of an `ALL APPLICATION PACKAGES`-only sentinel, access
to an existing parent registry sentinel, parent-process read/write access and an
owned loopback TCP listener (Winsock 10013). Parent positive controls prove
those resources exist and are usable outside the sandbox. The exact-package
read-only sentinel remains readable; all fixture bytes remain unchanged.

An intentionally inheritable private-file handle is excluded from the child.
The parent checks its suspended child's actual handle table using
[DuplicateHandle](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-duplicatehandle)
and [CompareObjectHandles](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-compareobjecthandles),
with a positive control. It does not mistake an unrelated numeric handle for
the private file. Probes are compiled only into the test executable.

The first ordinary-AppContainer prototype denied private resources but still
connected to loopback on this machine despite having zero capabilities. That
was a failed network-isolation test, not a pass. LPAC corrected the observed
gap; the retained successful log includes the actual denial. No blanket claim
about all Windows configurations follows from one machine.

Both actual release entry points passed 61 local app assertions with generated
files and the pinned public DBPF index fixture. See [GUI results](evidence/lpac-agent-local-2026-11-12.json)
and [console summary](evidence/lpac-console-local-2026-11-12.json). Crash, stall,
cancellation, allocation, descendant and non-spinning-supervisor tests now use
the same LPAC launcher, not an ordinary child. Actual CPU-time termination is
run only on dedicated Windows CI; check the hosted result before claiming it.

The [six-format component comparison](evidence/lpac-codecs-local-2026-11-12.json)
compares PNG, JPEG, BMP, TIFF, GIF and PNG-in-ICO fixtures with the in-process
reference, checking exact pixels, dimensions, stride, delays and unchanged
source hashes. These are generated Windows GDI+ encodings, not real-game or
independent specialist fidelity evidence. `scripts/compare-wic-fixtures.ps1`
reproduces the comparison after building `decoder_bench`. Its executable hash,
individual samples and uncontrolled-cache scope are retained. No speedup is
claimed. A [short native resource probe](evidence/lpac-performance-local-2026-11-12.json)
also exercises the actual LPAC build; its endpoint/timer limits still apply.

## Remaining boundaries

LPAC still permits explicitly granted resources, shared OS resources and its
own persistent profile, which is shared by Hover workers. This is not a
write-free, disk-budgeted or per-asset profile. Parent source I/O, profile/launch
APIs, GPU/broker activity, external media and additional codec/version coverage
need further hard boundaries and evidence. The 8-second supervised transaction
cannot interrupt a parent Windows API blocked during startup or source reads.
The worker's resource limits apply once Windows creates it.

Install/provisioning, source-authenticated updates, rollback, Explorer/physical
input, accessibility/DPI and all game-specific/artist fidelity gates remain
open. This development source and its tests are not release certification.
