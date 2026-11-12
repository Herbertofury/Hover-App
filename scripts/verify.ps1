$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $repoRoot
try {
    foreach ($arguments in @(
        @('fmt','--all','--','--check'),
        @('clippy','--workspace','--all-targets','--locked','--','-D','warnings'),
        @('test','--workspace','--locked'),
        @('build','--release','--workspace','--locked')
    )) {
        & cargo @arguments
        if ($LASTEXITCODE -ne 0) { throw "cargo $arguments failed" }
    }
} finally { Pop-Location }
