# Builds the ZipMount installers.
#
# Two files come out:
#   target\ZipMount.msi         - the package, for when WinFsp is already there
#   target\ZipMount-Setup.exe   - the bundle: installs WinFsp itself if missing
#
# The order matters: the program first, then the context menu package (the
# program builds and signs it itself, which needs the Windows SDK), then the
# MSI, and only then the bundle that carries the MSI inside.
#
#     powershell -ExecutionPolicy Bypass -File tools\build-installer.ps1
#
# Needs WiX 4 with its extensions. The version must be given: without it the
# current branch is fetched, which WiX 4 cannot load and silently marks
# "damaged". Run from the repository root - the extensions land in .wix next
# to the project.
#
#     dotnet tool install --global wix
#     wix extension add WixToolset.Bal.wixext/4.0.5
#     wix extension add WixToolset.Util.wixext/4.0.5

[CmdletBinding()]
param(
    [string] $Version,
    [switch] $SkipBuild,
    [switch] $MsiOnly
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot

if (-not $Version) {
    # The version is the one in Cargo.toml, with the commit count as a fourth
    # field.
    #
    # The commit count alone, as before, will not do: in a fresh repository it
    # would start over, the version would come out lower than the installed
    # one, and the installer would stop upgrading. What counts is the project
    # version, bumped by hand before a release; the commit count only orders
    # the builds of one version. The bundle compares all four fields, Windows
    # Installer only three, hence AllowSameVersionUpgrades in ZipMount.wxs.
    #
    # Without a growing version every build was installed next to the previous
    # one as another entry in "Installed apps" - seven of them piled up during
    # debugging. CI needs a full clone (fetch-depth: 0), or the count is 1.
    $manifest = Get-Content (Join-Path $root "Cargo.toml") -Raw
    $match = [regex]::Match($manifest, '(?ms)^\[workspace\.package\].*?^version\s*=\s*"(\d+\.\d+\.\d+)"')
    if (-not $match.Success) { throw "no version in [workspace.package] in Cargo.toml" }
    $count = & git -C $root rev-list --count HEAD 2>$null
    if ($LASTEXITCODE -ne 0 -or -not $count) { $count = 0 }
    $Version = "$($match.Groups[1].Value).$count"
}
Write-Host "Version: $Version"

$bin = Join-Path $root "target\release"
$assets = Join-Path $root "crates\zipmount\assets"
$package = Join-Path $root "target\package"
$deps = Join-Path $root "target\deps"
$localized = Join-Path $root "target\installer-locales"
$msi = Join-Path $root "target\ZipMount.msi"
$setup = Join-Path $root "target\ZipMount-Setup.exe"

# The official WinFsp installer. It goes into the bundle as is, with its
# author's signature - so people get exactly the file on the project's page.
$winfspVersion = "2.1.25156"
$winfspUrl = "https://github.com/winfsp/winfsp/releases/download/v2.1/winfsp-$winfspVersion.msi"
$winfspMsi = Join-Path $deps "winfsp-$winfspVersion.msi"

# Translations of the bundle's window, by Windows LCID. The bootstrapper
# matches the display language's LCID exactly, so common regional variants
# are listed too; anything unlisted falls back to English. The RTF charset and
# font help RichEdit pick glyphs for the license text: Segoe UI has no CJK.
$locales = [ordered]@{
    "ru"    = @{ Lcids = @(1049); Charset = 204; Font = "Segoe UI" }
    "zh-CN" = @{ Lcids = @(2052, 4100); Charset = 134; Font = "Microsoft YaHei UI" }
    "ja"    = @{ Lcids = @(1041); Charset = 128; Font = "Yu Gothic UI" }
    "ko"    = @{ Lcids = @(1042); Charset = 129; Font = "Malgun Gothic" }
    "pt-BR" = @{ Lcids = @(1046, 2070); Charset = 0; Font = "Segoe UI" }
    "es"    = @{ Lcids = @(3082, 1034, 2058, 11274, 9226, 13322, 10250, 8202, 58378); Charset = 0; Font = "Segoe UI" }
    "de"    = @{ Lcids = @(1031, 3079, 2055, 4103, 5127); Charset = 0; Font = "Segoe UI" }
}

function Resolve-Wix {
    $found = Get-Command wix -ErrorAction SilentlyContinue
    if ($found) { return $found.Source }
    $candidate = Join-Path $env:USERPROFILE ".dotnet\tools\wix.exe"
    if (Test-Path $candidate) { return $candidate }
    throw "wix not found - dotnet tool install --global wix"
}

function ConvertTo-Rtf([string] $text, [int] $charset, [string] $font) {
    # The bundle shows text only as RTF, and anything outside ASCII goes in as
    # numeric codes: otherwise it turns into question marks on a machine with
    # another code page. RTF takes those codes as signed 16-bit numbers, so
    # everything above 32767 - all of Korean Hangul, for one - has to be
    # written negative.
    $sb = New-Object System.Text.StringBuilder
    [void] $sb.Append("{\rtf1\ansi\deff0{\fonttbl{\f0\fnil\fcharset$charset $font;}}\fs18 ")
    foreach ($ch in $text.ToCharArray()) {
        $code = [int] $ch
        if ($ch -eq "`r") { continue }
        elseif ($ch -eq "`n") { [void] $sb.Append('\par ') }
        elseif ($ch -eq '\') { [void] $sb.Append('\\') }
        elseif ($ch -eq '{') { [void] $sb.Append('\{') }
        elseif ($ch -eq '}') { [void] $sb.Append('\}') }
        elseif ($code -gt 32767) { [void] $sb.Append('\u' + ($code - 65536) + '?') }
        elseif ($code -gt 127) { [void] $sb.Append('\u' + $code + '?') }
        else { [void] $sb.Append($ch) }
    }
    [void] $sb.Append('}')
    return $sb.ToString()
}

function Write-License([string] $code, [string] $target, [int] $charset, [string] $font) {
    $text = Get-Content (Join-Path $root "installer\locales\$code\about.txt") -Raw -Encoding UTF8
    New-Item -ItemType Directory (Split-Path $target) -Force | Out-Null
    Set-Content -Path $target -Value (ConvertTo-Rtf $text $charset $font) -Encoding Ascii
}

if (-not $SkipBuild) {
    Write-Host "Building the program..."
    Push-Location $root
    try { cargo build --release } finally { Pop-Location }
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}

foreach ($file in @("zipmount.exe", "zipmount_shell.dll")) {
    $path = Join-Path $bin $file
    if (-not (Test-Path $path)) { throw "no $path - build the project" }
}

Write-Host "Building the context menu package..."
$report = & (Join-Path $bin "zipmount.exe") shell-package --out $package
if ($LASTEXITCODE -ne 0) { throw "cannot build the menu package" }

# The command answers with "key=value" lines. The library's name carries a
# fingerprint of its contents and changes from build to build - the installer
# needs exactly that name, because the manifest inside the package refers to
# it.
$fields = @{}
foreach ($line in $report) {
    $pair = "$line".Split("=", 2)
    if ($pair.Count -eq 2) { $fields[$pair[0].Trim()] = $pair[1].Trim() }
}
$library = $fields["library"]
if (-not $library) { throw "the command did not report the library name" }
Write-Host "  library: $library"

$wix = Resolve-Wix

Write-Host "Building the MSI..."
& $wix build (Join-Path $root "installer\ZipMount.wxs") `
    -arch x64 `
    -d "Version=$Version" `
    -d "BinDir=$bin" `
    -d "AssetDir=$assets" `
    -d "PackageDir=$package" `
    -d "LibraryName=$library" `
    -o $msi
if ($LASTEXITCODE -ne 0) { throw "building the MSI failed" }

if ($MsiOnly) {
    Write-Host ""
    Write-Host ("Done: {0} ({1:N2} MB)" -f $msi, ((Get-Item $msi).Length / 1MB))
    return
}

New-Item -ItemType Directory $deps -Force | Out-Null
if (-not (Test-Path $winfspMsi)) {
    Write-Host "Downloading WinFsp $winfspVersion..."
    curl.exe -sL $winfspUrl -o $winfspMsi
    if (-not (Test-Path $winfspMsi)) { throw "cannot download $winfspUrl" }
}
$signature = Get-AuthenticodeSignature $winfspMsi
if ($signature.Status -ne "Valid") {
    throw "the downloaded WinFsp has an invalid signature: $($signature.Status)"
}

# The license text in every language, and the list of payloads that carries
# the translations into the bundle. The English license is the default one;
# the payload name "license.rtf" is what the bootstrapper probes for inside
# each LCID directory.
Remove-Item $localized -Recurse -Force -ErrorAction SilentlyContinue
$license = Join-Path $localized "license.rtf"
Write-License "en" $license 0 "Segoe UI"

$payloads = New-Object System.Text.StringBuilder
foreach ($code in $locales.Keys) {
    $info = $locales[$code]
    $rtf = Join-Path $localized "$code\license.rtf"
    Write-License $code $rtf $info.Charset $info.Font
    $wxl = Join-Path $root "installer\locales\$code\thm.wxl"
    foreach ($lcid in $info.Lcids) {
        # Explicit ids: WiX derives them from the source file, and one
        # translation serves several LCIDs.
        [void] $payloads.AppendLine("      <Payload Id=`"Theme$lcid`" Name=`"$lcid\thm.wxl`" SourceFile=`"$wxl`" />")
        [void] $payloads.AppendLine("      <Payload Id=`"License$lcid`" Name=`"$lcid\license.rtf`" SourceFile=`"$rtf`" />")
    }
}
$fragment = Join-Path $localized "Localizations.wxs"
@"
<?xml version="1.0" encoding="utf-8"?>
<!-- Generated by tools\build-installer.ps1 from installer\locales. -->
<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs">
  <Fragment>
    <PayloadGroup Id="BundleLocalizations">
$($payloads.ToString().TrimEnd())
    </PayloadGroup>
  </Fragment>
</Wix>
"@ | Set-Content -Path $fragment -Encoding UTF8

Write-Host "Building the bundle..."
& $wix build (Join-Path $root "installer\Bundle.wxs") $fragment `
    -ext WixToolset.Bal.wixext `
    -ext WixToolset.Util.wixext `
    -arch x64 `
    -d "Version=$Version" `
    -d "IconFile=$(Join-Path $assets 'ZipMount.ico')" `
    -d "LogoFile=$(Join-Path $assets 'Square44x44Logo.altform-unplated_targetsize-64.png')" `
    -d "LicenseFile=$license" `
    -d "WinFspMsi=$winfspMsi" `
    -d "ZipMountMsi=$msi" `
    -o $setup
if ($LASTEXITCODE -ne 0) { throw "building the bundle failed" }

Write-Host ""
Write-Host ("Done: {0} ({1:N2} MB)" -f $setup, ((Get-Item $setup).Length / 1MB))
Write-Host ("      {0} ({1:N2} MB)" -f $msi, ((Get-Item $msi).Length / 1MB))
Write-Host ""
Write-Host "Regular install:  run ZipMount-Setup.exe"
Write-Host "Silent install:   ZipMount-Setup.exe /quiet"
Write-Host "Uninstall:        ZipMount-Setup.exe /uninstall"
