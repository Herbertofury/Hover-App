# Native Windows development packages

Hover has a real per-user MSI authoring and test path, using the Windows SDK,
Windows Installer and makecab. These are unsigned development packages, not a
verified GitHub release. The previous WiX/MSIX scaffolds are not the tested path.

The app embeds a multi-size version of its existing code-native mark, Windows
file/product version resources, JSON build identity and the same
`Herbertofury.Hover` AppUserModelID used by its Start Menu shortcut. Build identity
and a local SHA-256 manifest are inspection aids, not publisher authentication.

## Reproduce on Windows

Use Visual Studio C++ tools, Windows SDK MSI authoring tools and Rust 1.95 or
later. The SDK tools must already be installed from Microsoft. From a clean
source checkout:

```powershell
./scripts/test-development-installation.ps1
# Optional reviewed public fixture:
./scripts/test-development-installation.ps1 -DbpfFixture path/to/TestPackage.package
```

The script extracts the locally installed, signature-verified Microsoft Orca
SDK package into a fresh evidence directory. It builds static-CRT payloads from
the same source with development versions 0.1.0 and 0.1.1, remaps build paths,
bundles project/dependency notices and checks uncompressed shipped files for
Windows profile paths. This is a versioned **same-source upgrade fixture**,
not evidence of an upgrade between different production implementations.
`-AllowDirty` is available for private prototypes; their identity says `-dirty`.
It is never passed in CI. Tools, validation cubes and personal files are not
included in the shipped payload.

Each MSI is validated before installation using all ICEs in the trusted SDK's
darice.cub, through the documented Evalcom2 ABI. The adapter loads the
signature-verified engine without COM registration or privilege changes.
Errors and unknown warnings stop the build. The reviewed warnings are ICE06
about the SDK validation action schema, ICE61 about explicit newer-version
removal for local rollback, and ICE91 about deliberate per-user-only files.
The complete messages and reasons remain beside each MSI. No Hover custom
actions, services, browser registration, startup change or file associations
are added by this package.

The test uses a unique `Hover-Test-...` folder and product family under the
current user's Programs directory. It verifies packaged/installed file hashes,
Windows Installer registration, CLI version, actual Shell shortcut launch,
shortcut AppUserModelID/icon, native previews and LPAC decoding through both
GUI and console entry points, graceful normal restart and separation between
two installation folders. It tests a deliberately missing incoming executable,
checks that rollback leaves the old product installed and usable, then tests
successful upgrade, denied downgrade, explicit older-package rollback and
uninstall. An extra user file survives every transaction and uninstall.
All MSI logs, raw app responses, fixtures and painter exports are retained in
`target/packaging-evidence`; logs can contain private paths and must be reviewed
before sharing.

The initial private prototype removed the old product before copying incoming
files. A failed upgrade restored files but failed to restore registration on
this unelevated Windows session. That prototype failed acceptance and was
uninstalled through Windows Installer. The current package schedules
InstallExecute, RemoveExistingProducts, InstallFinalize in Microsoft's
[supported order](https://learn.microsoft.com/en-us/windows/win32/msi/removeexistingproducts-action).
Incoming payload failures now roll back before touching the old registration.
Metadata/notices/browser host follow the versioned GUI as companion files, so
version changes replace managed payload consistently. Settings and user-added
files are not managed payload. This test does not cover every possible failure
inside old-product removal, power loss or system recovery.

## Stable development installation

For a reviewed clean source commit:

```powershell
$sdk = ./scripts/prepare-installer-sdk.ps1 -OutputDirectory target/sdk-for-install
./scripts/build-development.ps1 -Version 0.1.1 -OutputDirectory target/development-build
./installer/native/build-msi.ps1 -PayloadDirectory target/development-build/payload `
  -Version 0.1.1 -SdkDirectory $sdk -OutputDirectory target/development-msi
msiexec /i target/development-msi/Hover-Development-0.1.1-x64.msi /qn /norestart
```

The default location is `%LOCALAPPDATA%\Programs\Hover-Development` with a
Start Menu shortcut. Stop its normal resident with the installed
`hover-agent.exe --shutdown` before replacing it. Never stop an unrelated
installation. Installed executables support `--version-json`; the console
entry also offers `--resident-status`, `--show`, `--shutdown` and `--stdio`.
The GUI entry supports `--agent-stdio` with redirected pipes.

An explicit trusted local rollback requires both MSI properties
`HOVER_ALLOW_ROLLBACK=1 REINSTALLMODE=amus`. The latter restores older managed
files rather than retaining newer executable versions. Omitting either guard
is tested to fail before the transaction. These properties are a deliberate
local operator action, not a security boundary or an automatic updater.

## Evidence boundaries

Clean source `0a883d67ffd08adc4dc236f34680e5e2a2acd1ea` passed 93 installer assertions, plus eight 61-check
installed-app suites and four 17-check normal lifecycle suites with the public
DBPF fixture. Without that fixture each preview suite has 56 checks. These
counts cover actual Windows processes and native installer transactions,
including source-byte and installed-file hash checks; they do not establish
Explorer hit testing, physical typing/dragging, screen-reader behavior,
mixed DPI, taskbar pinning, real game/3D fidelity, storage-cold performance or
authenticated network updates. One earlier prototype app-suite quit check
failed; explicit quit exit diagnostics are now retained rather than weakening
the assertion. Later complete prototype runs passed it.

Retained public summaries: [clean-source Windows test](evidence/windows-development-install-local-2026-11-12.json),
[exact stable development packages and final install](evidence/windows-development-stable-install-2026-11-12.json),
and [51 installed-app performance samples](evidence/windows-development-installed-performance-2026-11-12.json).
The stable default packages also passed the full 93-check transaction suite;
version 0.1.1 was then installed again, passed 61 preview and 17 lifecycle checks,
and launched normally through its Start Menu shortcut. The performance probe
labels caches as uncontrolled and command/response timing separately from
screen latency; it is not an Explorer speed claim. Public CI acceptance is
tracked in PR #46 and must pass before this branch enters protected main.

The first hosted installer jobs exposed a test-fixture collision: the decoder
permission test created a volatile intermediate `Software\Hover` registry key
on fresh profiles. Windows rejects permanent installation keys beneath a
volatile parent. A local isolated probe reproduced that exact restriction;
the disposable decoder sentinel now lives directly beneath existing Software,
outside Hover's real namespace. No ACL, security policy or privilege change is
used to resolve this test isolation defect.

Taskbar pinning and source-authenticated update delivery remain release gates.
No signing certificate or trust-store entry is installed. No GitHub release is
published by these scripts. Exact release artifacts still need all remaining
Windows, format, safety and delivery acceptance evidence.

Primary API references: [SDK validation](https://learn.microsoft.com/en-us/windows/win32/msi/validation-automation),
[Evalcom2](https://learn.microsoft.com/en-us/windows/win32/msi/using-evalcom2),
[companion files](https://learn.microsoft.com/en-us/windows/win32/msi/companion-files).

The comparison prototype exposed makecab's default disk-size split after the
executable grew: the fixed cabinet name was overwritten with a chained cabinet
missing HoverExe. Actual installation failed with 1334 despite zero ICE errors.
Both package and negative-fixture builders now disable disk/cabinet splitting.
`installer/native/check-cabinet.ps1` rejects chained/spanning cabinets and verifies
every logical ID, size and sequence against the MSI File table before authoring.
This checks cabinet structure, not decompressed content; installed-file hashes
remain required. The corrected prototype passed 97 installer assertions.

For the daily unsigned development installation, `test-installation.ps1
-UpgradeInstalledDevelopment` requires the exact package A already installed,
validates every managed file hash, and stops its resident through the scoped CLI.
It exercises failure, upgrade and explicit rollback, then installs package B
again and launches it from the Start Menu. It preserves existing user-added
files and never uninstalls this daily app in test cleanup. On failure it retains
the installation and logs for inspection. This is explicit local artifact
testing; it does not establish authenticated network updates or release trust.
