param(
    [Parameter(Mandatory)][string]$PayloadDirectory,
    [Parameter(Mandatory)][string]$Version,
    [Parameter(Mandatory)][string]$SdkDirectory,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [string]$InstallLeaf='Hover-Development',
    [guid]$UpgradeCode='A67BBD1A-3042-41D4-9702-936EF1696D7E'
)
$ErrorActionPreference='Stop'
if($Version -notmatch '^(\d+)\.(\d+)\.(\d+)$' -or [int]$Matches[1] -gt 255 -or [int]$Matches[2] -gt 255 -or [int]$Matches[3] -gt 65535){throw 'MSI version requires major/minor <=255, build <=65535'}
if($InstallLeaf -notmatch '^Hover-[A-Za-z0-9-]{1,60}$'){throw 'Install leaf must be a bounded Hover- development/test name'}
$payload=(Resolve-Path -LiteralPath $PayloadDirectory).Path
$sdk=(Resolve-Path -LiteralPath $SdkDirectory).Path
$output=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $output){throw 'Use a new output directory; artifacts are never overwritten'}
New-Item -ItemType Directory -Path $output | Out-Null
$files=@(
    @('HoverExe','hover.exe'),@('AgentExe','hover-agent.exe'),@('BrowserExe','hover-browser-host.exe'),
    @('License','LICENSE'),@('Notices','THIRD-PARTY-NOTICES.md'),@('DependencyLicenses','DEPENDENCY-LICENSES.txt'),@('BuildInfo','BUILD-INFO.json')
)
$actual=@(Get-ChildItem -LiteralPath $payload -Force)
if($actual.Count -ne $files.Count -or @($actual | Where-Object { $_.PSIsContainer -or $_.Name -notin $files.ForEach({$_[1]}) }).Count){throw 'Payload must contain exactly the reviewed allowlisted files'}
$identity=& (Join-Path $payload 'hover-agent.exe') --version-json | ConvertFrom-Json
if($LASTEXITCODE -ne 0 -or $identity.packageVersion -ne $Version -or $identity.releaseVerified){throw 'Payload build identity/version mismatch'}
$staged=Join-Path $output 'payload'; New-Item -ItemType Directory -Path $staged | Out-Null
foreach($file in $files){Copy-Item -LiteralPath (Join-Path $payload $file[1]) -Destination (Join-Path $staged $file[1])}
$ddf=@('.OPTION EXPLICIT','.Set CabinetNameTemplate=Hover.cab','.Set DiskDirectoryTemplate=.','.Set MaxDiskSize=0','.Set MaxCabinetSize=0','.Set CompressionType=MSZIP','.Set Cabinet=on','.Set Compress=on')
foreach($file in $files){$ddf+='"payload\'+$file[1]+'" '+$file[0]}
[IO.File]::WriteAllLines((Join-Path $output 'payload.ddf'),$ddf,[Text.Encoding]::ASCII)
Push-Location $output
try{ & "$env:WINDIR\System32\makecab.exe" /f payload.ddf | Out-Null; if($LASTEXITCODE -ne 0){throw 'Windows cabinet creation failed'} }finally{Pop-Location}
$expectedFiles=@($files.ForEach({[pscustomobject]@{id=$_[0];size=(Get-Item -LiteralPath (Join-Path $staged $_[1])).Length}}))
$cabinet=& (Join-Path $PSScriptRoot 'check-cabinet.ps1') -Path (Join-Path $output 'Hover.cab') -ExpectedFiles $expectedFiles
$cabinet | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $output 'cabinet-validation.json') -Encoding utf8
$msi=Join-Path $output "Hover-Development-$Version-x64.msi"
# Empty standard schema supplied with the installed Windows SDK. No SDK tool,
# validation cube, third-party installer engine, or custom action is shipped.
Copy-Item -LiteralPath (Join-Path $sdk 'orca.dat') -Destination $msi
$installer=New-Object -ComObject WindowsInstaller.Installer
$database=$null; $summary=$null
function Invoke-Com($Object,[string]$Name,[object[]]$Arguments){
    # Reflection's COM binder requires raw variants rather than PowerShell's
    # extended string wrappers returned by filesystem cmdlets.
    $raw=[object[]]::new($Arguments.Count)
    for($n=0;$n -lt $Arguments.Count;$n++){if($null -ne $Arguments[$n]){$raw[$n]=$Arguments[$n].PSObject.BaseObject}}
    $Object.GetType().InvokeMember($Name,'InvokeMethod',$null,$Object,$raw)
}
function Release-Com($Object){if($null -ne $Object){[void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($Object)}}
function SQL([string]$Query,$Record=$null){
    $view=Invoke-Com $database 'OpenView' @($Query)
    try{Invoke-Com $view 'Execute' @($Record) | Out-Null}finally{Release-Com $view}
}
function Row([string]$Table,[string[]]$Columns,[object[]]$Values){
    $record=Invoke-Com $installer 'CreateRecord' @($Values.Count)
    try{
        for($n=0;$n -lt $Values.Count;$n++){
            if($null -ne $Values[$n]){
                $field=if($Values[$n] -is [int]){'IntegerData'}else{'StringData'}
                $record.GetType().InvokeMember($field,'SetProperty',$null,$record,@(($n+1),$Values[$n])) | Out-Null
            }
        }
        $names=($Columns.ForEach({'`'+$_+'`'}) -join ',')
        $placeholders=(1..$Values.Count).ForEach({'?'}) -join ','
        SQL ('INSERT INTO `'+$Table+'` ('+$names+') VALUES ('+$placeholders+')') $record
    }finally{Release-Com $record}
}
function Property([string]$Name,[string]$Value){Row 'Property' @('Property','Value') @($Name,$Value)}
function Guid-Text([guid]$Value){'{'+$Value.ToString().ToUpperInvariant()+'}'}
$product=Guid-Text ([guid]::NewGuid()); $upgrade=Guid-Text $UpgradeCode
$digest=[Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes("Hover component v1 $upgrade"))
$component=Guid-Text ([guid]::new([byte[]]$digest[0..15]))
try{
    $database=Invoke-Com $installer 'OpenDatabase' @($msi,1)
    $keep=@('_Validation','Property','Directory','Feature','Component','File','FeatureComponents','Media','Registry','Shortcut','MsiShortcutProperty','RemoveFile','Upgrade','LaunchCondition','InstallExecuteSequence','AdvtExecuteSequence','Icon')
    $view=Invoke-Com $database 'OpenView' @('SELECT `Name` FROM `_Tables`')
    $tables=[Collections.Generic.List[string]]::new()
    try{Invoke-Com $view 'Execute' @($null) | Out-Null;while($record=Invoke-Com $view 'Fetch' @()){
        try{$tables.Add($record.GetType().InvokeMember('StringData','GetProperty',$null,$record,@(1)))}finally{Release-Com $record}
    }}finally{Release-Com $view}
    foreach($table in $tables){
        if($table -notin $keep){SQL ('DROP TABLE `'+$table+'`')}
        elseif($table -ne '_Validation'){SQL ('DELETE FROM `'+$table+'`')}
    }
    Property 'ProductCode' $product; Property 'UpgradeCode' $upgrade
    Property 'ProductName' "Hover Development ($InstallLeaf)"; Property 'ProductVersion' $Version
    Property 'ProductLanguage' '1033'; Property 'Manufacturer' 'Hover contributors'
    Property 'INSTALLLEVEL' '1'; Property 'ARPNOMODIFY' '1'
    Property 'ARPPRODUCTICON' 'HoverIcon'; Property 'ARPURLINFOABOUT' 'https://github.com/Herbertofury/Hover-App'
    Property 'ARPCOMMENTS' 'Unsigned development package. Release qualification is incomplete.'
    Property 'SecureCustomProperties' 'HOVER_OLDER_PRODUCTS;HOVER_NEWER_PRODUCTS;HOVER_ALLOW_ROLLBACK'
    $shortLeaf='H'+([Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($InstallLeaf)))).Substring(0,7)
    foreach($row in @(@('TARGETDIR',$null,'SourceDir'),@('LocalAppDataFolder','TARGETDIR','.'),@('ProgramsFolder','LocalAppDataFolder','Programs'),@('INSTALLDIR','ProgramsFolder',"$shortLeaf|$InstallLeaf"),@('ProgramMenuFolder','TARGETDIR','.'),@('HoverMenu','ProgramMenuFolder',"$shortLeaf|$InstallLeaf"))){Row 'Directory' @('Directory','Directory_Parent','DefaultDir') $row}
    Row 'Feature' @('Feature','Title','Description','Display','Level','Directory_','Attributes') @('Hover','Hover','Native read-only preview app',1,1,'INSTALLDIR',0)
    Row 'Component' @('Component','ComponentId','Directory_','Attributes','KeyPath') @('HoverApp',$component,'INSTALLDIR',260,'InstallMarker')
    Row 'FeatureComponents' @('Feature_','Component_') @('Hover','HoverApp')
    Row 'Registry' @('Registry','Root','Key','Name','Value','Component_') @('InstallMarker',1,"Software\Hover\Installations\$InstallLeaf",'InstallPath','[INSTALLDIR]','HoverApp')
    $sequence=0
    foreach($file in $files){
        $sequence++;$path=Join-Path $staged $file[1]
        $fileVersion=[Diagnostics.FileVersionInfo]::GetVersionInfo($path).FileVersion
        if($file[1] -notlike '*.exe'){$fileVersion=$null}
        $companion=-not $fileVersion
        if($companion){$fileVersion='HoverExe'}
        $shortNames=@{AgentExe='HOVAGENT.EXE';BrowserExe='BROWSER.EXE';Notices='NOTICES.MD';DependencyLicenses='DEPLICS.TXT';BuildInfo='BUILDINF.JSN'}
        $name=if($shortNames.ContainsKey($file[0])){$shortNames[$file[0]]+'|'+$file[1]}else{$file[1]}
        $language=if(-not $companion){'1033'}else{$null}
        Row 'File' @('File','Component_','FileName','FileSize','Version','Language','Attributes','Sequence') @($file[0],'HoverApp',$name,[int](Get-Item -LiteralPath $path).Length,$fileVersion,$language,512,$sequence)
    }
    Row 'Media' @('DiskId','LastSequence','Cabinet') @(1,$sequence,'#Hover.cab')
    Row 'Shortcut' @('Shortcut','Directory_','Name','Component_','Target','Description','Icon_','IconIndex','ShowCmd','WkDir') @('HoverShortcut','HoverMenu','HOVERDEV|Hover Development','HoverApp','[#HoverExe]','Hover development preview app','HoverIcon',0,1,'INSTALLDIR')
    Row 'MsiShortcutProperty' @('MsiShortcutProperty','Shortcut_','PropertyKey','PropVariantValue') @('AppId','HoverShortcut','System.AppUserModel.ID','Herbertofury.Hover')
    foreach($directory in @('INSTALLDIR','HoverMenu','ProgramsFolder')){Row 'RemoveFile' @('FileKey','Component_','DirProperty','InstallMode') @("Remove$directory",'HoverApp',$directory,2)}
    Row 'Upgrade' @('UpgradeCode','VersionMax','Attributes','ActionProperty') @($upgrade,$Version,1,'HOVER_OLDER_PRODUCTS')
    Row 'Upgrade' @('UpgradeCode','VersionMin','Attributes','ActionProperty') @($upgrade,$Version,256,'HOVER_NEWER_PRODUCTS')
    Row 'LaunchCondition' @('Condition','Description') @('NOT ALLUSERS','Hover Development must be installed for the current user.')
    Row 'LaunchCondition' @('Condition','Description') @('Installed OR NOT HOVER_NEWER_PRODUCTS OR (HOVER_ALLOW_ROLLBACK=1 AND REINSTALLMODE="amus")','A same or newer Hover Development is installed. Explicit local rollback requires HOVER_ALLOW_ROLLBACK=1 REINSTALLMODE=amus.')
    $actions=@(
        @('FindRelatedProducts','NOT Installed',25),@('LaunchConditions',$null,100),
        @('CostInitialize',$null,800),@('FileCost',$null,900),@('CostFinalize',$null,1000),@('MigrateFeatureStates',$null,1200),
        @('InstallValidate',$null,1400),@('InstallInitialize',$null,1500),@('ProcessComponents',$null,1600),
        @('UnpublishFeatures',$null,1800),@('RemoveShortcuts',$null,3200),@('RemoveFiles',$null,3500),@('RemoveFolders',$null,3600),
        @('CreateFolders',$null,3700),@('InstallFiles',$null,4000),@('CreateShortcuts',$null,4500),@('WriteRegistryValues',$null,5000),@('RemoveRegistryValues',$null,2600),
        @('RegisterUser',$null,6000),@('RegisterProduct',$null,6100),@('PublishFeatures',$null,6300),@('PublishProduct',$null,6400),@('InstallExecute',$null,6500),@('RemoveExistingProducts',$null,6550),@('InstallFinalize',$null,6600)
    )
    foreach($action in $actions){Row 'InstallExecuteSequence' @('Action','Condition','Sequence') $action}
    foreach($action in $actions | Where-Object {$_[0] -in @('CostInitialize','CostFinalize','InstallValidate','InstallInitialize','CreateShortcuts','PublishFeatures','PublishProduct','InstallFinalize')}){Row 'AdvtExecuteSequence' @('Action','Condition','Sequence') $action}
    # Binary streams are inserted through records, never interpolated into SQL.
    foreach($stream in @(@('_Streams','Name','Data','Hover.cab',(Join-Path $output 'Hover.cab')),@('Icon','Name','Data','HoverIcon',(Join-Path $staged 'hover.exe')))){
        $record=Invoke-Com $installer 'CreateRecord' @(2)
        try{
            $record.GetType().InvokeMember('StringData','SetProperty',$null,$record,@(1,$stream[3])) | Out-Null
            Invoke-Com $record 'SetStream' @(2,$stream[4]) | Out-Null
            SQL ('INSERT INTO `'+$stream[0]+'` (`'+$stream[1]+'`,`'+$stream[2]+'`) VALUES (?,?)') $record
        }finally{Release-Com $record}
    }
    $summary=$database.GetType().InvokeMember('SummaryInformation','GetProperty',$null,$database,@(20))
    foreach($entry in @(@(1,1252),@(2,'Installation Database'),@(3,'Hover Development'),@(4,'Hover contributors'),@(7,'x64;1033'),@(9,(Guid-Text ([guid]::NewGuid()))),@(14,500),@(15,10),@(18,'Hover native MSI authoring'),@(19,2))){$summary.GetType().InvokeMember('Property','SetProperty',$null,$summary,$entry) | Out-Null}
    Invoke-Com $summary 'Persist' @() | Out-Null
    Invoke-Com $database 'Commit' @() | Out-Null
}finally{Release-Com $summary;Release-Com $database;Release-Com $installer}
& (Join-Path $PSScriptRoot 'validate-msi.ps1') -Package $msi -SdkDirectory $sdk -Output (Join-Path $output 'ice-validation.json')
if($LASTEXITCODE -ne 0){throw 'Installer SDK validation failed'}
$validation=Get-Content -LiteralPath (Join-Path $output 'ice-validation.json') -Raw | ConvertFrom-Json
$warningReview=@{
    ICE06='The SDK cube describes ExtendedType on its own validation CustomAction table; Hover ships no CustomAction table or actions.'
    ICE61='Explicit local rollback intentionally removes a newer related development product, gated before the transaction by HOVER_ALLOW_ROLLBACK=1.'
    ICE91='The package is deliberately per-user only; a launch condition rejects ALLUSERS.'
}
foreach($warning in $validation.Messages | Where-Object Type -eq 2){if(-not $warningReview.ContainsKey($warning.Ice)){throw "Unreviewed SDK warning: $($warning.Ice) $($warning.Description)"}}
$warningReview | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $output 'ice-warning-review.json') -Encoding utf8
$manifest=[ordered]@{schemaVersion=1;productCode=$product;upgradeCode=$upgrade;version=$Version;installLeaf=$InstallLeaf;perUser=$true;customActions=$false;releaseVerified=$false;package=[IO.Path]::GetFileName($msi);sha256=(Get-FileHash -LiteralPath $msi).Hash;build=$identity;files=@($files.ForEach({[ordered]@{name=$_[1];sha256=(Get-FileHash -LiteralPath (Join-Path $staged $_[1])).Hash}}))}
$manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $output 'package.json') -Encoding utf8
Write-Host "Built development MSI: $msi"
exit 0
