param(
    [Parameter(Position = 0)]
    [string]$Repo = $env:EFFICEINTNLP_REPO,

    [Parameter(Position = 1)]
    [string]$Version = $(if ($env:EFFICEINTNLP_VERSION) { $env:EFFICEINTNLP_VERSION } else { "latest" }),

    [Parameter(Position = 2)]
    [string]$Variant = $(if ($env:EFFICEINTNLP_VARIANT) { $env:EFFICEINTNLP_VARIANT } else { "default" })
)

$ErrorActionPreference = "Stop"

if (-not $Repo) {
    Write-Error "Usage: iwr https://raw.githubusercontent.com/OWNER/REPO/main/install.ps1 -useb -OutFile install.ps1; ./install.ps1 OWNER/REPO [version] [variant]"
}

switch ($Variant) {
    "default" { }
    "fire-and-forget" { }
    "default-ui" { }
    "fire-and-forget-ui" { }
    default { throw "Unsupported variant: $Variant. Supported variants: default, fire-and-forget, default-ui, fire-and-forget-ui" }
}

$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
switch ($arch) {
    "X64" { $archive = "QUICKTASKS-$Variant-x86_64-pc-windows-msvc.zip" }
    default { throw "Unsupported architecture: $arch" }
}

if ($Version -eq "latest") {
    $downloadUrl = "https://github.com/$Repo/releases/latest/download/$archive"
} else {
    $downloadUrl = "https://github.com/$Repo/releases/download/$Version/$archive"
}

$installRoot = if ($env:EFFICEINTNLP_INSTALL_DIR) { $env:EFFICEINTNLP_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "QUICKTASKS" }
$binDir = if ($env:EFFICEINTNLP_BIN_DIR) { $env:EFFICEINTNLP_BIN_DIR } else { Join-Path $env:LOCALAPPDATA "Microsoft\WindowsApps" }
$tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ("QUICKTASKS-" + [System.Guid]::NewGuid().ToString("N"))

New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null
New-Item -ItemType Directory -Force -Path $installRoot | Out-Null
New-Item -ItemType Directory -Force -Path $binDir | Out-Null

$archivePath = Join-Path $tmpDir "QUICKTASKS.zip"

Write-Host "Downloading $downloadUrl"
try {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $archivePath
} catch {
    throw "Failed to download $downloadUrl. This installer only works with published GitHub release assets. GitHub Actions run artifacts are not available at releases/latest/download/... Create or use a tagged release that includes $archive, then retry."
}

Get-ChildItem -Force $installRoot | Remove-Item -Recurse -Force
Expand-Archive -Path $archivePath -DestinationPath $installRoot -Force

$launcherPath = Join-Path $binDir "QUICKTASKS.cmd"
$binaryPath = Join-Path $installRoot "QUICKTASKS.exe"

@"
@echo off
"$binaryPath" %*
"@ | Set-Content -Path $launcherPath -Encoding ASCII

Remove-Item -Recurse -Force $tmpDir

Write-Host "Installed to: $installRoot"
Write-Host "Launcher:     $launcherPath"
Write-Host "Variant:      $Variant"
Write-Host ""
Write-Host "Next:"
Write-Host "  1. Run: QUICKTASKS"
Write-Host "  2. The first startup downloads default models if they are missing."
