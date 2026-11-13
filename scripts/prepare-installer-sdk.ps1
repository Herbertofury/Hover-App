param([Parameter(Mandatory)][string]$OutputDirectory)
$ErrorActionPreference='Stop'
$output=[IO.Path]::GetFullPath($OutputDirectory)
if(Test-Path -LiteralPath $output){throw 'SDK extraction requires a new directory'}
$versions=Get-ChildItem -LiteralPath 'C:\Program Files (x86)\Windows Kits\10\bin' -Directory | Sort-Object Name -Descending
$source=$versions | ForEach-Object {Join-Path $_.FullName 'x86/Orca-x86_en-us.msi'} | Where-Object {Test-Path -LiteralPath $_ -PathType Leaf} | Select-Object -First 1
if(-not $source){throw 'Install the Windows SDK MSI authoring tools first; Orca-x86_en-us.msi is missing'}
$signature=Get-AuthenticodeSignature -LiteralPath $source
if($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'CN=Microsoft Corporation,'){throw 'SDK MSI must have a valid Microsoft signature'}
New-Item -ItemType Directory -Path $output | Out-Null
$p=Start-Process -FilePath "$env:WINDIR\System32\msiexec.exe" -ArgumentList @('/a',"`"$source`"",'/qn',"TARGETDIR=`"$output`"",'/l*v',"`"$output\extract.log`"") -WindowStyle Hidden -PassThru
if(-not $p.WaitForExit(60000)){throw 'SDK extraction is still running; inspect before retrying'}
if($p.ExitCode -ne 0){throw "SDK extraction failed: $($p.ExitCode)"}
$sdk=Join-Path $output 'Orca'
foreach($name in @('orca.dat','darice.cub','evalcom2.dll')){if(-not(Test-Path -LiteralPath (Join-Path $sdk $name) -PathType Leaf)){throw "Missing SDK authoring file: $name"}}
[ordered]@{schemaVersion=1;source=[IO.Path]::GetFileName($source);sourceSha256=(Get-FileHash -LiteralPath $source).Hash;publisherVerified=$true;files=@(Get-ChildItem -LiteralPath $sdk -File | ForEach-Object {[ordered]@{name=$_.Name;sha256=(Get-FileHash -LiteralPath $_.FullName).Hash}})} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $output 'provenance.json') -Encoding utf8
Write-Output $sdk
