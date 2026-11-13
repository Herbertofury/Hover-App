param(
    [string]$Executable = (Join-Path $PSScriptRoot '../target/release/hover.exe'),
    [ValidateRange(1,8)][int]$Sessions = 3,
    [ValidateRange(1,64)][int]$WarmSamples = 16,
    [ValidateRange(1000,30000)][int]$IdleMilliseconds = 3000,
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '../target/performance-evidence'),
    [switch]$Json
)
$ErrorActionPreference = 'Stop'
$executablePath = (Resolve-Path -LiteralPath $Executable).Path
$runDirectory = Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ('session-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
$imagePath = Join-Path $runDirectory 'generated-800x600.bmp'
$pixels = [byte[]]::new(800*600*4)
for ($i=0; $i -lt $pixels.Length; $i+=4) {
    $pixels[$i]=20; $pixels[$i+1]=180; $pixels[$i+2]=230; $pixels[$i+3]=255
}
$writer = [IO.BinaryWriter]::new([IO.File]::Open($imagePath,[IO.FileMode]::CreateNew))
try {
    $writer.Write([byte[]](66,77)); $writer.Write([uint32](54+$pixels.Length)); $writer.Write([uint32]0); $writer.Write([uint32]54)
    $writer.Write([uint32]40); $writer.Write([int32]800); $writer.Write([int32](-600)); $writer.Write([uint16]1); $writer.Write([uint16]32)
    $writer.Write([uint32]0); $writer.Write([uint32]$pixels.Length); $writer.Write([byte[]]::new(16)); $writer.Write($pixels)
} finally { $writer.Dispose() }
$fixtureHash=(Get-FileHash -LiteralPath $imagePath -Algorithm SHA256).Hash
$script:ownedProcess=$null
$script:sequence=0
$samples=[Collections.Generic.List[object]]::new()
$idle=[Collections.Generic.List[object]]::new()
$interactions=[Collections.Generic.List[object]]::new()
if (-not ('HoverMeasurements.NativeIo' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
namespace HoverMeasurements {
    [StructLayout(LayoutKind.Sequential)]
    public struct IoCounters {
        public ulong ReadOperations, WriteOperations, OtherOperations;
        public ulong ReadBytes, WriteBytes, OtherBytes;
    }
    public static class NativeIo {
        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        private static extern bool GetProcessIoCounters(IntPtr process, out IoCounters counters);
        public static IoCounters Read(IntPtr ownedProcess) {
            IoCounters result;
            if (!GetProcessIoCounters(ownedProcess, out result))
                throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
            return result;
        }
    }
}
'@
}

function Invoke-Probe([hashtable]$Command) {
    $script:sequence++
    $Command.id=$script:sequence
    $script:ownedProcess.StandardInput.WriteLine(($Command | ConvertTo-Json -Compress -Depth 8))
    $script:ownedProcess.StandardInput.Flush()
    $read=$script:ownedProcess.StandardOutput.ReadLineAsync()
    if (-not $read.Wait(15000)) { throw "No response to $($Command.command)" }
    if ($null-eq $read.Result) { throw 'Owned Hover exited before its reply' }
    $reply=$read.Result | ConvertFrom-Json
    if ($reply.protocol-ne 1 -or $reply.id-ne $script:sequence -or -not $reply.ok) {
        throw "Invalid or unsuccessful reply to $($Command.command)"
    }
    return $reply.result
}

function Measure-OwnedIdle([string]$Phase) {
    # No protocol requests or input are sent during this interval. Endpoint
    # process counters do not sample a peak or GPU/child-process working set.
    $script:ownedProcess.Refresh()
    $cpuBefore=$script:ownedProcess.TotalProcessorTime.TotalMilliseconds
    $privateBefore=$script:ownedProcess.PrivateMemorySize64
    $workingBefore=$script:ownedProcess.WorkingSet64
    $handlesBefore=$script:ownedProcess.HandleCount
    $ioBefore=[HoverMeasurements.NativeIo]::Read($script:ownedProcess.Handle)
    $clock=[Diagnostics.Stopwatch]::StartNew()
    if ($script:ownedProcess.WaitForExit($IdleMilliseconds)) { throw 'Hover exited during idle measurement' }
    $clock.Stop()
    $script:ownedProcess.Refresh()
    $cpuMs=$script:ownedProcess.TotalProcessorTime.TotalMilliseconds-$cpuBefore
    $ioAfter=[HoverMeasurements.NativeIo]::Read($script:ownedProcess.Handle)
    $idle.Add([ordered]@{
        phase=$Phase;wallMs=$clock.Elapsed.TotalMilliseconds;processCpuMs=$cpuMs
        cpuPercentOfOneLogicalProcessor=100*$cpuMs/$clock.Elapsed.TotalMilliseconds
        cpuPercentOfMachine=100*$cpuMs/$clock.Elapsed.TotalMilliseconds/[Environment]::ProcessorCount
        privateBytesBefore=$privateBefore;privateBytesAfter=$script:ownedProcess.PrivateMemorySize64
        workingSetBytesBefore=$workingBefore;workingSetBytesAfter=$script:ownedProcess.WorkingSet64
        handlesBefore=$handlesBefore;handlesAfter=$script:ownedProcess.HandleCount
        ioReadOperations=$ioAfter.ReadOperations-$ioBefore.ReadOperations;ioWriteOperations=$ioAfter.WriteOperations-$ioBefore.WriteOperations
        ioOtherOperations=$ioAfter.OtherOperations-$ioBefore.OtherOperations;ioReadBytes=$ioAfter.ReadBytes-$ioBefore.ReadBytes
        ioWriteBytes=$ioAfter.WriteBytes-$ioBefore.WriteBytes;ioOtherBytes=$ioAfter.OtherBytes-$ioBefore.OtherBytes
        processLifetimePeakWorkingSetBytes=$script:ownedProcess.PeakWorkingSet64
    })
}

function Assert-Ready($State) {
    if ($State.phase-ne 'ready' -or $State.path-ne $imagePath -or $State.content.kind-ne 'image' -or $State.content.image_width-ne 800 -or $State.content.image_height-ne 600 -or $State.decoder_process.active_pid-ne 0 -or $State.decoder_process.last_task.outcome-ne 'success') {
        throw 'Measurement did not produce the expected settled native image preview'
    }
}

try {
    for ($session=0; $session-lt $Sessions; $session++) {
        $startInfo=[Diagnostics.ProcessStartInfo]::new()
        $startInfo.FileName=$executablePath; $startInfo.Arguments='--agent-stdio'
        $startInfo.UseShellExecute=$false; $startInfo.CreateNoWindow=$true
        $startInfo.WindowStyle=[Diagnostics.ProcessWindowStyle]::Hidden
        $startInfo.RedirectStandardInput=$true; $startInfo.RedirectStandardOutput=$true; $startInfo.RedirectStandardError=$true
        $startInfo.StandardInputEncoding=[Text.UTF8Encoding]::new($false)
        $startInfo.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
        $script:ownedProcess=[Diagnostics.Process]::new(); $script:ownedProcess.StartInfo=$startInfo
        $startup=[Diagnostics.Stopwatch]::StartNew()
        if (-not $script:ownedProcess.Start()) { throw 'Could not start owned Hover' }
        $capabilities=Invoke-Probe @{command='capabilities'}
        $startup.Stop()
        if (-not $capabilities.owns_session) { throw 'Expected an owned agent session' }
        $startupMs=$startup.Elapsed.TotalMilliseconds
        if ($session-eq 0) { Measure-OwnedIdle 'dormant_controlled_session' }
        for ($iteration=0; $iteration-le $WarmSamples; $iteration++) {
            $readyClock=[Diagnostics.Stopwatch]::StartNew()
            $ack=if ($iteration-eq 0) { Invoke-Probe @{command='open';path=$imagePath;pin=$true;activate=$false} } else { Invoke-Probe @{command='reload'} }
            $ackMs=$readyClock.Elapsed.TotalMilliseconds
            if ($ack.path-ne $imagePath -and $ack.pending_path-ne $imagePath) { throw 'Acknowledgment has the wrong identity' }
            $state=Invoke-Probe @{command='wait';timeout_ms=10000}
            $readyClock.Stop()
            Assert-Ready $state
            $script:ownedProcess.Refresh()
            $samples.Add([ordered]@{
                session=$session;iteration=$iteration;mode=$(if($iteration-eq 0){'new_process_first_open'}else{'warm_process_reload'})
                processStartupToCapabilitiesMs=$startupMs;openOrReloadAckMs=$ackMs;commandToSettledResponseMs=$readyClock.Elapsed.TotalMilliseconds
                parentPrivateBytes=$script:ownedProcess.PrivateMemorySize64;parentWorkingSetBytes=$script:ownedProcess.WorkingSet64
                parentHandles=$script:ownedProcess.HandleCount
                decoder=$state.decoder_process.last_task
            })
        }
        if ($session-eq 0) {
            $null=Invoke-Probe @{command='resize';width=500;height=400}
            $state=Invoke-Probe @{command='zoom';mode='native'}
            $previousPanX=$state.content.pan_x
            for($i=0;$i-lt $WarmSamples;$i++) {
                $dx=if($i%2){-5}else{5}
                $clock=[Diagnostics.Stopwatch]::StartNew()
                $state=Invoke-Probe @{command='pan';dx=$dx;dy=0}
                $clock.Stop()
                if ($state.content.scale-ne 1 -or [Math]::Abs($state.content.pan_x-$previousPanX-$dx)-gt 0.01) { throw 'Interaction did not retain native zoom and requested pan' }
                $previousPanX=$state.content.pan_x
                $interactions.Add([ordered]@{command='pan';roundTripMs=$clock.Elapsed.TotalMilliseconds;panX=$state.content.pan_x;panY=$state.content.pan_y})
            }
            Measure-OwnedIdle 'pinned_static_image'
            $null=Invoke-Probe @{command='minimize'}
            Measure-OwnedIdle 'minimized_static_image'
        }
        $null=Invoke-Probe @{command='quit'}
        if (-not $script:ownedProcess.WaitForExit(4000) -or $script:ownedProcess.ExitCode-ne 0) { throw 'Owned Hover did not shut down successfully' }
        $script:ownedProcess.Dispose(); $script:ownedProcess=$null
    }
    if ((Get-FileHash -LiteralPath $imagePath -Algorithm SHA256).Hash-ne $fixtureHash) { throw 'Fixture bytes changed' }
    $cpu=Get-CimInstance Win32_Processor | Select-Object -First 1
    $os=Get-CimInstance Win32_OperatingSystem
    $evidence=[ordered]@{
        schemaVersion=1;measuredAtUtc=[DateTime]::UtcNow.ToString('o');passed=$true
        executableName=[IO.Path]::GetFileName($executablePath);executableSha256=(Get-FileHash -LiteralPath $executablePath -Algorithm SHA256).Hash
        fixture=[ordered]@{classification='generated synthetic BMP, 800x600';sha256=$fixtureHash;byteIdentical=$true}
        environment=[ordered]@{os=$os.Caption;version=$os.Version;build=$os.BuildNumber;architecture=$os.OSArchitecture;cpu=$cpu.Name;logicalProcessors=[Environment]::ProcessorCount}
        cache='OS caches uncontrolled; generated fixture was just written; new process does not mean storage-cold'
        timing='External stopwatch; JSON command/response plus real native UI-thread/worker scheduling. Wait polls at 20 ms. Ack verifies file identity/phase, not pixels painted to screen.'
        resourceScope='Owned parent CPU/private bytes/working set/handles and GetProcessIoCounters deltas; decoder job accounting separately. I/O includes pipes and cached operations, not physical-disk activity. CPU timer granularity applies; endpoint memory is not a concurrent peak.'
        excludes=@('Explorer hover','screen/compositor latency','cold-storage results','3D/GPU/VRAM','real game fidelity','long-run resource stability','installer/update/rollback')
        samples=$samples;idleIntervals=$idle;interactions=$interactions
    }
    $evidencePath=Join-Path $runDirectory 'measurements.json'
    $evidence | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $evidencePath -Encoding utf8
    if ($Json) { $evidence | ConvertTo-Json -Depth 20 } else {
        Write-Output "Performance evidence: $evidencePath"
        Write-Output "Passed $($samples.Count) image measurements, $($idle.Count) idle intervals and $($interactions.Count) interactions."
    }
} finally {
    if ($null-ne $script:ownedProcess) {
        try {
            if (-not $script:ownedProcess.HasExited) {
                $script:ownedProcess.StandardInput.Close()
                if (-not $script:ownedProcess.WaitForExit(4000)) { $script:ownedProcess.Kill(); $null=$script:ownedProcess.WaitForExit(4000) }
            }
        } finally { $script:ownedProcess.Dispose() }
    }
}
exit 0
