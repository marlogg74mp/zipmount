# Publishes the built installers as a draft GitHub release.
#
#     powershell -ExecutionPolicy Bypass -File tools\build-installer.ps1
#     powershell -ExecutionPolicy Bypass -File tools\publish-release.ps1
#
# A draft rather than straight away: a release is public and lands in
# watchers' notifications, so the last look happens on the release page, one
# click to publish. GitHub creates the tag v<version> on publishing, at the
# latest commit of the default branch - so publish what has been pushed.
#
# Needs the GitHub CLI signed in to the repository owner's account:
#
#     winget install GitHub.cli
#     gh auth login
#
# -WhatIf prints what would be published, and the SHA256 of each file, without
# touching anything. Publishing the hashes is worth it while the installer is
# not code-signed.
#
# -StageOnly checks the version and copies the installers under their
# published names into target\release-<version>, and stops there: the release
# workflow does so, and makes one release of them with the Linux and macOS
# builds. Run by hand without it, the script publishes the Windows installers
# alone.

[CmdletBinding()]
param(
    [string] $Repository = "marlogg74mp/zipmount",
    [switch] $WhatIf,
    [switch] $StageOnly
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$target = Join-Path $root "target"

function Get-MsiVersion([string] $path) {
    $installer = New-Object -ComObject WindowsInstaller.Installer
    $db = $installer.GetType().InvokeMember("OpenDatabase", "InvokeMethod", $null, $installer, @($path, 0))
    $query = "SELECT Value FROM Property WHERE Property='ProductVersion'"
    $view = $db.GetType().InvokeMember("OpenView", "InvokeMethod", $null, $db, @($query))
    # Without [void] the return value of Execute ends up in the function's
    # output, which then yields two values instead of one - the version comes
    # back with a leading space.
    [void] $view.GetType().InvokeMember("Execute", "InvokeMethod", $null, $view, $null)
    $record = $view.GetType().InvokeMember("Fetch", "InvokeMethod", $null, $view, $null)
    if (-not $record) { throw "no ProductVersion in $path" }
    return [string] $record.GetType().InvokeMember("StringData", "GetProperty", $null, $record, 1)
}

# The release version comes from Cargo.toml, and the package is checked to
# have been built from exactly that. Otherwise yesterday's build is easily
# published under a new number.
$manifest = Get-Content (Join-Path $root "Cargo.toml") -Raw
$match = [regex]::Match($manifest, '(?ms)^\[workspace\.package\].*?^version\s*=\s*"(\d+\.\d+\.\d+)"')
if (-not $match.Success) { throw "no version in [workspace.package] in Cargo.toml" }
$Version = $match.Groups[1].Value
$built = Get-MsiVersion (Join-Path $target "ZipMount.msi")
if (-not $built.StartsWith("$Version.")) {
    throw "the package was built as $built, but Cargo.toml says $Version - rebuild: tools\build-installer.ps1"
}

$artifacts = @(
    @{ Source = "ZipMount-Setup.exe"; Published = "ZipMount-Setup-$Version.exe" },
    @{ Source = "ZipMount.msi";       Published = "ZipMount-$Version.msi" }
)

foreach ($item in $artifacts) {
    $path = Join-Path $target $item.Source
    if (-not (Test-Path $path)) {
        throw "no $path - build it: tools\build-installer.ps1"
    }
    $item.Path = $path
    $item.Hash = (Get-FileHash $path -Algorithm SHA256).Hash
    $item.Size = (Get-Item $path).Length
}

Write-Host "Release $Version (build $built)"
foreach ($item in $artifacts) {
    Write-Host ("  {0}  {1:N2} MB" -f $item.Published, ($item.Size / 1MB))
    Write-Host ("    SHA256 {0}" -f $item.Hash)
}

if ($WhatIf) {
    Write-Host ""
    Write-Host "Nothing done: -WhatIf given"
    return
}

if (-not $StageOnly -and -not (Get-Command gh -ErrorAction SilentlyContinue)) {
    throw "no GitHub CLI - winget install GitHub.cli, then gh auth login"
}

$staging = Join-Path $target "release-$Version"
Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory $staging | Out-Null
foreach ($item in $artifacts) {
    $item.Staged = Join-Path $staging $item.Published
    Copy-Item $item.Path $item.Staged
}

if ($StageOnly) {
    Write-Host ""
    Write-Host "Staged in $staging"
    return
}

# The same line format sha256sum -c understands, with LF line ends:
# Set-Content would write CRLF, and sha256sum then looks for a file whose name
# ends in a carriage return.
$sumsFile = Join-Path $staging "SHA256SUMS.txt"
$sums = $artifacts | ForEach-Object { "{0}  {1}`n" -f $_.Hash.ToLower(), $_.Published }
[IO.File]::WriteAllText($sumsFile, -join $sums, [Text.Encoding]::ASCII)

$exe = $artifacts[0]
$msi = $artifacts[1]
$notes = @"
Mounts zip, 7z, tar and tar.gz as an ordinary Windows drive - read-only, without extracting. Speaks English, Russian, Chinese, Japanese, Korean, Portuguese, Spanish and German.

| File | Size | Use |
|---|---|---|
| ``$($exe.Published)`` | $("{0:N2}" -f ($exe.Size / 1MB)) MB | regular install; installs WinFsp if it is missing |
| ``$($msi.Published)`` | $("{0:N2}" -f ($msi.Size / 1MB)) MB | when WinFsp is already there, or for deployment through group policy |

The installer is not code-signed yet, so Windows will warn about an unknown publisher. Check the hash before running it - ``SHA256SUMS.txt`` is attached:

``````
certutil -hashfile $($exe.Published) SHA256
``````

What the installer does and why WinFsp stays after uninstalling: see the [README](https://github.com/$Repository#installation).
"@
$notesFile = Join-Path $staging "notes.md"
Set-Content -Path $notesFile -Value $notes -Encoding UTF8

& gh release create "v$Version" $exe.Staged $msi.Staged $sumsFile `
    --repo $Repository `
    --title "ZipMount $Version" `
    --notes-file $notesFile `
    --draft
if ($LASTEXITCODE -ne 0) { throw "gh release create failed" }

Write-Host ""
Write-Host "Draft release created. Review and publish it:"
Write-Host "    https://github.com/$Repository/releases"
