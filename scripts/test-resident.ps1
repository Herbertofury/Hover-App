param(
    [string]$Executable = (Join-Path $PSScriptRoot '../target/release/hover.exe'),
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '../target/resident-evidence')
)
$ErrorActionPreference = 'Stop'
$exe = (Resolve-Path -LiteralPath $Executable).Path
$cli = Join-Path (Split-Path $exe) 'hover-agent.exe'
$run = Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ('session-' + [guid]::NewGuid().ToString('N'))
$config = Join-Path $run 'appdata/Hover'
New-Item -ItemType Directory -Path $config -Force | Out-Null
$settings = Join-Path $config 'settings.ini'
[IO.File]::WriteAllText($settings,"[general]`nenabled = false`nshow_tray_icon = true`n",[Text.UTF8Encoding]::new($false))
$hash = (Get-FileHash -LiteralPath $settings).Hash
$checks = [Collections.Generic.List[string]]::new()
function Assert([bool]$ok,[string]$name){if(-not $ok){throw "FAILED: $name"}; $checks.Add($name)}
function Command([string]$argument,[string]$Controller=$cli){
    $s=[Diagnostics.ProcessStartInfo]::new($Controller)
    $s.ArgumentList.Add($argument); $s.UseShellExecute=$false; $s.CreateNoWindow=$true
    $s.RedirectStandardOutput=$true; $s.RedirectStandardError=$true
    $p=[Diagnostics.Process]::Start($s)
    if(-not $p.WaitForExit(10000)){throw "CLI timed out: $argument"}
    $text=$p.StandardOutput.ReadToEnd(); $errorText=$p.StandardError.ReadToEnd()
    if($p.ExitCode -ne 0){throw "CLI failed: $argument $errorText"}
    return $text | ConvertFrom-Json
}
function Launch([string]$App=$exe) {
    $s=[Diagnostics.ProcessStartInfo]::new($App); $s.UseShellExecute=$false
    $s.Environment['APPDATA']=Split-Path $config
    return [Diagnostics.Process]::Start($s)
}
function Wait-Resident([int]$ExpectedPid,[string]$Controller=$cli){
    for($i=0;$i -lt 100;$i++){
        $status=Command '--resident-status' $Controller
        if($status.running -and $status.pid -eq $ExpectedPid){return $true}
        Start-Sleep -Milliseconds 50
    }
    return $false
}
$first=$null; $second=$null; $other=$null; $otherCli=$null
try {
    Assert (-not (Command '--resident-status').running) 'No prior resident in tested installation'
    $identity=Command '--version-json'
    Assert ($identity.app -eq 'Hover' -and $identity.agentProtocol -eq 1 -and -not $identity.releaseVerified) 'CLI identifies build without claiming release certification'
    $first=Launch
    Assert (Wait-Resident $first.Id) 'Normal GUI launch registers verified same-installation resident'
    $otherDirectory=Join-Path $run 'other-installation'
    New-Item -ItemType Directory -Path $otherDirectory | Out-Null
    foreach($name in @('hover.exe','hover-agent.exe')){Copy-Item -LiteralPath (Join-Path (Split-Path $exe) $name) -Destination (Join-Path $otherDirectory $name)}
    $otherCli=Join-Path $otherDirectory 'hover-agent.exe'
    Assert (-not (Command '--resident-status' $otherCli).running) 'A different installation cannot discover this resident as its own'
    $other=Launch (Join-Path $otherDirectory 'hover.exe')
    Assert (Wait-Resident $other.Id $otherCli) 'A separate installation has an independent native resident'
    Assert ((Command '--resident-status').pid -eq $first.Id) 'Separate installation preserves original resident identity'
    Assert ((Command '--shutdown' $otherCli).sent -and $other.WaitForExit(5000) -and $other.ExitCode -eq 0 -and -not $first.HasExited) 'Shutdown in another installation leaves the original resident running'
    $duplicate=Launch
    Assert ($duplicate.WaitForExit(5000) -and $duplicate.ExitCode -eq 0) 'Duplicate normal launch exits successfully'
    Assert ((Command '--resident-status').pid -eq $first.Id) 'Duplicate launch preserves original resident'
    Assert ((Command '--show').sent) 'First-party show request delivered'
    Assert ((Command '--shutdown').sent) 'First-party shutdown request delivered'
    Assert ($first.WaitForExit(5000) -and $first.ExitCode -eq 0) 'Normal tray app shuts down gracefully'
    Assert (-not (Command '--resident-status').running) 'Shutdown removes resident window and registration'
    Assert (-not (Command '--shutdown').sent) 'Repeated shutdown has no unrelated process target'
    $second=Launch
    Assert (Wait-Resident $second.Id) 'Normal GUI restarts after graceful shutdown'
    Assert ((Command '--shutdown').sent -and $second.WaitForExit(5000) -and $second.ExitCode -eq 0) 'Restarted app shuts down gracefully'
    Assert ((Get-FileHash -LiteralPath $settings).Hash -eq $hash) 'Lifecycle commands preserve isolated normal settings'
    $result=[ordered]@{schemaVersion=1;scope='Actual native normal GUI lifecycle; synthetic isolated settings; no Explorer input or taskbar pin claim';passed=$true;assertions=$checks.Count;checks=$checks;build=$identity;executableSha256=(Get-FileHash -LiteralPath $exe).Hash}
    $result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $run 'result.json') -Encoding utf8
    $result | ConvertTo-Json -Depth 8
    Write-Host "Retained evidence: $run"
} finally {
    if($other -and -not $other.HasExited){[void](Command '--shutdown' $otherCli);if(-not $other.WaitForExit(5000)){$other.Kill();$other.WaitForExit();throw 'Separate owned resident failed cleanup'}}
    foreach($p in @($first,$second)){
        if($p -and -not $p.HasExited){ [void](Command '--shutdown'); if(-not $p.WaitForExit(5000)){ $p.Kill(); $p.WaitForExit(); throw 'Owned resident required forced cleanup; lifecycle test failed' } }
    }
}
exit 0
