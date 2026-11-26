# Packages Green as a portable Windows zip: green.exe, fonts, icons, licences,
# notices and an empty data folder. Run from anywhere; it stages under dist/.
[CmdletBinding()]
param(
    [string]$Version,
    [string]$OutDir = "dist",
    [switch]$SkipBuild,
    [switch]$SkipVcrtCheck
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not $Version) {
    $manifest = Get-Content (Join-Path $root "Cargo.toml") -Raw
    $found = [regex]::Match($manifest, '(?m)^version\s*=\s*"([^"]+)"')
    if (-not $found.Success) { throw "no version in Cargo.toml" }
    $Version = $found.Groups[1].Value
}

if ($env:CARGO_TARGET_DIR) {
    $target = $env:CARGO_TARGET_DIR
    if (-not [System.IO.Path]::IsPathRooted($target)) { $target = Join-Path $root $target }
} else {
    $target = Join-Path $root "target"
}

if (-not $SkipBuild) {
    Write-Host "building green $Version (release)"
    & cargo build -p green --release --locked
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}

$exe = Join-Path $target "release/green.exe"
if (-not (Test-Path $exe)) { throw "no release build at $exe" }

$name = "Green-$Version-windows-x86_64"
$stage = Join-Path (Join-Path $root $OutDir) $name
$zip = Join-Path (Join-Path $root $OutDir) "$name.zip"
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
if (Test-Path $zip) { Remove-Item -Force $zip }
New-Item -ItemType Directory -Force $stage | Out-Null

# The app reads `fonts/` beside the executable; the icons are for reference.
Copy-Item $exe (Join-Path $stage "green.exe")
New-Item -ItemType Directory -Force (Join-Path $stage "fonts") | Out-Null
$fonts = Get-ChildItem (Join-Path $root "assets/fonts") -Filter "*.otf"
if ($fonts.Count -eq 0) { throw "no .otf files in assets/fonts" }
$fonts | ForEach-Object { Copy-Item $_.FullName (Join-Path $stage "fonts") }
$needed = @("NotoSansCJKsc-Regular.otf", "NotoSansCJKsc-Bold.otf")
foreach ($font in $needed) {
    if (-not (Test-Path (Join-Path $stage "fonts/$font"))) { throw "missing font $font" }
}

New-Item -ItemType Directory -Force (Join-Path $stage "assets/icons") | Out-Null
foreach ($asset in @("assets/icon.svg", "assets/icon-small.svg", "assets/icons/green.ico", "assets/icons/green.png")) {
    $from = Join-Path $root $asset
    if (-not (Test-Path $from)) { throw "missing $asset" }
    Copy-Item $from (Join-Path $stage $asset)
}

Copy-Item (Join-Path $root "LICENSE") (Join-Path $stage "LICENSE")
New-Item -ItemType Directory -Force (Join-Path $stage "LICENSES") | Out-Null
Copy-Item (Join-Path $root "LICENSES/*") (Join-Path $stage "LICENSES")
Copy-Item (Join-Path $root "THIRD-PARTY-NOTICES.txt") (Join-Path $stage "THIRD-PARTY-NOTICES.txt")
Copy-Item (Join-Path $root "packaging/windows/README-portable.txt") (Join-Path $stage "README.txt")

# A data folder beside the executable makes the copy portable; the readme
# keeps the folder in the zip.
New-Item -ItemType Directory -Force (Join-Path $stage "data") | Out-Null
Copy-Item (Join-Path $root "packaging/windows/data-readme.txt") (Join-Path $stage "data/README.txt")

if (-not $SkipVcrtCheck) {
    $imports = $null
    if (Get-Command llvm-readobj -ErrorAction SilentlyContinue) {
        $imports = (& llvm-readobj --coff-imports $exe 2>$null | Out-String)
    } elseif (Get-Command dumpbin -ErrorAction SilentlyContinue) {
        $imports = (& dumpbin /dependents $exe 2>$null | Out-String)
    } elseif (Get-Command objdump -ErrorAction SilentlyContinue) {
        $imports = (& objdump -p $exe 2>$null | Out-String)
    }
    if (-not $imports) {
        throw "cannot read $exe's imports: install LLVM (llvm-readobj) or run from a Visual Studio prompt"
    }
    $forbidden = @("VCRUNTIME140", "MSVCP140", "ucrtbase", "api-ms-win-crt")
    foreach ($dll in $forbidden) {
        if ($imports -match [regex]::Escape($dll)) { throw "$exe imports $dll; it is not crt-static" }
    }
    Write-Host "vcrt: no $($forbidden -join ', ') imports"
}

Add-Type -AssemblyName System.IO.Compression.FileSystem
# Entries use `/`, as the zip format asks; CreateFromDirectory writes `\` on Windows.
$archive = [System.IO.Compression.ZipFile]::Open($zip, "Create")
try {
    Get-ChildItem -Recurse $stage -File | ForEach-Object {
        $entry = $_.FullName.Substring($stage.Length + 1).Replace("\", "/")
        [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
            $archive, $_.FullName, $entry, [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
    }
} finally {
    $archive.Dispose()
}

# The zip must carry everything the app and the licence need.
$archive = [System.IO.Compression.ZipFile]::OpenRead($zip)
try {
    $entries = $archive.Entries | ForEach-Object { $_.FullName }
    foreach ($wanted in @("green.exe", "fonts/NotoSansCJKsc-Regular.otf", "fonts/NotoSansCJKsc-Bold.otf",
            "data/README.txt", "LICENSE", "LICENSES/OFL-1.1.txt", "THIRD-PARTY-NOTICES.txt", "README.txt")) {
        if ($entries -notcontains $wanted) { throw "the zip lacks $wanted" }
    }
} finally {
    $archive.Dispose()
}

Write-Host ""
Write-Host "staged $name"
Get-ChildItem -Recurse $stage | Where-Object { -not $_.PSIsContainer } |
    ForEach-Object { "  {0,10:N0}  {1}" -f $_.Length, $_.FullName.Substring($stage.Length + 1) }
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash
Write-Host ""
Write-Host "zip:  $zip"
Write-Host "sha256: $hash"
