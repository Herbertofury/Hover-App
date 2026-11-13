param(
    [string]$Executable=(Join-Path $PSScriptRoot '../target/release/hover.exe'),
    [string]$AgentArguments='--agent-stdio',
    [string]$DbpfFixture='',
    [switch]$ReferenceChecks,
    [string]$OutputDirectory=(Join-Path $PSScriptRoot '../target/comparison-evidence')
)
$ErrorActionPreference='Stop'
$exe=(Resolve-Path -LiteralPath $Executable).Path
$run=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ('session-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $run -Force | Out-Null
$checks=[Collections.Generic.List[string]]::new();$responses=[Collections.Generic.List[object]]::new();$timings=[Collections.Generic.List[object]]::new();$idle=[Collections.Generic.List[object]]::new();$sequence=0;$process=$null;$passed=$false
if(-not ('HoverComparisonResources' -as [type])){Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;public static class HoverComparisonResources{[DllImport("user32.dll")]public static extern uint GetGuiResources(IntPtr process,uint flags);}'}
function Assert([bool]$ok,[string]$name){if(-not $ok){throw "FAILED: $name"};$checks.Add($name)}
function Invoke-App([hashtable]$Command,[switch]$AllowError){
    $script:sequence++;$Command.id=$script:sequence
    $clock=[Diagnostics.Stopwatch]::StartNew();$process.StandardInput.WriteLine(($Command|ConvertTo-Json -Compress -Depth 12));$process.StandardInput.Flush()
    $read=$process.StandardOutput.ReadLineAsync();if(-not $read.Wait(15000)){throw 'Native comparison request timed out'}
    if($null -eq $read.Result){throw "Native app exited: $($process.StandardError.ReadToEnd())"}
    $clock.Stop();$timings.Add([ordered]@{command=$Command.command;action=$Command.action;roundTripMs=$clock.Elapsed.TotalMilliseconds});$response=$read.Result|ConvertFrom-Json;$responses.Add($response)
    if($response.id -ne $script:sequence -or $response.protocol -ne 1){throw 'Incorrect response identity/protocol'}
    if(-not $response.ok -and -not $AllowError){throw "Native app rejected command: $($response.error)"}
    if($AllowError){return $response};return $response.result
}
function Compare-Files([string[]]$Paths){$null=Invoke-App @{command='compare';paths=$Paths};return (Invoke-App @{command='wait';timeout_ms=30000}).comparison}
function Control([string]$Action,[hashtable]$Fields=@{}){$Fields.command='comparison';$Fields.action=$Action;return Invoke-App $Fields}
function Sample-Idle([string]$Phase){
    $process.Refresh();$cpu=$process.TotalProcessorTime.TotalMilliseconds;$memory=$process.PrivateMemorySize64;$working=$process.WorkingSet64;$handles=$process.HandleCount;$gdi=[HoverComparisonResources]::GetGuiResources($process.Handle,0)
    $clock=[Diagnostics.Stopwatch]::StartNew();Start-Sleep -Milliseconds 1000;$clock.Stop();$process.Refresh()
    $idle.Add([ordered]@{phase=$Phase;wallMs=$clock.Elapsed.TotalMilliseconds;parentCpuMs=($process.TotalProcessorTime.TotalMilliseconds-$cpu);privateBytesBefore=$memory;privateBytesAfter=$process.PrivateMemorySize64;workingSetBefore=$working;workingSetAfter=$process.WorkingSet64;handlesBefore=$handles;handlesAfter=$process.HandleCount;gdiBefore=$gdi;gdiAfter=[HoverComparisonResources]::GetGuiResources($process.Handle,0);parentLifetimePeakWorkingSet=$process.PeakWorkingSet64})
}
function Bmp([string]$Path,[int]$Width=800,[int]$Height=600){
    $pixels=[byte[]]::new($Width*$Height*4)
    for($y=0;$y -lt $Height;$y++){for($x=0;$x -lt $Width;$x++){$i=($y*$Width+$x)*4;$pixels[$i]=if($x -lt $Width/2){30}else{210};$pixels[$i+1]=if($y -lt $Height/2){170}else{50};$pixels[$i+2]=70;$pixels[$i+3]=255}}
    $writer=[IO.BinaryWriter]::new([IO.File]::Open($Path,[IO.FileMode]::CreateNew))
    try{$writer.Write([byte[]](66,77));$writer.Write([uint32](54+$pixels.Length));$writer.Write([uint32]0);$writer.Write([uint32]54);$writer.Write([uint32]40);$writer.Write([int32]$Width);$writer.Write([int32](-$Height));$writer.Write([uint16]1);$writer.Write([uint16]32);$writer.Write([uint32]0);$writer.Write([uint32]$pixels.Length);$writer.Write([byte[]]::new(16));$writer.Write($pixels)}finally{$writer.Dispose()}
}
$a=Join-Path $run 'reference.bmp';$b=Join-Path $run 'changed.bmp';$c=Join-Path $run 'same-pixels-different-bytes.bmp';$large=Join-Path $run 'different-dimensions.bmp';$bad=Join-Path $run 'corrupt.bin';$left=Join-Path $run 'before.txt';$right=Join-Path $run 'after.txt';$gif=Join-Path $run 'animation.gif';$gif2=Join-Path $run 'animation-copy.gif';$video=Join-Path $run 'recognized-only.mp4'
Bmp $a;Bmp $large 1000 800
$bytes=[IO.File]::ReadAllBytes($a)
for($y=20;$y -lt 40;$y++){for($x=20;$x -lt 40;$x++){$bytes[54+($y*800+$x)*4]++}}
[IO.File]::WriteAllBytes($b,$bytes)
$bytes=[IO.File]::ReadAllBytes($a);$stream=[IO.File]::Open($c,[IO.FileMode]::CreateNew);try{$stream.Write($bytes);$stream.WriteByte(99)}finally{$stream.Dispose()}
[IO.File]::WriteAllBytes($bad,[byte[]](1,2,3,4,5));[IO.File]::WriteAllText($left,"one`nthree`n");[IO.File]::WriteAllText($right,"one`ntwo`nthree`n");[IO.File]::WriteAllBytes($video,[byte[]](0,0,0,0))
$gifHex='47494638396102000100800000ff000000ff0021ff0b4e45545343415045322e30030100000021f904040c0000002c0000000002000100000202440a0021f90404060000002c00000000020001000002020c0a003b'
$gifBytes=[Convert]::FromHexString($gifHex);[IO.File]::WriteAllBytes($gif,$gifBytes);[IO.File]::WriteAllBytes($gif2,$gifBytes)
$alpha=Join-Path $run 'transparent.png';$alpha2=Join-Path $run 'transparent-copy.png'
Add-Type -AssemblyName System.Drawing
$bitmap=[Drawing.Bitmap]::new(64,64,[Drawing.Imaging.PixelFormat]::Format32bppArgb)
try{for($y=0;$y -lt 64;$y++){for($x=0;$x -lt 64;$x++){$bitmap.SetPixel($x,$y,[Drawing.Color]::FromArgb(128,255,0,0))}};$bitmap.Save($alpha,[Drawing.Imaging.ImageFormat]::Png)}finally{$bitmap.Dispose()}
Copy-Item -LiteralPath $alpha -Destination $alpha2
$hashes=@{};foreach($path in @($a,$b,$c,$large,$bad,$left,$right,$gif,$gif2,$video)){$hashes[$path]=(Get-FileHash -LiteralPath $path).Hash}
foreach($path in @($alpha,$alpha2)){$hashes[$path]=(Get-FileHash -LiteralPath $path).Hash}
try{
    $start=[Diagnostics.ProcessStartInfo]::new($exe);$start.Arguments=$AgentArguments;$start.UseShellExecute=$false;$start.CreateNoWindow=$true;$start.RedirectStandardInput=$true;$start.RedirectStandardOutput=$true;$start.RedirectStandardError=$true;$start.StandardInputEncoding=[Text.UTF8Encoding]::new($false);$start.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
    $process=[Diagnostics.Process]::Start($start)
    $cap=Invoke-App @{command='capabilities'};Assert ($cap.commands -contains 'compare' -and $cap.commands -contains 'comparison') 'Real native app advertises comparison commands'
    $state=Compare-Files @($a,$b)
    $root=Invoke-App @{command='status'};Assert ($root.phase -eq 'ready' -and $root.content.kind -eq 'comparison' -and $root.pinned) 'Shared CLI status identifies the retained comparison as the actual active native preview'
    Assert ($state.visible -and -not $state.loading -and $state.panes.Count -eq 2 -and $state.panes[0].path -eq $a -and $state.panes[1].path -eq $b) 'Two native panes preserve explicit source order and settle'
    Assert ($state.panes[0].phase -eq 'ready' -and $state.panes[1].phase -eq 'ready' -and $state.pair.decoded_frames_compared -eq 1) 'Both LPAC image previews decode and compare actual frames'
    Assert ($state.pair.changed_pixels -eq 400 -and $state.pair.rgb_changed_pixels -eq 400 -and $state.pair.alpha_changed_pixels -eq 0) 'Decoded pixel comparison finds the exact changed block without inventing alpha changes'
    Assert ($state.pair.byte_scan.state -eq 'completed' -and $state.pair.byte_scan.changed_positions -eq 400) 'Independent byte scan reports exact differing byte positions'
    Assert ($state.controls.Count -eq 10 -and $state.controls[0].visible) 'Comparison has real native keyboard-focusable controls and pane buttons'
    $tip=Control 'tooltip' @{pane=1};Assert ($tip.source -eq 'native_tooltip_control' -and $tip.text.Contains($b) -and $tip.text.Contains('decoded preview frames')) 'Native tooltip callback exposes the actual source path and truthful preview stage'
    Sample-Idle 'settled_static_comparison'
    $state=Control 'capture' @{path=(Join-Path $run 'originals.bmp')};Assert ((Get-Item -LiteralPath (Join-Path $run 'originals.bmp')).Length -gt 10000) 'Actual native comparison painter and controls export a bounded client bitmap'
    $state=Control 'difference' @{value=$true};$null=Control 'capture' @{path=(Join-Path $run 'differences.bmp')}
    $mask=[IO.File]::ReadAllBytes((Join-Path $run 'differences.bmp'));$pink=0;for($i=54;$i -lt $mask.Length-3;$i+=4){if($mask[$i] -gt 110 -and $mask[$i+1] -lt 120 -and $mask[$i+2] -gt 210){$pink++}}
    Assert ($pink -gt 40) 'Difference highlights are present in pixels rendered by the native window'
    $null=Control 'difference' @{value=$false};$null=Control 'zoom' @{mode='native'};$state=Control 'pan' @{dx=25;dy=10}
    Assert ($state.panes[0].zoom.scale -eq 1 -and $state.panes[1].zoom.scale -eq 1 -and $state.panes[0].zoom.pan_x -eq $state.panes[1].zoom.pan_x -and $state.panes[1].zoom.pan_x -eq 25) 'Linked native image panes use the same actual pixel scale and pan'
    $state=Control 'focus' @{target='content'};Assert ($state.content_focused) 'Comparison content receives real native keyboard focus'
    $null=Control 'key' @{key='right'};$state=Control 'status';Assert ([Math]::Abs($state.panes[1].zoom.pan_x+15) -lt 0.001 -and [Math]::Abs($state.panes[0].zoom.pan_x+15) -lt 0.001) 'Owned Win32 arrow-key message pans linked content through the native message loop'
    $null=Control 'key' @{key='left'};$state=Control 'invoke' @{control='link'};Assert (-not $state.linked) 'Native Link button invocation reaches its actual window command handler'
    $null=Control 'focus' @{target='pane'};$null=Control 'key' @{key='tab'};$state=Control 'status';Assert ($state.content_focused) 'Native Tab navigation reaches interactive content after the pane controls'
    $state=Control 'pan' @{dx=10;dy=0}
    Assert ($state.panes[0].zoom.pan_x -eq 25 -and $state.panes[1].zoom.pan_x -eq 35) 'Unlink lets the active image move independently'
    $null=Control 'key' @{key='reload'};$state=(Invoke-App @{command='wait'}).comparison
    Assert ($state.panes[1].zoom.pan_x -eq 35 -and $state.panes[0].zoom.pan_x -eq 25 -and -not $state.linked) 'Comparison reload preserves independent compatible views and link state'
    $null=Control 'minimize';$null=Control 'reload';$state=(Invoke-App @{command='wait'}).comparison
    Assert ($state.minimized -and $state.panes[1].zoom.pan_x -eq 35) 'Reload while minimized preserves the last real comparison viewport and pan'
    $state=Control 'restore';Assert ($state.panes[1].zoom.pan_x -eq 35) 'Restoring the native comparison retains its image view'
    $renamed=Join-Path $run 'temporarily-renamed.bmp';Move-Item -LiteralPath $b -Destination $renamed
    Assert (Test-Path -LiteralPath $renamed) 'Source can be renamed while comparison is open'
    $null=Control 'reload';$state=(Invoke-App @{command='wait'}).comparison
    Assert ($state.panes[1].phase -eq 'error' -and $state.panes[0].phase -eq 'ready') 'Missing source has an independent error without hiding the other file'
    Move-Item -LiteralPath $renamed -Destination $b;$null=Control 'reload';$state=(Invoke-App @{command='wait'}).comparison
    Assert ($state.panes[1].phase -eq 'ready' -and $state.panes[1].zoom.pan_x -eq 35) 'Retry after missing-file error restores the last good comparison view'
    $state=Control 'resize' @{width=600;height=700};Assert ($state.client_width -lt 700 -and @($state.panes|Where-Object {$_.body.bottom -le $_.body.top}).Count -eq 0) 'Compact native layout wraps controls and retains positive content viewports'
    $state=Compare-Files @($a,$c)
    Assert ($state.pair.changed_pixels -eq 0 -and $state.pair.byte_scan.changed_positions -eq 1) 'Matching decoded pixels are distinguished from differing file bytes'
    $state=Compare-Files @($a,$large)
    Assert ($state.pair.decoded_frames_compared -eq 0 -and $state.pair.note -like '*dimensions*') 'Different dimensions produce a precise unresampled pixel-difference fallback'
    $null=Control 'link' @{value=$true};$null=Control 'zoom' @{mode='native'};$state=Control 'pan' @{dx=30;dy=20}
    Assert ($state.panes[0].zoom.scale -eq $state.panes[1].zoom.scale -and [Math]::Abs($state.panes[0].zoom.pan_x/800-$state.panes[1].zoom.pan_x/1000) -lt 0.001) 'Linked views normalize image center across different image dimensions'
    $state=Compare-Files @($left,$right)
    Assert ($state.pair.aligned_rows -eq 3 -and $state.pair.changed_rows -eq 1) 'Displayed text aligns an insertion while preserving unchanged following lines'
    $detail=Control 'inspect' @{offset=0;limit=3};Assert ($detail.panes[1].content.lines[1] -eq 'two') 'Bounded CLI inspection returns actual displayed comparison text'
    $state=Compare-Files @($a,$b,$bad,$left)
    Assert ($state.panes.Count -eq 4 -and $state.panes[2].phase -eq 'error' -and $state.panes[3].phase -eq 'ready') 'Four-pane mixed comparison isolates corrupt input and retains source order'
    $tip=Control 'tooltip' @{pane=2};Assert ($tip.text.Contains($bad) -and -not $tip.text.Contains($b)) 'Replacement tooltip identifies the current failed source without retaining the previous file'
    $state=Control 'resize' @{width=500;height=400};Assert (@($state.panes|Where-Object {$_.body.bottom -le $_.body.top -or $_.body.right -le $_.body.left}).Count -eq 0) 'Short four-pane native layout retains positive content viewports'
    Assert ($state.resident_payload_bytes -le $state.limits.resident_payload_bytes) 'Observed retained payload stays inside the shared comparison budget'
    $error=Invoke-App @{command='compare';paths=@($a,$b,$c,$large,$left)} -AllowError
    Assert (-not $error.ok) 'More than four requested panes is rejected before replacing a valid comparison'
    $error=Invoke-App @{command='compare';paths=@($a,$a)} -AllowError;Assert (-not $error.ok) 'Duplicate file identities are rejected'
    $state=Control 'status';Assert ($state.panes.Count -eq 4) 'Invalid commands preserve the existing valid comparison'
    $state=Compare-Files @($gif,$gif2)
    Assert (-not $state.playing -and -not $state.animation_running -and $state.panes[0].content.frames -eq 2 -and $state.pair.timing_equal) 'Animated comparisons load real frames paused without a render timer and compare timing metadata'
    $state=Control 'frame' @{index=1};Assert ($state.panes[0].frame -eq 1 -and $state.panes[1].frame -eq 1) 'Explicit frame selection synchronizes both decoded animations'
    Sample-Idle 'paused_animated_comparison'
    $state=Control 'play' @{value=$true};Start-Sleep -Milliseconds 260;$state=Control 'status'
    Assert ($state.playing -and $state.animation_running -and $state.panes[0].frame -eq $state.panes[1].frame) 'Explicit playback uses a shared native animation timeline'
    $null=Control 'play' @{value=$false};$state=Control 'minimize';Assert ($state.minimized) 'Comparison minimizes its real native window'
    Sample-Idle 'minimized_comparison'
    $error=Invoke-App @{command='comparison';action='capture';path=(Join-Path $run 'forbidden-minimized.bmp')} -AllowError;Assert (-not $error.ok -and -not(Test-Path -LiteralPath (Join-Path $run 'forbidden-minimized.bmp'))) 'Minimized comparison capture leaves output untouched'
    $state=Control 'restore';Assert (-not $state.minimized) 'Comparison restores its real native window'
    $state=Compare-Files @($alpha,$alpha2);$null=Control 'zoom' @{mode='native'};$state=Control 'status';$null=Control 'capture' @{path=(Join-Path $run 'alpha-composite.bmp')}
    $composite=[IO.File]::ReadAllBytes((Join-Path $run 'alpha-composite.bmp'));$width=[BitConverter]::ToInt32($composite,18)
    $pane=$state.panes[1];$x=[int]($pane.body.left+$pane.zoom.viewport_width/2);$y=[int]($pane.body.top+$pane.zoom.viewport_height/2);$pixel=54+($y*$width+$x)*4
    Assert ($composite[$pixel+2] -gt 135 -and $composite[$pixel+2] -lt 165 -and $composite[$pixel] -gt 10 -and $composite[$pixel] -lt 35) 'Actual native painter composites premultiplied transparent pixels over its visible checkerboard'
    $gdiBefore=[HoverComparisonResources]::GetGuiResources($process.Handle,0)
    for($i=0;$i -lt 8;$i++){$null=Control 'capture' @{path=(Join-Path $run "alpha-repeat-$i.bmp")}}
    $gdiAfter=[HoverComparisonResources]::GetGuiResources($process.Handle,0)
    Assert ($gdiAfter -le $gdiBefore+1) 'Repeated transparent-frame and control captures release their temporary native GDI surfaces'
    if($DbpfFixture){
        $package=(Resolve-Path -LiteralPath $DbpfFixture).Path;$copy=Join-Path $run 'same-index.package';Copy-Item -LiteralPath $package -Destination $copy
        $hashes[$package]=(Get-FileHash -LiteralPath $package).Hash;$hashes[$copy]=$hashes[$package]
        Assert ($hashes[$package] -eq '63466BD02E9FEBFD6B02D823D2BEF4FF301167D94A1A9B1657AF7DCCCB0251C2') 'DBPF comparison fixture matches reviewed pinned public upstream bytes'
        $state=Compare-Files @($package,$copy)
        Assert ($state.pair.changed_rows -eq 0 -and $state.pair.byte_scan.changed_positions -eq 0 -and $state.panes[0].content.kind -eq 'text') 'Two actual DBPF indexes compare as displayed metadata without claiming image or reconstructed asset fidelity'
    }
    $state=Compare-Files @($video,$left);$single=Invoke-App @{command='status'}
    Assert ($state.panes[0].content.subtitle -eq 'Media recognized' -and -not $single.video_running -and -not $single.audio_running) 'Comparison recognizes media without starting an external player'
    $state=Compare-Files @('\\example.invalid\share\never-read.bmp',$left)
    Assert ($state.panes[0].phase -eq 'error' -and $state.panes[0].error -like '*Network and device*' -and $state.panes[1].phase -eq 'ready') 'Automatic comparison rejects UNC input before network access while the local pane still loads'
    for($i=0;$i -lt 24;$i++){$null=Invoke-App @{command='compare';paths=$(if($i%2){@($a,$b)}else{@($left,$right)})}}
    $state=Compare-Files @($left,$right)
    Assert ($state.panes[0].path -eq $left -and $state.panes[1].path -eq $right -and $state.panes[0].content.kind -eq 'text') 'Rapid selection replacement discards stale image and text generations'
    $null=Control 'close';Start-Sleep -Milliseconds 150;$status=Invoke-App @{command='status'}
    Assert ($null -eq $status.comparison -and -not $status.loading) 'Closed comparison cannot reopen from a late worker result'
    $null=Compare-Files @($left,$right);$null=Control 'invoke' @{control='single'};$status=Invoke-App @{command='wait'}
    Assert ($status.phase -eq 'ready' -and $status.path -eq $right -and $null -eq $status.comparison) 'Native Open alone control returns the selected pane to the preserved single-file viewer'
    foreach($path in $hashes.Keys){Assert ((Get-FileHash -LiteralPath $path).Hash -eq $hashes[$path]) "Source remains byte-identical: $([IO.Path]::GetFileName($path))"}
    $null=Invoke-App @{command='open';path=$left};$status=Invoke-App @{command='wait'}
    Assert ($status.phase -eq 'ready' -and $status.path -eq $left -and $null -eq $status.comparison) 'Single-file preview remains functional after comparison sessions'
    $null=Invoke-App @{command='quit'};Assert ($process.WaitForExit(5000) -and $process.ExitCode -eq 0) 'Comparison and single-file windows close with their owned native process'
    $passed=$true
}finally{
    if($process -and -not $process.HasExited){$process.StandardInput.Close();if(-not $process.WaitForExit(5000)){$process.Kill();$process.WaitForExit()}}
    [ordered]@{schemaVersion=1;passed=$passed;execution='actual native Windows app via private stdio, own Win32 windows/controls and painter';fixtures='generated BMP/PNG/GIF/text/media/corrupt inputs; optional pinned DBPF index fixture; no game-fidelity or physical Explorer claim';timingScope='JSON/native-state or client-export round trips; OS caches uncontrolled, not storage-cold or compositor/input timing';resourceScope='Short owned-parent CPU, endpoint memory/handles/GDI samples; not concurrent peak, child/GPU/VRAM or long-run stability';executableSha256=(Get-FileHash -LiteralPath $exe).Hash;assertions=$checks.Count;checks=$checks;responses=$responses;timings=$timings;idleSamples=$idle}|ConvertTo-Json -Depth 22|Set-Content -LiteralPath (Join-Path $run 'result.json') -Encoding utf8
    Write-Output "Comparison evidence: $run";Write-Output "Passed checks: $($checks.Count); success: $passed"
}
if($ReferenceChecks){& (Join-Path $PSScriptRoot 'compare-comparison-references.ps1') -EvidenceDirectory $run | Out-Null;if($LASTEXITCODE -ne 0){throw 'Independent comparison reference failed'}}
exit 0
