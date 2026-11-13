param(
    [Parameter(Mandatory)][string]$PackageA,
    [Parameter(Mandatory)][string]$PackageB,
    [string]$DbpfFixture='',
    [string]$OutputDirectory=(Join-Path $PSScriptRoot '../target/installation-evidence')
)
$ErrorActionPreference='Stop'
Add-Type -TypeDefinition @'
using System;using System.Runtime.InteropServices;
public static class HoverInstallNative {
 [DllImport("msi.dll",CharSet=CharSet.Unicode)]public static extern int MsiQueryProductState(string product);
 [DllImport("shell32.dll",CharSet=CharSet.Unicode)]public static extern uint ExtractIconEx(string file,int index,IntPtr large,IntPtr small,uint count);
}
'@
$a=Get-Content -LiteralPath (Join-Path $PackageA 'package.json') -Raw | ConvertFrom-Json
$b=Get-Content -LiteralPath (Join-Path $PackageB 'package.json') -Raw | ConvertFrom-Json
if($a.installLeaf -notmatch '^Hover-Test-[0-9a-f]{12}$' -or $a.installLeaf -ne $b.installLeaf -or $a.upgradeCode -ne $b.upgradeCode -or [version]$a.version -ge [version]$b.version){throw 'Use two ordered development versions with one unique Hover-Test-<12 hex> scope'}
foreach($pair in @(@($PackageA,$a),@($PackageB,$b))){
    $path=Join-Path $pair[0] $pair[1].package
    if((Get-FileHash -LiteralPath $path).Hash -ne $pair[1].sha256){throw 'MSI artifact hash mismatch'}
    $validation=Get-Content -LiteralPath (Join-Path $pair[0] 'ice-validation.json') -Raw | ConvertFrom-Json
    if($validation.Errors -ne 0 -or -not $validation.Completed){throw 'SDK ICE validation must complete with zero errors before installation'}
}
$target=Join-Path $env:LOCALAPPDATA "Programs\$($a.installLeaf)"
$link=Join-Path ([Environment]::GetFolderPath('Programs')) "$($a.installLeaf)\Hover Development.lnk"
if(Test-Path -LiteralPath $target){throw 'Installation test requires a fresh exact target'}
if([HoverInstallNative]::MsiQueryProductState($a.productCode) -eq 5 -or [HoverInstallNative]::MsiQueryProductState($b.productCode) -eq 5){throw 'Test product is already installed'}
$run=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ('session-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $run -Force | Out-Null
$checks=[Collections.Generic.List[string]]::new();$transactions=[Collections.Generic.List[object]]::new()
$exe=Join-Path $target 'hover.exe';$cli=Join-Path $target 'hover-agent.exe'
$marker=Join-Path $target 'user-added-preserve.txt';$markerHash=$null
function Assert([bool]$ok,[string]$name){if(-not $ok){throw "FAILED: $name"};$checks.Add($name)}
function Install([string]$Msi,[string]$Name,[string[]]$Extra=@(),[int]$Expected=0){
    $log=Join-Path $run "$Name.log"
    $arguments=@('/i',('"'+[IO.Path]::GetFullPath($Msi)+'"'),'/qn','/norestart','REBOOT=ReallySuppress','/l*v',('"'+$log+'"'))+$Extra
    $p=Start-Process -FilePath "$env:WINDIR\System32\msiexec.exe" -ArgumentList $arguments -WindowStyle Hidden -PassThru
    if(-not $p.WaitForExit(60000)){throw "MSI transaction $Name is still running; inspect before retrying"}
    $transactions.Add([ordered]@{name=$Name;exitCode=$p.ExitCode;expected=$Expected;log=[IO.Path]::GetFileName($log)})
    Assert ($p.ExitCode -eq $Expected) "MSI $Name returns expected $Expected (actual $($p.ExitCode))"
}
function Check-Installed($Manifest){
    Assert ([HoverInstallNative]::MsiQueryProductState($Manifest.productCode) -eq 5) "Windows Installer registers version $($Manifest.version)"
    foreach($file in $Manifest.files){Assert ((Get-FileHash -LiteralPath (Join-Path $target $file.name)).Hash -eq $file.sha256) "Installed $($Manifest.version) $($file.name) matches packaged SHA-256"}
    $identity=& $cli --version-json | ConvertFrom-Json
    Assert ($LASTEXITCODE -eq 0 -and $identity.packageVersion -eq $Manifest.version) "Installed CLI reports $($Manifest.version)"
    if($markerHash){Assert ((Get-FileHash -LiteralPath $marker).Hash -eq $markerHash) 'Transaction preserves user-added file byte-for-byte'}
}
function Check-Shortcut {
    Assert (Test-Path -LiteralPath $link -PathType Leaf) 'Windows Installer creates Start Menu shortcut'
    $shell=New-Object -ComObject WScript.Shell;$shortcut=$shell.CreateShortcut($link)
    try{
        Assert ($shortcut.TargetPath -eq $exe -and $shortcut.WorkingDirectory.TrimEnd('\') -eq $target) 'Start Menu shortcut points to stable installed executable and working directory'
        $icon=($shortcut.IconLocation -split ',')[0]
        Assert ([HoverInstallNative]::ExtractIconEx($icon,-1,[IntPtr]::Zero,[IntPtr]::Zero,0) -gt 0) 'Installed shortcut icon resource can be extracted by Windows'
    }finally{[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shortcut);[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell)}
    $shell=New-Object -ComObject Shell.Application;$folder=$shell.NameSpace((Split-Path $link));$item=$folder.ParseName((Split-Path $link -Leaf))
    try{Assert ($item.ExtendedProperty('System.AppUserModel.ID') -eq 'Herbertofury.Hover') 'Installed shortcut carries the same AppUserModelID as native windows'}finally{foreach($object in @($item,$folder,$shell)){[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($object)}}
}
function Exercise-Shortcut {
    $config=Join-Path $run 'shortcut-appdata/Hover';New-Item -ItemType Directory -Path $config -Force | Out-Null
    [IO.File]::WriteAllText((Join-Path $config 'settings.ini'),"[general]`nenabled = false`nshow_tray_icon = true`n")
    $savedAppData=$env:APPDATA
    try{
        $env:APPDATA=Split-Path $config
        $start=[Diagnostics.ProcessStartInfo]::new($link);$start.UseShellExecute=$true
        $process=[Diagnostics.Process]::Start($start)
    }finally{$env:APPDATA=$savedAppData}
    $status=$null
    for($n=0;$n -lt 100;$n++){$status=& $cli --resident-status | ConvertFrom-Json;if($LASTEXITCODE -ne 0){throw 'Shortcut status failed'};if($status.running){break};Start-Sleep -Milliseconds 50}
    Assert ([bool]$status.running) 'Windows Shell launches the installed Start Menu shortcut as a normal resident'
    $process=Get-Process -Id $status.pid
    try{
        $reply=& $cli --shutdown | ConvertFrom-Json
        Assert ($LASTEXITCODE -eq 0 -and $reply.sent -and $process.WaitForExit(5000)) 'Shortcut-launched resident accepts graceful first-party shutdown'
    }finally{if(-not $process.HasExited){throw 'Shortcut-launched resident remains active; inspect before installer operations'}}
}
function Exercise-App {
    & (Join-Path $PSScriptRoot 'test-resident.ps1') -Executable $exe -OutputDirectory (Join-Path $run 'resident') | Out-Null
    if($LASTEXITCODE -ne 0){throw 'Installed resident lifecycle failed'}
    $agentParams=@{Executable=$exe;OutputDirectory=(Join-Path $run 'agent')}
    if($DbpfFixture){$agentParams.DbpfFixture=$DbpfFixture}
    & (Join-Path $PSScriptRoot 'test-agent.ps1') @agentParams | Out-Null
    if($LASTEXITCODE -ne 0){throw 'Installed native GUI agent checks failed'}
    Assert $true 'Installed GUI passes native preview, cancellation, file-lock, LPAC decoder and lifecycle suite'
    $agentParams.Executable=$cli;$agentParams.AgentArguments='--stdio'
    & (Join-Path $PSScriptRoot 'test-agent.ps1') @agentParams | Out-Null
    if($LASTEXITCODE -ne 0){throw 'Installed console agent checks failed'}
    Assert $true 'Installed console agent passes the same native preview suite'
}
$msiA=Join-Path $PackageA $a.package;$msiB=Join-Path $PackageB $b.package
$success=$false
try{
    Install $msiA 'install-a';Check-Installed $a;Check-Shortcut;Exercise-Shortcut;Exercise-App
    [IO.File]::WriteAllText($marker,'User-owned fixture; installer must preserve this.');$markerHash=(Get-FileHash -LiteralPath $marker).Hash
    # Deliberately malformed CAB negative fixture. This is a direct MSI
    # transaction test, not an authenticated updater accepting a changed hash.
    $fault=Join-Path $run 'broken-upgrade';New-Item -ItemType Directory -Path $fault | Out-Null
    $ddf=@('.OPTION EXPLICIT','.Set CabinetNameTemplate=Broken.cab','.Set DiskDirectoryTemplate=.','.Set CompressionType=MSZIP','.Set Cabinet=on','.Set Compress=on')
    $ids=@{ 'hover-agent.exe'='AgentExe';'hover-browser-host.exe'='BrowserExe';'LICENSE'='License';'THIRD-PARTY-NOTICES.md'='Notices';'DEPENDENCY-LICENSES.txt'='DependencyLicenses';'BUILD-INFO.json'='BuildInfo' }
    foreach($file in $b.files){$name=$file.name;if($ids.ContainsKey($name)){$ddf+='"'+[IO.Path]::GetFullPath((Join-Path $PackageB "payload/$name"))+'" '+$ids[$name]}}
    [IO.File]::WriteAllLines((Join-Path $fault 'broken.ddf'),$ddf,[Text.Encoding]::ASCII)
    Push-Location $fault
    try{& "$env:WINDIR\System32\makecab.exe" /f broken.ddf | Out-Null;if($LASTEXITCODE -ne 0){throw 'Could not create missing-file CAB fixture'}}finally{Pop-Location}
    $broken=Join-Path $fault 'broken.msi';Copy-Item -LiteralPath $msiB -Destination $broken
    $installer=New-Object -ComObject WindowsInstaller.Installer;$database=$null;$view=$null;$record=$null;$summary=$null
    try{
        $database=$installer.GetType().InvokeMember('OpenDatabase','InvokeMethod',$null,$installer,@([string]$broken,1))
        $view=$database.GetType().InvokeMember('OpenView','InvokeMethod',$null,$database,@('UPDATE `_Streams` SET `Data`=? WHERE `Name`=?'))
        $record=$installer.GetType().InvokeMember('CreateRecord','InvokeMethod',$null,$installer,@(2))
        $record.GetType().InvokeMember('SetStream','InvokeMethod',$null,$record,@(1,[string](Join-Path $fault 'Broken.cab'))) | Out-Null
        $record.GetType().InvokeMember('StringData','SetProperty',$null,$record,@(2,'Hover.cab')) | Out-Null
        $view.GetType().InvokeMember('Execute','InvokeMethod',$null,$view,@($record)) | Out-Null
        $summary=$database.GetType().InvokeMember('SummaryInformation','GetProperty',$null,$database,@(1))
        $summary.GetType().InvokeMember('Property','SetProperty',$null,$summary,@(9,('{'+[guid]::NewGuid().ToString().ToUpperInvariant()+'}'))) | Out-Null
        $summary.GetType().InvokeMember('Persist','InvokeMethod',$null,$summary,$null) | Out-Null
        $database.GetType().InvokeMember('Commit','InvokeMethod',$null,$database,$null) | Out-Null
    }finally{foreach($object in @($summary,$record,$view,$database,$installer)){if($null -ne $object){[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($object)}}}
    Install $broken 'failed-upgrade-rolls-back-a' @() 1603
    $failureLog=[IO.File]::ReadAllText((Join-Path $run 'failed-upgrade-rolls-back-a.log'))
    Assert ($failureLog -match '1334' -and $failureLog -match 'Rollback' -and $failureLog -notmatch 'Action start [^\r\n]*RemoveExistingProducts') 'Missing-file upgrade rolls back before removing the existing product'
    Check-Installed $a;Assert ([HoverInstallNative]::MsiQueryProductState($b.productCode) -ne 5) 'Failed upgrade leaves newer product unregistered';Exercise-App
    Install $msiB 'upgrade-b';Check-Installed $b;Check-Shortcut
    Assert ([HoverInstallNative]::MsiQueryProductState($a.productCode) -ne 5) 'Successful upgrade removes old product registration';Exercise-App
    Install $msiA 'downgrade-denied' @() 1603;Check-Installed $b
    Install $msiA 'incomplete-rollback-denied' @('HOVER_ALLOW_ROLLBACK=1') 1603;Check-Installed $b
    Install $msiA 'explicit-local-rollback-a' @('HOVER_ALLOW_ROLLBACK=1','REINSTALLMODE=amus');Check-Installed $a;Check-Shortcut;Exercise-App
    Assert ([HoverInstallNative]::MsiQueryProductState($b.productCode) -ne 5) 'Explicit rollback removes newer product registration'
    $success=$true
}finally{
    # Uninstall only these two exact test product codes through Windows
    # Installer. User-added files and raw evidence remain for inspection.
    foreach($product in @($a.productCode,$b.productCode)){
        if([HoverInstallNative]::MsiQueryProductState($product) -in @(1,5)){
            $p=Start-Process -FilePath "$env:WINDIR\System32\msiexec.exe" -ArgumentList @('/x',$product,'/qn','/norestart','/l*v',('"'+(Join-Path $run 'uninstall.log')+'"')) -WindowStyle Hidden -PassThru
            if(-not $p.WaitForExit(60000) -or $p.ExitCode -ne 0){throw 'Owned test MSI uninstall failed; inspect retained installation'}
        }
    }
    if($success){
        Assert (-not(Test-Path -LiteralPath $exe) -and -not(Test-Path -LiteralPath $link)) 'Uninstall removes managed executable and shortcut'
        Assert ((Get-FileHash -LiteralPath $marker).Hash -eq $markerHash) 'Uninstall preserves user-added file byte-for-byte'
        $result=[ordered]@{schemaVersion=1;passed=$true;scope='Actual Windows MSI install, native installed-app preview/restart, same-source versioned upgrade, transaction failure, downgrade guard, explicit local rollback, uninstall. Unsigned development fixtures; no network updater, Explorer input, taskbar pin or release qualification claim.';assertions=$checks.Count;checks=$checks;transactions=$transactions;packageA=$a.sha256;packageB=$b.sha256;sourceA=$a.build.sourceRevision;sourceB=$b.build.sourceRevision}
        $result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $run 'result.json') -Encoding utf8
        Write-Host "Installation test passed ($($checks.Count) checks); retained evidence: $run"
    }
}
exit 0
