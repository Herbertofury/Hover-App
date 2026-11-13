param([Parameter(Mandatory)][string]$Package,[Parameter(Mandatory)][string]$SdkDirectory,[Parameter(Mandatory)][string]$Output)
$ErrorActionPreference='Stop'
$packagePath=(Resolve-Path -LiteralPath $Package).Path
$sdk=(Resolve-Path -LiteralPath $SdkDirectory).Path
$engine=Join-Path $sdk 'evalcom2.dll'
$cube=Join-Path $sdk 'darice.cub'
if((Get-AuthenticodeSignature -LiteralPath $engine).Status -ne 'Valid'){throw 'Microsoft SDK validation engine must have a valid signature'}
if((Get-AuthenticodeSignature -LiteralPath $engine).SignerCertificate.Subject -notmatch 'CN=Microsoft Corporation,'){throw 'Unexpected SDK validation engine publisher'}
if(-not(Test-Path -LiteralPath $cube -PathType Leaf)){throw 'SDK darice.cub is required'}
if([Environment]::Is64BitProcess){
    # SDK Orca ships a 32-bit validator. Use its native bitness without COM
    # registration, elevation, execution-policy changes, or global installation.
    & "$env:WINDIR\SysWOW64\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -File $PSCommandPath -Package $packagePath -SdkDirectory $sdk -Output ([IO.Path]::GetFullPath($Output))
    if($LASTEXITCODE -ne 0){throw "SDK ICE validation failed: exit $LASTEXITCODE"}
    exit 0
}
Add-Type -Path (Join-Path $PSScriptRoot 'MsiValidation.cs')
$result=[HoverPackaging.MsiValidation]::Run($engine,$packagePath,$cube)
$result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Output -Encoding UTF8
if($result.HResult -ne 0 -or $result.Errors -gt 0 -or -not $result.Completed){throw "SDK ICE validation failed: HRESULT $($result.HResult), errors $($result.Errors), completed $($result.Completed). See $Output"}
Write-Host "SDK ICE validation completed: $($result.Errors) errors, $($result.Warnings) warnings. $Output"
