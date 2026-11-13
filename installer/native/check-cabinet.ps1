param([Parameter(Mandatory)][string]$Path,[Parameter(Mandatory)][object[]]$ExpectedFiles)
$ErrorActionPreference='Stop'
$length=(Get-Item -LiteralPath $Path).Length
if($length -lt 36 -or $length -gt 512MB){throw 'Cabinet size is outside the development-package budget'}
$stream=[IO.File]::OpenRead((Resolve-Path -LiteralPath $Path).Path)
$reader=[IO.BinaryReader]::new($stream)
try{
    $header=$reader.ReadBytes(36)
    if([Text.Encoding]::ASCII.GetString($header,0,4) -ne 'MSCF' -or [BitConverter]::ToUInt32($header,8) -ne $length){throw 'Invalid cabinet header or declared length'}
    $fileCount=[BitConverter]::ToUInt16($header,28);$flags=[BitConverter]::ToUInt16($header,30);$index=[BitConverter]::ToUInt16($header,34);$folders=[BitConverter]::ToUInt16($header,26)
    if(($flags -band 3) -ne 0 -or $index -ne 0){throw 'A single embedded cabinet is required; makecab produced a split/chained cabinet'}
    if($fileCount -ne $ExpectedFiles.Count){throw 'Cabinet file count differs from the complete payload'}
    $offset=[BitConverter]::ToUInt32($header,16)
    if($offset -lt 36 -or $offset -ge $length){throw 'Invalid cabinet file-list offset'}
    $stream.Position=$offset
    $files=@()
    for($number=0;$number -lt $fileCount;$number++){
        $record=$reader.ReadBytes(16);if($record.Length -ne 16){throw 'Truncated cabinet file record'}
        if([BitConverter]::ToUInt16($record,8) -ge $folders){throw 'A payload file spans outside this cabinet'}
        $name=[Collections.Generic.List[byte]]::new()
        while($true){$value=$reader.ReadByte();if($value -eq 0){break};$name.Add($value);if($name.Count -gt 128){throw 'Unexpected cabinet file identifier'}}
        $id=[Text.Encoding]::ASCII.GetString($name.ToArray());$size=[BitConverter]::ToUInt32($record,0)
        if($id -cne $ExpectedFiles[$number].id -or $size -ne $ExpectedFiles[$number].size){throw 'Cabinet identifiers, sizes or sequence differ from the MSI File table'}
        $files+=[ordered]@{id=$id;size=$size}
    }
    [ordered]@{schemaVersion=1;completeSingleCabinet=$true;bytes=$length;files=$files}
}finally{$reader.Dispose()}
