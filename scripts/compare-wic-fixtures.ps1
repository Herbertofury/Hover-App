param(
    [string]$Benchmark = (Join-Path $PSScriptRoot '../target/release/examples/decoder_bench.exe'),
    [ValidateRange(1,16)][int]$Repeats = 4,
    [string]$OutputDirectory = (Join-Path $PSScriptRoot '../target/codec-evidence'),
    [switch]$Json
)
$ErrorActionPreference='Stop'
$benchmarkPath=(Resolve-Path -LiteralPath $Benchmark).Path
$runDirectory=Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ('session-'+[Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
Add-Type -AssemblyName System.Drawing
$bitmap=[Drawing.Bitmap]::new(128,64,[Drawing.Imaging.PixelFormat]::Format32bppArgb)
try {
    for($y=0;$y-lt 64;$y++) {
        for($x=0;$x-lt 128;$x++) {
            $bitmap.SetPixel($x,$y,[Drawing.Color]::FromArgb($(if($x-lt 64){255}else{160}),($x*2),($y*4),(255-$x)))
        }
    }
    foreach($item in @(
        @('png',[Drawing.Imaging.ImageFormat]::Png),
        @('jpg',[Drawing.Imaging.ImageFormat]::Jpeg),
        @('bmp',[Drawing.Imaging.ImageFormat]::Bmp),
        @('tiff',[Drawing.Imaging.ImageFormat]::Tiff),
        @('gif',[Drawing.Imaging.ImageFormat]::Gif)
    )) { $bitmap.Save((Join-Path $runDirectory ('generated.'+$item[0])),$item[1]) }
} finally { $bitmap.Dispose() }
# An ICO with one explicitly generated PNG frame, not a borrowed icon asset.
$png=[IO.File]::ReadAllBytes((Join-Path $runDirectory 'generated.png'))
$writer=[IO.BinaryWriter]::new([IO.File]::Open((Join-Path $runDirectory 'generated.ico'),[IO.FileMode]::CreateNew))
try {
    $writer.Write([uint16]0);$writer.Write([uint16]1);$writer.Write([uint16]1)
    $writer.Write([byte[]](128,64,0,0));$writer.Write([uint16]1);$writer.Write([uint16]32)
    $writer.Write([uint32]$png.Length);$writer.Write([uint32]22);$writer.Write($png)
} finally { $writer.Dispose() }
$cases=[Collections.Generic.List[object]]::new()
foreach($extension in @('png','jpg','bmp','tiff','gif','ico')) {
    $fixture=Join-Path $runDirectory ('generated.'+$extension)
    $before=(Get-FileHash -LiteralPath $fixture -Algorithm SHA256).Hash
    $raw=& $benchmarkPath $fixture $Repeats
    if($LASTEXITCODE-ne 0){throw "Native component comparison failed for generated $extension"}
    $comparison=$raw | ConvertFrom-Json
    if(-not $comparison.all_outputs_equal -or $comparison.samples.Count-ne 2*$Repeats){throw "Incomplete exact comparison for $extension"}
    if((Get-FileHash -LiteralPath $fixture -Algorithm SHA256).Hash-ne $before){throw "Generated $extension bytes changed"}
    $cases.Add([ordered]@{format=$extension;fixture='generated.'+$extension;sha256=$before;sourceUnchanged=$true;comparison=$comparison})
}
$evidence=[ordered]@{
    schemaVersion=1;measuredAtUtc=[DateTime]::UtcNow.ToString('o');passed=$true
    classification='Generated synthetic Windows GDI+ encoded images and a generated PNG-in-ICO frame'
    comparison='Exact WIC/reference versus LPAC process pixels, geometry, stride and frame delays; GIF uses the same Rust reference decoder'
    benchmarkExecutableSha256=(Get-FileHash -LiteralPath $benchmarkPath -Algorithm SHA256).Hash
    os=[Environment]::OSVersion.VersionString;architecture=$env:PROCESSOR_ARCHITECTURE
    cache='Generated fixtures were just written; OS caches uncontrolled; sequential component comparison'
    excludes=@('independent specialist rendering','real game/artist files','installed third-party codec certification','Explorer','physical frames','storage-cold','installer/update/rollback')
    cases=$cases
}
$evidence | ConvertTo-Json -Depth 25 | Set-Content -LiteralPath (Join-Path $runDirectory 'codec-comparison.json') -Encoding utf8
if($Json){$evidence | ConvertTo-Json -Depth 25}else{Write-Output "Codec comparison evidence: $runDirectory";Write-Output 'Passed six generated format comparisons; all source hashes unchanged.'}
exit 0
