$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location -LiteralPath $repoRoot
try {
    $metadataJson = cargo metadata --format-version 1 --locked
    if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
    $metadata = $metadataJson | ConvertFrom-Json
    $noticeRoot = Join-Path $repoRoot 'third-party/registry'
    New-Item -ItemType Directory -Path $noticeRoot -Force | Out-Null
    $index = [System.Collections.Generic.List[string]]::new()
    $index.Add('# Cargo dependency notices')
    $index.Add('')
    $index.Add('Generated from the exact Cargo.lock by scripts/collect-licenses.ps1. Includes all resolved targets, including build tools. No runtime media players are bundled.')
    $index.Add('')
    foreach ($package in ($metadata.packages | Where-Object source | Sort-Object name,version)) {
        if (-not $package.license) { throw "Missing license: $($package.name)" }
        $packageRoot = Split-Path -Parent $package.manifest_path
        $licenseFiles = @(Get-ChildItem -LiteralPath $packageRoot -File | Where-Object Name -Match '^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT|AUTHORS)($|[._-])')
        $licenseDirectory = Join-Path $packageRoot 'LICENSES'
        if (Test-Path -LiteralPath $licenseDirectory) {
            $licenseFiles += @(Get-ChildItem -LiteralPath $licenseDirectory -File -Recurse)
        }
        if ($licenseFiles.Count -eq 0) { throw "No license text for $($package.name)" }
        $packageLabel = "$($package.name)-$($package.version)"
        $destination = Join-Path $noticeRoot $packageLabel
        New-Item -ItemType Directory -Path $destination -Force | Out-Null
        foreach ($file in $licenseFiles) {
            Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $destination $file.Name) -Force
        }
        $index.Add("- [$packageLabel](registry/$packageLabel/): $($package.license); [upstream]($($package.repository)).")
    }
    $index | Set-Content -LiteralPath (Join-Path $repoRoot 'third-party/DEPENDENCIES.md') -Encoding utf8
} finally { Pop-Location }
