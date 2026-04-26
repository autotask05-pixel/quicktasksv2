param(
    [string]$Variant = "ui"
)

$REPO = "autotask05-pixel/quicktasksv2"
$BINARY_NAME = "quicktasks.exe"

# -------- Variant Mapping --------
switch ($Variant) {
    "ui" { $REAL_VARIANT = "ui" }
    "noui" { $REAL_VARIANT = "default" }
    "fire-and-forget" { $REAL_VARIANT = "fire-and-forget" }
    "fire-and-forget-ui" { $REAL_VARIANT = "fire-and-forget-ui" }
    default {
        Write-Host " Invalid variant: $Variant"
        exit 1
    }
}

Write-Host "🎯 Variant: $Variant → $REAL_VARIANT"

# -------- ARCH --------
$ARCH = $env:PROCESSOR_ARCHITECTURE

switch ($ARCH) {
    "AMD64" { $TARGET = "x86_64-pc-windows-msvc" }
    "ARM64" { $TARGET = "aarch64-pc-windows-msvc" }
    default {
        Write-Host " Unsupported architecture: $ARCH"
        exit 1
    }
}

# -------- VERSION --------
$VERSION = (Invoke-RestMethod "https://api.github.com/repos/$REPO/releases/latest").tag_name

if (-not $VERSION) {
    Write-Host " Failed to fetch version"
    exit 1
}

$FILE = "quicktasks-$REAL_VARIANT-$TARGET.zip"
$URL = "https://github.com/$REPO/releases/download/$VERSION/$FILE"

Write-Host "⬇️ $URL"

$TMP = New-Item -ItemType Directory -Force -Path ([System.IO.Path]::GetTempPath() + [System.Guid]::NewGuid())

$ZIP = "$TMP\pkg.zip"

Invoke-WebRequest $URL -OutFile $ZIP
Expand-Archive $ZIP -DestinationPath $TMP -Force

# -------- INSTALL PATH --------
$INSTALL_DIR = "$env:LOCALAPPDATA\quicktasks"
New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null

Move-Item "$TMP\$BINARY_NAME" "$INSTALL_DIR\$BINARY_NAME" -Force

# -------- PATH SET --------
$CURRENT_PATH = [Environment]::GetEnvironmentVariable("Path", "User")

if ($CURRENT_PATH -notlike "*$INSTALL_DIR*") {
    [Environment]::SetEnvironmentVariable("Path", "$CURRENT_PATH;$INSTALL_DIR", "User")
    Write-Host "⚙️ Added to PATH"
}

Remove-Item $TMP -Recurse -Force

Write-Host "✅ Installed → quicktasks"
Write-Host "👉 Restart terminal if command not found"
