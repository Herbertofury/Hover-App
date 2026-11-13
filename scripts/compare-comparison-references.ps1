param([Parameter(Mandatory)][string]$EvidenceDirectory)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Drawing
$run=(Resolve-Path -LiteralPath $EvidenceDirectory).Path
$native=Get-Content -LiteralPath (Join-Path $run 'result.json') -Raw|ConvertFrom-Json
if(-not $native.passed){throw 'Use a passing native comparison run'}
function Pixels([string]$Path){
    $bitmap=[Drawing.Bitmap]::new($Path)
    $locked=$null
    try{
        $locked=$bitmap.LockBits([Drawing.Rectangle]::new(0,0,$bitmap.Width,$bitmap.Height),[Drawing.Imaging.ImageLockMode]::ReadOnly,[Drawing.Imaging.PixelFormat]::Format32bppPArgb)
        $bytes=[byte[]]::new($bitmap.Width*$bitmap.Height*4)
        for($row=0;$row -lt $bitmap.Height;$row++){[Runtime.InteropServices.Marshal]::Copy([IntPtr]::Add($locked.Scan0,$row*$locked.Stride),$bytes,$row*$bitmap.Width*4,$bitmap.Width*4)}
        return ,$bytes
    }finally{if($locked){$bitmap.UnlockBits($locked)};$bitmap.Dispose()}
}
$original=Pixels (Join-Path $run 'reference.bmp')
$imageChecks=@()
foreach($case in @(@('changed.bmp',400),@('same-pixels-different-bytes.bmp',0))){
    $other=Pixels (Join-Path $run $case[0]);if($other.Length -ne $original.Length){throw 'Reference dimensions differ'}
    $changed=0
    for($pixel=0;$pixel -lt $original.Length;$pixel+=4){if($original[$pixel] -ne $other[$pixel] -or $original[$pixel+1] -ne $other[$pixel+1] -or $original[$pixel+2] -ne $other[$pixel+2] -or $original[$pixel+3] -ne $other[$pixel+3]){$changed++}}
    if($changed -ne $case[1]){throw 'GDI+ reference disagrees with expected/native pixel difference'}
    $imageChecks+=[ordered]@{fixture=$case[0];gdiplusChangedPixels=$changed;nativeExpected=$case[1];matches=$true}
}
$alpha=Pixels (Join-Path $run 'transparent.png')
if($alpha[2] -ne 128 -or $alpha[3] -ne 128){throw 'Independent GDI+ premultiplied-alpha control failed'}
$output=git -c core.autocrlf=false diff --no-index --no-ext-diff --no-textconv --text --minimal --numstat -- (Join-Path $run 'before.txt') (Join-Path $run 'after.txt')
if($LASTEXITCODE -notin @(0,1)){throw 'External Git reference failed'}
$fields=$output -split "`t";if([int]$fields[0] -ne 1 -or [int]$fields[1] -ne 0){throw 'External Git displayed-line edit count disagrees'}
$result=[ordered]@{schemaVersion=1;passed=$true;scope='Independent Windows GDI+ decode/premultiplied-channel checks and external Git minimal line edit; generated fixtures only';nativeExecutableSha256=$native.executableSha256;images=$imageChecks;alphaControl=[ordered]@{format='Format32bppPArgb';red=128;alpha=128;matches=$true};text=[ordered]@{tool=(git --version);added=[int]$fields[0];removed=[int]$fields[1];nativeAlignedChangedRows=1;matches=$true};notEstablished=@('ImageMagick comparison: tool not installed','perceptual/colorimetric similarity','specialist game/3D fidelity','physical shell/keyboard/accessibility/DPI','fair cross-tool speed comparison')}
$result|ConvertTo-Json -Depth 7|Set-Content -LiteralPath (Join-Path $run 'references.json') -Encoding utf8
$result|ConvertTo-Json -Depth 7
exit 0
