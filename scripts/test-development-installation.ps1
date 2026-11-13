param(
    [string]$RustToolchain='1.95.0',
    [string]$SdkDirectory='',
    [string]$DbpfFixture='',
    [string]$OutputDirectory=(Join-Path $PSScriptRoot '../target/packaging-evidence'),
    [switch]$AllowDirty
)
$ErrorActionPreference='Stop'
$run=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ('session-'+[guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $run -Force | Out-Null
if(-not $SdkDirectory){$SdkDirectory=& (Join-Path $PSScriptRoot 'prepare-installer-sdk.ps1') -OutputDirectory (Join-Path $run 'sdk')}
$leaf='Hover-Test-'+[guid]::NewGuid().ToString('N').Substring(0,12)
$upgrade=[guid]::NewGuid()
$packages=[Collections.Generic.List[string]]::new()
foreach($version in @('0.1.0','0.1.1')){
    $build=Join-Path $run "build-$version"
    & (Join-Path $PSScriptRoot 'build-development.ps1') -Version $version -OutputDirectory $build -RustToolchain $RustToolchain -AllowDirty:$AllowDirty
    if($LASTEXITCODE -ne 0){throw 'Development payload build failed'}
    $package=Join-Path $run "package-$version"
    & (Join-Path $PSScriptRoot '../installer/native/build-msi.ps1') -PayloadDirectory (Join-Path $build 'payload') -Version $version -SdkDirectory $SdkDirectory -OutputDirectory $package -InstallLeaf $leaf -UpgradeCode $upgrade
    if($LASTEXITCODE -ne 0){throw 'Development MSI build failed'}
    $packages.Add($package)
}
& (Join-Path $PSScriptRoot 'test-installation.ps1') -PackageA $packages[0] -PackageB $packages[1] -DbpfFixture $DbpfFixture -OutputDirectory (Join-Path $run 'tests')
if($LASTEXITCODE -ne 0){throw 'Native installation tests failed'}
Write-Host "Complete development-package evidence: $run"
exit 0
