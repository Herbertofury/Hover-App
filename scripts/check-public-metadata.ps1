param([string]$Revision = 'HEAD')
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $repoRoot
try {
    $commit = git rev-parse --verify --end-of-options "$Revision`^{commit}"
    if ($LASTEXITCODE -ne 0) { throw 'Cannot resolve revision for metadata check' }
    $emails = @(git log '--format=%ae%n%ce' $commit)
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect commit metadata' }
    $personal = @($emails | Where-Object {
        $_ -notlike '*@users.noreply.github.com' -and $_ -ne 'noreply@github.com'
    })
    if ($personal.Count -ne 0) {
        throw 'Publication blocked: personal author/committer metadata exists in this history. Use the repository-local GitHub no-reply identity and inspect the affected commits privately.'
    }
    Write-Output 'Public commit metadata check passed.'
} finally { Pop-Location }
