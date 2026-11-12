# Third-party notices

Hover's original desktop source identifies rust-hover-preview as the reference
for its MSAA/Shell target resolution, Win32 overlay, GIF compositing and media
command approach. The reference is MIT licensed at revision
`90f9e3b59ad4030bdbbe97e2039bb869c720449f`:
[upstream](https://github.com/cosmokud/rust-hover-preview/tree/90f9e3b59ad4030bdbbe97e2039bb869c720449f).
Its complete copyright and license are retained in
[third-party/rust-hover-preview-LICENSE.txt](third-party/rust-hover-preview-LICENSE.txt).
The reference checkout and its generated binaries are excluded from this repo.

Cargo dependency license expressions and complete bundled notice files are
listed in [third-party/DEPENDENCIES.md](third-party/DEPENDENCIES.md). Regenerate
them with `pwsh -File scripts/collect-licenses.ps1` after changing Cargo.lock.
Retain these notices with distributed binaries.

WIC, Shell and Win32 APIs are supplied by Windows. External ffplay, mpv or VLC
are optional, separately installed playback tools. They are not bundled or
relicensed by Hover. No specialist game tool, game data, private save, creator
asset or copyleft-only game implementation has been imported in this baseline.
For dependencies offering MIT or Apache alongside copyleft alternatives, the
permissive option is selected; r-efi is one such alternative-license package
and is not a Windows runtime dependency. Unicode and Zlib notices remain
applicable where specified by the individual package.
