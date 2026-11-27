# Fetches the NVIDIA files DLSS needs (about 70 MB) from NVIDIA's own GitHub release of Streamline into
# <To>\Streamline\bin\x64. They are NVIDIA's, under NVIDIA's licences (saved beside them: read them); this
# repository does not and may not carry them, so each person gets them from NVIDIA directly. `victory get-dlss`
# (and victory's first start with VICTORY_DLSS=1, after asking) runs this with -To %LOCALAPPDATA%\victory\nvidia-sdk.
# Nothing of NVIDIA's is needed to build victory: the files are loaded when the game runs.
# Contacts only github.com (the release's download address and the address it redirects to); writes only in -To.
# (port of trustno1 scripts/get-nvidia-sdks.ps1, GPL-3.0)
param(
    [string]$StreamlineTag = 'v2.14.16',
    # The folder for them (`victory get-dlss` names %LOCALAPPDATA%\victory\nvidia-sdk).
    [Parameter(Mandatory = $true)][string]$To
)
$ProgressPreference = 'SilentlyContinue'   # Invoke-WebRequest's progress bar makes it many times slower
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$sdk = $To
$sl = Join-Path $sdk 'Streamline'
$bin = Join-Path $sl 'bin\x64'
# What Streamline loads for DLSS: its interposer and plugins, and NGX's DLSS models.
$dlls = 'sl.interposer.dll', 'sl.common.dll', 'sl.dlss.dll', 'sl.dlss_g.dll', 'sl.reflex.dll', 'sl.pcl.dll', 'nvngx_dlss.dll', 'nvngx_dlssg.dll'

# One file from a URL, with curl's progress line where Windows has curl (it has since Windows 10 1803).
function Get-File([string]$Url, [string]$To) {
    New-Item -ItemType Directory -Force (Split-Path $To) | Out-Null
    $part = "$To.part"
    if (Get-Command curl.exe -ErrorAction SilentlyContinue) {
        & curl.exe --location --fail --silent --show-error --progress-bar --output $part $Url
        if ($LASTEXITCODE -ne 0) { Remove-Item $part -ErrorAction SilentlyContinue; throw "could not get $Url" }
    } else {
        Invoke-WebRequest -Uri $Url -OutFile $part -UseBasicParsing -ErrorAction Stop
    }
    Move-Item $part $To -Force
}

# Bytes first..last of a URL.
function Get-Range([string]$Url, [long]$First, [long]$Last) {
    $q = [Net.HttpWebRequest]::Create($Url)
    $q.UserAgent = 'get-nvidia-sdks'
    $q.AddRange($First, $Last)
    $r = $q.GetResponse()
    try {
        if ([int]$r.StatusCode -ne 206) { throw 'the server does not give parts of the file' }
        $m = New-Object IO.MemoryStream
        $r.GetResponseStream().CopyTo($m)
        , $m.ToArray()
    } finally { $r.Close() }
}

# Takes the named files out of a zip on a server without downloading the whole zip: its directory is at
# its end, and each file in it can be asked for by its place. $Want: a regex on the paths in the zip.
# Each goes into $Into under the part of its path after the SDK's top folder.
function Get-FromZip([string]$Url, [string]$Want, [string]$Into) {
    # Where the file really is (the download address redirects), and how long it is.
    $h = [Net.HttpWebRequest]::Create($Url); $h.Method = 'HEAD'; $h.UserAgent = 'get-nvidia-sdks'
    $r = $h.GetResponse(); $real = $r.ResponseUri.AbsoluteUri; $length = $r.ContentLength; $r.Close()
    $realHost = ([Uri]$real).Host
    if ($realHost -ne 'github.com' -and $realHost -notlike '*.githubusercontent.com') { throw "the release redirects to $realHost, not GitHub" }
    $tailLen = [Math]::Min($length, 2MB)
    $tail = Get-Range $real ($length - $tailLen) ($length - 1)
    $end = -1
    for ($i = $tail.Length - 22; $i -ge 0; $i--) {
        if ($tail[$i] -eq 0x50 -and $tail[$i + 1] -eq 0x4b -and $tail[$i + 2] -eq 5 -and $tail[$i + 3] -eq 6) { $end = $i; break }
    }
    if ($end -lt 0) { throw 'no zip directory found' }
    $dirSize = [BitConverter]::ToUInt32($tail, $end + 12)
    $dirAt = [BitConverter]::ToUInt32($tail, $end + 16)
    if ($dirAt -eq [uint32]::MaxValue) { throw 'a zip64 archive' }
    $dir = if ($dirAt -ge $length - $tailLen) {
        $from = [int]($dirAt - ($length - $tailLen)); , $tail[$from..($from + $dirSize - 1)]
    } else { Get-Range $real $dirAt ($dirAt + $dirSize - 1) }
    $got = 0
    $at = 0
    while ($at + 46 -le $dir.Length -and [BitConverter]::ToUInt32($dir, $at) -eq 0x02014b50) {
        $method = [BitConverter]::ToUInt16($dir, $at + 10)
        $packed = [BitConverter]::ToUInt32($dir, $at + 20)
        $size = [BitConverter]::ToUInt32($dir, $at + 24)
        $nameLen = [BitConverter]::ToUInt16($dir, $at + 28)
        $extraLen = [BitConverter]::ToUInt16($dir, $at + 30)
        $noteLen = [BitConverter]::ToUInt16($dir, $at + 32)
        $headerAt = [BitConverter]::ToUInt32($dir, $at + 42)
        $name = [Text.Encoding]::UTF8.GetString($dir, $at + 46, $nameLen)
        $at += 46 + $nameLen + $extraLen + $noteLen
        if ($name -notmatch $Want) { continue }
        # The file's own header (its extra field may be longer than the directory's), then its bytes.
        $piece = Get-Range $real $headerAt ($headerAt + 30 + $nameLen + 2048 + $packed - 1)
        if ([BitConverter]::ToUInt32($piece, 0) -ne 0x04034b50) { throw "no file header for $name" }
        $dataAt = 30 + [BitConverter]::ToUInt16($piece, 26) + [BitConverter]::ToUInt16($piece, 28)
        if ($dataAt + $packed -gt $piece.Length) { throw "the header of $name is longer than expected" }
        $out = [IO.MemoryStream]::new()
        if ($method -eq 0) { $out.Write($piece, $dataAt, [int]$packed) }
        elseif ($method -eq 8) {
            $in = [IO.MemoryStream]::new($piece, $dataAt, [int]$packed)
            $z = [IO.Compression.DeflateStream]::new($in, [IO.Compression.CompressionMode]::Decompress)
            $z.CopyTo($out); $z.Close()
        } else { throw "$name is packed in a way this script does not know ($method)" }
        if ($out.Length -ne $size) { throw "$name came out $($out.Length) bytes long, not $size" }
        # Only bin\x64\<file> and license.txt: the regex below admits nothing else, so nothing lands outside $Into.
        $rel = ($name -replace '^.*?(?=(bin/x64/|license\.txt$))', '') -replace '/', '\'
        if ($rel -match '\.\.') { throw "${name}: a path that leaves the folder" }
        $to = Join-Path $Into $rel
        New-Item -ItemType Directory -Force (Split-Path $to) | Out-Null
        [IO.File]::WriteAllBytes($to, $out.ToArray())
        Write-Host ("  {0}  {1:N1} MB" -f $rel, ($size / 1MB))
        $got++
    }
    $got
}

Write-Host "NVIDIA Streamline $StreamlineTag with DLSS, from github.com/NVIDIA-RTX/Streamline (about 70 MB):"
$missing = @($dlls | Where-Object { -not (Test-Path (Join-Path $bin $_)) })
if (-not $missing) {
    Write-Host "  all there already: $bin"
} else {
    $url = "https://github.com/NVIDIA-RTX/Streamline/releases/download/$StreamlineTag/streamline-sdk-$StreamlineTag.zip"
    $names = ($dlls | ForEach-Object { [regex]::Escape($_) }) -join '|'
    $want = "(^|/)bin/x64/($names|[^/]*license[^/]*\.txt)$|^([^/]+/)?license\.txt$"
    try { [void](Get-FromZip $url $want $sl) }
    catch { Write-Host "  Taking the files out of the archive in place did not work ($($_.Exception.Message))." }
    if (@($dlls | Where-Object { -not (Test-Path (Join-Path $bin $_)) })) {
        # The whole archive then (263 MB), into -To and removed after.
        Write-Host '  Getting the whole archive instead (263 MB) ...'
        $zip = Join-Path $sdk "streamline-sdk-$StreamlineTag.zip"
        Get-File $url $zip
        $into = Join-Path $sdk 'streamline-unpacked'
        Expand-Archive -Path $zip -DestinationPath $into -Force -ErrorAction Stop
        $found = Get-ChildItem $into -Recurse -Filter 'sl.interposer.dll' |
            Where-Object { $_.FullName -match '\\bin\\x64\\sl\.interposer\.dll$' -and $_.FullName -notmatch '\\development\\' } |
            Select-Object -First 1
        if (-not $found) { throw 'The Streamline archive has no bin\x64\sl.interposer.dll.' }
        $top = Split-Path (Split-Path (Split-Path $found.FullName))
        New-Item -ItemType Directory -Force $bin | Out-Null
        Get-ChildItem (Join-Path $top 'bin\x64') -File |
            Where-Object { $dlls -contains $_.Name -or $_.Name -match 'license.*\.txt$' } |
            Copy-Item -Destination $bin -Force
        Copy-Item (Join-Path $top 'license.txt') $sl -Force -ErrorAction SilentlyContinue
        Remove-Item $into -Recurse -Force -ErrorAction SilentlyContinue
        Remove-Item $zip -Force
    }
}
$still = @($dlls | Where-Object { -not (Test-Path (Join-Path $bin $_)) })
if ($still) { throw "Not there after all: $($still -join ', ')" }
Write-Host "  licences: $(Join-Path $sl 'license.txt') (Streamline) and the *.license.txt files in $bin (DLSS, Reflex)"
Write-Host ''
Write-Host 'Done. They are used from the next start.'
