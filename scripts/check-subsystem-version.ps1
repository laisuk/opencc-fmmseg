param(
    [Parameter(Mandatory = $true)]
    [string]$Path
)

$bytes = [System.IO.File]::ReadAllBytes($Path)

if ($bytes.Length -lt 0x40) {
    throw "Invalid PE file: $Path"
}

$peOffset = [BitConverter]::ToInt32($bytes, 0x3C)
$optionalHeader = $peOffset + 24

if ($bytes.Length -lt ($optionalHeader + 0x34)) {
    throw "Invalid PE optional header: $Path"
}

$major = [BitConverter]::ToUInt16($bytes, $optionalHeader + 0x30)
$minor = [BitConverter]::ToUInt16($bytes, $optionalHeader + 0x32)

Write-Host "$Path : subsystem version $major.$minor"

if ($major -ne 6 -or $minor -ne 1) {
    throw "Unexpected subsystem version $major.$minor in $Path; expected 6.1"
}