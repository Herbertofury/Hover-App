param(
    [Parameter(Mandatory)][string]$Version,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [string]$RustToolchain='1.95.0',
    [switch]$AllowDirty
)
$ErrorActionPreference='Stop'
$root=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$output=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $output){throw 'Use a new output directory; builds are never overwritten'}
if($Version -notmatch '^\d+\.\d+\.\d+$'){throw 'Use a numeric three-part development version'}
Push-Location $root
$saved=@{}
foreach($name in @('CARGO_ENCODED_RUSTFLAGS','RUSTFLAGS','HOVER_PACKAGE_VERSION','HOVER_SOURCE_REVISION')){$saved[$name]=[Environment]::GetEnvironmentVariable($name,'Process')}
try{
    $revision=(git rev-parse HEAD).Trim(); if($LASTEXITCODE -ne 0){throw 'Cannot identify source commit'}
    $dirty=@(git status --porcelain).Count -gt 0
    if($LASTEXITCODE -ne 0){throw 'Cannot inspect source state'}
    if($dirty -and -not $AllowDirty){throw 'Commit reviewed source before creating a traceable package, or explicitly use -AllowDirty for a private prototype'}
    if($dirty){$revision+='-dirty'}
    $env:HOVER_PACKAGE_VERSION=$Version; $env:HOVER_SOURCE_REVISION=$revision
    $flags=@('-C','target-feature=+crt-static',"--remap-path-prefix=$root=/workspace/Hover")
    if($env:USERPROFILE){$flags+="--remap-path-prefix=$env:USERPROFILE=/local"}
    $env:CARGO_ENCODED_RUSTFLAGS=$flags -join [char]31
    [Environment]::SetEnvironmentVariable('RUSTFLAGS',$null,'Process')
    cargo "+$RustToolchain" build --release --workspace --locked
    if($LASTEXITCODE -ne 0){throw 'Static CRT development build failed'}
    $payload=Join-Path $output 'payload';New-Item -ItemType Directory -Path $payload -Force | Out-Null
    foreach($name in @('hover.exe','hover-agent.exe','hover-browser-host.exe')){Copy-Item -LiteralPath (Join-Path $root "target/release/$name") -Destination (Join-Path $payload $name)}
    foreach($name in @('LICENSE','THIRD-PARTY-NOTICES.md')){Copy-Item -LiteralPath (Join-Path $root $name) -Destination (Join-Path $payload $name)}
    $licenses=[Text.StringBuilder]::new()
    foreach($file in Get-ChildItem -LiteralPath (Join-Path $root 'third-party') -Recurse -File | Sort-Object FullName){
        $relative=[IO.Path]::GetRelativePath($root,$file.FullName).Replace('\','/')
        [void]$licenses.AppendLine("===== $relative =====");[void]$licenses.AppendLine([IO.File]::ReadAllText($file.FullName));[void]$licenses.AppendLine()
    }
    [IO.File]::WriteAllText((Join-Path $payload 'DEPENDENCY-LICENSES.txt'),$licenses.ToString(),[Text.UTF8Encoding]::new($false))
    $identity=& (Join-Path $payload 'hover-agent.exe') --version-json | ConvertFrom-Json
    if($LASTEXITCODE -ne 0 -or $identity.packageVersion -ne $Version -or $identity.sourceRevision -ne $revision){throw 'Built identity mismatch'}
    $info=[ordered]@{schemaVersion=1;channel='unsigned-development';releaseVerified=$false;repository='https://github.com/Herbertofury/Hover-App';sourceRevision=$revision;sourceDirty=$dirty;staticCrt=$true;rustToolchain=$RustToolchain;cargoLockSha256=(Get-FileHash -LiteralPath (Join-Path $root 'Cargo.lock')).Hash;identity=$identity;files=@(Get-ChildItem -LiteralPath $payload -File | Sort-Object Name | ForEach-Object {[ordered]@{name=$_.Name;sha256=(Get-FileHash -LiteralPath $_.FullName).Hash}})}
    $info | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $payload 'BUILD-INFO.json') -Encoding utf8
    # Check ASCII and UTF-16 paths in every uncompressed shipped file.
    foreach($file in Get-ChildItem -LiteralPath $payload -File){
        $bytes=[IO.File]::ReadAllBytes($file.FullName)
        foreach($encoding in @([Text.Encoding]::UTF8,[Text.Encoding]::Unicode)){
            $text=$encoding.GetString($bytes)
            if($text -match '(?i)[A-Z]:[\\/]Users[\\/]'){throw "Private Windows profile path in payload: $($file.Name)"}
        }
    }
    Write-Host "Prepared unsigned development payload: $payload (source $revision)"
}finally{
    foreach($name in $saved.Keys){[Environment]::SetEnvironmentVariable($name,$saved[$name],'Process')}
    Pop-Location
}
exit 0
