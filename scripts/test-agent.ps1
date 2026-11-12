param(
    [string]$Executable = (Join-Path $PSScriptRoot '../target/release/hover.exe'),
    [string]$AgentArguments = '--agent-stdio',
    [string]$DbpfFixture = '',
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '../target/agent-evidence')
)
$ErrorActionPreference = 'Stop'
$executablePath = (Resolve-Path -LiteralPath $Executable).Path
$runDirectory = Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ('session-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
$script:checks = [Collections.Generic.List[string]]::new()
$script:responses = [Collections.Generic.List[object]]::new()
$script:timings = [Collections.Generic.List[object]]::new()
$script:sequence = 0
$script:agentProcess = $null
$sentinelConfig = Join-Path $runDirectory 'sentinel-appdata'
$sentinelHover = Join-Path $sentinelConfig 'Hover'
New-Item -ItemType Directory -Path $sentinelHover -Force | Out-Null
$sentinelSettings = Join-Path $sentinelHover 'settings.ini'
[IO.File]::WriteAllText($sentinelSettings,"[preview_types]`nimages = false`ntext = false`n",[Text.UTF8Encoding]::new($false))
$sentinelHash = (Get-FileHash -LiteralPath $sentinelSettings -Algorithm SHA256).Hash

function Assert-That([bool]$Condition, [string]$Name) {
    if (-not $Condition) { throw "FAILED: $Name" }
    $script:checks.Add($Name)
}

function Start-OwnedAgent {
    $startInfo = [Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $executablePath
    $startInfo.Arguments = $AgentArguments
    $startInfo.UseShellExecute = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.WindowStyle = [Diagnostics.ProcessWindowStyle]::Hidden
    $startInfo.RedirectStandardInput = $true
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.StandardInputEncoding = [Text.UTF8Encoding]::new($false)
    $startInfo.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
    $startInfo.EnvironmentVariables['APPDATA'] = $sentinelConfig
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    if (-not $process.Start()) { throw 'Could not start owned Hover agent' }
    return $process
}

function Invoke-Hover([hashtable]$Command, [switch]$AllowError) {
    $script:sequence++
    $Command.id = $script:sequence
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $script:agentProcess.StandardInput.WriteLine(($Command | ConvertTo-Json -Compress -Depth 12))
    $script:agentProcess.StandardInput.Flush()
    $read = $script:agentProcess.StandardOutput.ReadLineAsync()
    if (-not $read.Wait(15000)) { throw "No response to $($Command.command)" }
    $clock.Stop()
    if ($null -eq $read.Result) { throw "Agent closed stdout: $($script:agentProcess.StandardError.ReadToEnd())" }
    $response = $read.Result | ConvertFrom-Json
    if ($response.id -ne $script:sequence -or $response.protocol -ne 1) { throw 'Invalid response identity/protocol' }
    $script:responses.Add($response)
    $script:timings.Add([ordered]@{command=$Command.command; roundTripMs=$clock.Elapsed.TotalMilliseconds})
    if (-not $AllowError -and -not $response.ok) { throw "Agent rejected $($Command.command): $($response.error)" }
    if ($AllowError) { return $response }
    return $response.result
}

function Stop-OwnedAgent($Process) {
    if ($null -eq $Process) { return }
    try {
        if (-not $Process.HasExited) {
            $Process.StandardInput.Close()
            if (-not $Process.WaitForExit(4000)) { $Process.Kill(); $Process.WaitForExit(4000) | Out-Null }
        }
    } finally { $Process.Dispose() }
}

$textPath = Join-Path $runDirectory ('fixture-' + [char]0x96ea + '.txt')
$imagePath = Join-Path $runDirectory 'quadrants.bmp'
$badPath = Join-Path $runDirectory 'corrupt.bin'
$videoPath = Join-Path $runDirectory 'paused.mp4'
[IO.File]::WriteAllLines($textPath, (0..119 | ForEach-Object { "original line $_" }), [Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllBytes($badPath, [byte[]](1,2,3,4,5))
[IO.File]::WriteAllBytes($videoPath, [byte[]](0,0,0,0))
$width = 800; $height = 600
$imageBytes = [byte[]]::new($width*$height*4)
for ($y=0; $y -lt $height; $y++) {
    for ($x=0; $x -lt $width; $x++) {
        $i=($y*$width+$x)*4
        $imageBytes[$i] = if ($x -lt 400) { 20 } else { 220 }
        $imageBytes[$i+1] = if ($y -lt 300) { 180 } else { 30 }
        $imageBytes[$i+2] = if ($x -lt 400) { 230 } else { 30 }
        $imageBytes[$i+3] = 255
    }
}
$stream = [IO.File]::Open($imagePath,[IO.FileMode]::CreateNew)
$writer = [IO.BinaryWriter]::new($stream)
try {
    $writer.Write([byte[]](66,77)); $writer.Write([uint32](54+$imageBytes.Length)); $writer.Write([uint32]0); $writer.Write([uint32]54)
    $writer.Write([uint32]40); $writer.Write([int32]$width); $writer.Write([int32](-$height)); $writer.Write([uint16]1); $writer.Write([uint16]32)
    $writer.Write([uint32]0); $writer.Write([uint32]$imageBytes.Length); $writer.Write([byte[]]::new(16)); $writer.Write($imageBytes)
} finally { $writer.Dispose() }
$imageHash = (Get-FileHash -LiteralPath $imagePath -Algorithm SHA256).Hash
$success = $false
try {
    $script:agentProcess = Start-OwnedAgent
    $capabilities = Invoke-Hover @{command='capabilities'}
    Assert-That ($capabilities.owns_session -and $capabilities.commands -contains 'capture') 'Real app exposes versioned private stdio control'
    Assert-That ($capabilities.configuration -eq 'ephemeral_defaults_no_startup_mutation') 'Agent session isolates real user settings and startup registry'
    $support = Invoke-Hover @{command='support'}
    Assert-That (-not $support.releaseVerified -and $support.games.Count -eq 16) 'Agent exposes truthful bundled status for every game milestone'
    $settings = Invoke-Hover @{command='settings'}
    Assert-That (-not $settings.persisted -and -not $settings.startup_mutation -and $settings.effective_ini.Contains('images = true')) 'Effective settings ignore the deliberately disabled disk profile'
    $state = Invoke-Hover @{command='status'}
    Assert-That ($state.pid -eq $script:agentProcess.Id -and $state.phase -eq 'empty' -and -not $state.visible) 'Status belongs to the owned native Windows process'
    $script:agentProcess.StandardInput.WriteLine('[]')
    $script:agentProcess.StandardInput.Flush()
    $invalidRead = $script:agentProcess.StandardOutput.ReadLineAsync()
    Assert-That ($invalidRead.Wait(5000) -and -not ($invalidRead.Result | ConvertFrom-Json).ok) 'Malformed request shape reports an error and the session remains usable'
    $invalid = Invoke-Hover @{command='open';path=$textPath;unexpected=$true} -AllowError
    Assert-That (-not $invalid.ok) 'Unknown fields are rejected before opening a preview'
    $state = Invoke-Hover @{command='open';path=$textPath}
    Assert-That ($state.path -eq $textPath -and $state.pinned -and -not $state.no_activate) 'Open uses actual native pinned window styles and exact Unicode identity'
    $state = Invoke-Hover @{command='wait'}
    Assert-That ($state.phase -eq 'ready' -and $state.content.kind -eq 'text') 'Generated text file decoded through the real worker and UI loop'
    $content = Invoke-Hover @{command='inspect';limit=2}
    Assert-That ($content.content.lines[0] -eq 'original line 0') 'Inspect returns the actual decoded text'
    $state = Invoke-Hover @{command='scroll';offset=20}
    Assert-That ($state.scroll -eq 20) 'Native text view scroll changes'
    $capture = Invoke-Hover @{command='capture';path=(Join-Path $runDirectory 'text-preview.bmp')}
    Assert-That ($capture.bytes -gt 10000 -and $capture.representation -eq 'native_preview_client_render') 'Actual preview painter and native controls export a bounded bitmap'
    $duplicate = Invoke-Hover @{command='capture';path=(Join-Path $runDirectory 'text-preview.bmp')} -AllowError
    Assert-That (-not $duplicate.ok) 'Capture refuses to overwrite an existing file'
    foreach ($deviceName in @('CON.bmp','CON .bmp',('COM'+[char]0xb9+'.bmp'),('LPT'+[char]0xb2+'.bmp'))) {
        $device = Invoke-Hover @{command='capture';path=(Join-Path $runDirectory $deviceName)} -AllowError
        Assert-That (-not $device.ok -and $device.error -like 'Device/stream*') 'Capture rejects a reserved Windows device alias before file I/O'
    }
    [IO.File]::WriteAllLines($textPath, (0..119 | ForEach-Object { "updated line $_" }), [Text.UTF8Encoding]::new($false))
    $textHash = (Get-FileHash -LiteralPath $textPath -Algorithm SHA256).Hash
    $null = Invoke-Hover @{command='reload'}
    $state = Invoke-Hover @{command='wait'}
    $content = Invoke-Hover @{command='inspect';limit=2}
    Assert-That ($state.scroll -eq 20 -and $content.content.lines[0] -eq 'updated line 0') 'External save reloads with real scroll continuity'
    $renamed = Join-Path $runDirectory 'renamed.txt'
    Move-Item -LiteralPath $textPath -Destination $renamed
    $null = Invoke-Hover @{command='reload'}
    $state = Invoke-Hover @{command='wait'}
    Assert-That ($state.phase -eq 'error' -and $state.path -eq $textPath -and $state.error) 'Rename succeeds during preview; missing source produces a named error'
    Move-Item -LiteralPath $renamed -Destination $textPath
    $null = Invoke-Hover @{command='reload'}
    $null = Invoke-Hover @{command='wait'}
    $null = Invoke-Hover @{command='open';path=$imagePath}
    $state = Invoke-Hover @{command='wait'}
    Assert-That ($state.phase -eq 'ready' -and $state.content.kind -eq 'image' -and $state.content.image_width -eq 800 -and $state.content.image_height -eq 600) 'WIC decodes generated BMP through the real application'
    Assert-That ($state.content.viewport_width -eq $state.content_rect.width -and $state.content.viewport_height -eq $state.content_rect.height) 'First pinned image fits the actual native client area including caption and border'
    $null = Invoke-Hover @{command='resize';width=500;height=400}
    $null = Invoke-Hover @{command='zoom';mode='native'}
    $state = Invoke-Hover @{command='pan';dx=90;dy=-60}
    $savedPanX=$state.content.pan_x; $savedPanY=$state.content.pan_y
    Assert-That ($state.content.scale -eq 1 -and $savedPanX -ne 0) 'Production zoom and pan operate in the real native viewport'
    $null = Invoke-Hover @{command='capture';path=(Join-Path $runDirectory 'image-preview.bmp')}
    $capturedBytes = [IO.File]::ReadAllBytes((Join-Path $runDirectory 'image-preview.bmp'))
    Assert-That ($capturedBytes[0] -eq 66 -and $capturedBytes[1] -eq 77 -and [BitConverter]::ToUInt16($capturedBytes,28) -eq 32) 'Capture has a valid 32-bit BMP header'
    $quadrantColors = [int[]]::new(4)
    for ($i=54; $i -lt $capturedBytes.Length; $i+=4) {
        if ($capturedBytes[$i+2] -eq 230 -and $capturedBytes[$i] -eq 20) {
            if ($capturedBytes[$i+1] -eq 180) { $quadrantColors[0]++ }
            elseif ($capturedBytes[$i+1] -eq 30) { $quadrantColors[2]++ }
        } elseif ($capturedBytes[$i+2] -eq 30 -and $capturedBytes[$i] -eq 220) {
            if ($capturedBytes[$i+1] -eq 180) { $quadrantColors[1]++ }
            elseif ($capturedBytes[$i+1] -eq 30) { $quadrantColors[3]++ }
        }
    }
    Assert-That (@($quadrantColors | Where-Object { $_ -gt 1000 }).Count -eq 4) 'Native painter retains all four known decoded quadrant colors; capture is not a blank surrogate'
    $null = Invoke-Hover @{command='reload'}
    $state = Invoke-Hover @{command='wait'}
    Assert-That ($state.content.scale -eq 1 -and $state.content.pan_x -eq $savedPanX -and $state.content.pan_y -eq $savedPanY) 'Compatible image reload preserves actual zoom and pan'
    $null = Invoke-Hover @{command='minimize'}
    $null = Invoke-Hover @{command='reload'}
    $state = Invoke-Hover @{command='wait'}
    Assert-That ($state.minimized -and $state.content.pan_x -eq $savedPanX) 'Reload leaves the real window minimized without losing its image view'
    $hiddenCapture = Invoke-Hover @{command='capture';path=(Join-Path $runDirectory 'forbidden-minimized.bmp')} -AllowError
    Assert-That (-not $hiddenCapture.ok -and -not (Test-Path -LiteralPath (Join-Path $runDirectory 'forbidden-minimized.bmp'))) 'Minimized capture fails without creating a file'
    $state = Invoke-Hover @{command='restore'}
    Assert-That (-not $state.minimized) 'Restore uses the real native window'
    $state = Invoke-Hover @{command='focus';target='content'}
    Assert-That ($state.content_focused) 'Agent focus inspection reads real native content focus'
    foreach ($label in @('Unpin','Reload','Close')) {
        $state = Invoke-Hover @{command='focus';target='next'}
        Assert-That (@($state.controls | Where-Object { $_.focused -and $_.label -eq $label }).Count -eq 1) "Native focus reaches $label control"
    }
    $state = Invoke-Hover @{command='focus';target='next'}
    Assert-That ($state.content_focused) 'Native focus returns to interactive content'
    for ($i=0; $i -lt 30; $i++) { $null = Invoke-Hover @{command='open';path=$(if ($i%2) {$imagePath} else {$textPath})} }
    $null = Invoke-Hover @{command='open';path=$textPath}
    $state = Invoke-Hover @{command='wait'}
    $content = Invoke-Hover @{command='inspect';limit=1}
    Assert-That ($state.path -eq $textPath -and $content.content.lines[0] -eq 'updated line 0') 'Rapid real-app replacement retains only the latest requested identity and content'
    $null = Invoke-Hover @{command='open';path=$imagePath}
    $null = Invoke-Hover @{command='close'}
    Start-Sleep -Milliseconds 100
    $state = Invoke-Hover @{command='status'}
    Assert-That ($state.phase -eq 'empty' -and -not $state.visible) 'Cancelled native decode cannot reopen a closed preview'
    $null = Invoke-Hover @{command='open';path=$badPath}
    $state = Invoke-Hover @{command='wait'}
    Assert-That ($state.phase -eq 'error' -and $state.error -and -not $state.loading) 'Corrupt/unsupported input terminates in an explicit error'
    $null = Invoke-Hover @{command='open';path=$videoPath}
    $state = Invoke-Hover @{command='wait'}
    Assert-That ($state.media_waiting -and -not $state.video_running -and -not $state.audio_running) 'Media recognition never starts an external player without explicit play'
    if ($DbpfFixture) {
        $dbpfPath = (Resolve-Path -LiteralPath $DbpfFixture).Path
        $dbpfHash = (Get-FileHash -LiteralPath $dbpfPath -Algorithm SHA256).Hash
        Assert-That ($dbpfHash -eq '63466BD02E9FEBFD6B02D823D2BEF4FF301167D94A1A9B1657AF7DCCCB0251C2') 'Public DBPF fixture matches the reviewed pinned upstream bytes'
        $null = Invoke-Hover @{command='open';path=$dbpfPath}
        $state = Invoke-Hover @{command='wait'}
        $content = Invoke-Hover @{command='inspect';limit=32}
        Assert-That ($state.phase -eq 'ready' -and $content.content.subtitle -like '*20/20 index entries inspected*') 'Actual Hover app loads every index entry of the specialist DBPF fixture'
        Assert-That (@($content.content.entries | Where-Object { $_.name -like '* 0x*' }).Count -eq 20 -and @($content.content.entries | Where-Object { $_.name -like 'String Table 0x*' }).Count -eq 18) 'Native metadata view exposes all 20 resource rows including 18 string tables'
        $null = Invoke-Hover @{command='capture';path=(Join-Path $runDirectory 'dbpf-preview.bmp')}
        $dbpfCaptureBytes = [IO.File]::ReadAllBytes((Join-Path $runDirectory 'dbpf-preview.bmp'))
        $captureWidth = [BitConverter]::ToInt32($dbpfCaptureBytes,18)
        $completeHeader = $true
        foreach ($control in $state.controls) {
            $ink = 0
            for ($y=$control.rect.y; $y -lt ($control.rect.y+$control.rect.height); $y++) {
                for ($x=$control.rect.x; $x -lt ($control.rect.x+$control.rect.width); $x++) {
                    $pixel = 54+($y*$captureWidth+$x)*4
                    if ($dbpfCaptureBytes[$pixel] -ne 25 -or $dbpfCaptureBytes[$pixel+1] -ne 22 -or $dbpfCaptureBytes[$pixel+2] -ne 20) { $ink++ }
                }
            }
            if ($ink -le 100) { $completeHeader=$false }
        }
        Assert-That ($completeHeader) 'Every native header control renders fully after view/focus/type changes'
        Assert-That ((Get-FileHash -LiteralPath $dbpfPath -Algorithm SHA256).Hash -eq $dbpfHash) 'Real upstream package remains byte-identical after native inspection and capture'
    }
    $state = Invoke-Hover @{command='close'}
    Assert-That ($state.phase -eq 'empty' -and -not $state.visible -and -not $script:agentProcess.HasExited) 'Close hides the preview and retains the resident application'
    Assert-That ((Get-FileHash -LiteralPath $imagePath -Algorithm SHA256).Hash -eq $imageHash) 'Source image remains byte-identical'
    Assert-That ((Get-FileHash -LiteralPath $textPath -Algorithm SHA256).Hash -eq $textHash) 'Externally updated source text remains byte-identical after preview/reload'
    Assert-That ((Get-FileHash -LiteralPath $sentinelSettings -Algorithm SHA256).Hash -eq $sentinelHash) 'Existing settings fixture remains byte-identical throughout the session'
    $null = Invoke-Hover @{command='quit'}
    Assert-That ($script:agentProcess.WaitForExit(5000) -and $script:agentProcess.ExitCode -eq 0) 'Quit closes the owned real app cleanly'
    Stop-OwnedAgent $script:agentProcess; $script:agentProcess=$null

    $eofProcess = Start-OwnedAgent
    try {
        $eofProcess.StandardInput.Close()
        Assert-That ($eofProcess.WaitForExit(5000) -and $eofProcess.ExitCode -eq 0) 'EOF closes only its owned Hover session'
    } finally { Stop-OwnedAgent $eofProcess }
    $oversizeProcess = Start-OwnedAgent
    try {
        $oversizeProcess.StandardInput.WriteLine(('x'*17000))
        $oversizeProcess.StandardInput.Flush()
        $read=$oversizeProcess.StandardOutput.ReadLineAsync()
        Assert-That ($read.Wait(5000)) 'Oversized protocol input receives a bounded error'
        $reply=$read.Result | ConvertFrom-Json
        Assert-That (-not $reply.ok -and $oversizeProcess.WaitForExit(5000) -and $oversizeProcess.ExitCode -eq 2) 'Oversized request stops its owned session without an unbounded allocation'
    } finally { Stop-OwnedAgent $oversizeProcess }
    $success = $true
} finally {
    Stop-OwnedAgent $script:agentProcess
    $evidence = [ordered]@{
        schemaVersion=1; passed=$success; fixtureType=$(if ($DbpfFixture) {'generated files plus pinned public DBPF index fixture'} else {'generated synthetic files on disk'}); execution='real Windows app via private stdio and native UI thread';
        dbpfFixture=$(if ($DbpfFixture) { [ordered]@{source='https://github.com/Llama-Logic/LlamaLogic/blob/4a3d576f0afb776aba783db561760afb5b5de8b7/LlamaLogic.Packages.UnitTests/TestPackage.package';sha256=$dbpfHash;stage='DBPF 2.1 index metadata only';gameBuild='not established'} } else {$null});
        excludes=@('Explorer targeting','physical input','screen reader','mixed DPI','installer/update/rollback','real game fidelity');
        executableSha256=(Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash;
        entrypoint=([IO.Path]::GetFileName($executablePath) + ' ' + $AgentArguments);
        os=[Environment]::OSVersion.VersionString; processArchitecture=$env:PROCESSOR_ARCHITECTURE;
        checks=$script:checks; commandRoundTrips=$script:timings; responses=$script:responses
    }
    $evidence | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath (Join-Path $runDirectory 'agent-results.json') -Encoding utf8
    Write-Output "Agent test evidence: $runDirectory"
    Write-Output "Passed checks: $($script:checks.Count); success: $success"
}
exit 0
