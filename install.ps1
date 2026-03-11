# ──────────────────────────────────────────────────────────────
#  Volantic Genesis — Windows Installer
#  Usage: irm https://raw.githubusercontent.com/Raindancer118/genesis/main/install.ps1 | iex
# ──────────────────────────────────────────────────────────────
#Requires -Version 5.1
$ErrorActionPreference = 'Stop'

$REPO     = "Raindancer118/genesis"
$BIN_NAME = "vg.exe"
$API_URL  = "https://api.github.com/repos/$REPO/releases/latest"
$ARTIFACT = "vg-x86_64-windows.zip"
$INSTALL_DIR = "$env:LOCALAPPDATA\Volantic\bin"

# ── Helpers ───────────────────────────────────────────────────
function Write-Header {
    Write-Host ""
    Write-Host "  " -NoNewline
    Write-Host "V O L A N T I C   G E N E S I S" -ForegroundColor Cyan -NoNewline
    Write-Host ""
    Write-Host "  ──────────────────────────────────" -ForegroundColor DarkGray
    Write-Host "  INSTALLER" -ForegroundColor White
    Write-Host ""
}

function Write-Info  ($msg) { Write-Host "  " -NoNewline; Write-Host "·" -ForegroundColor DarkGray -NoNewline; Write-Host " $msg" }
function Write-Ok    ($msg) { Write-Host "  " -NoNewline; Write-Host "✓" -ForegroundColor Green   -NoNewline; Write-Host " $msg" }
function Write-Fail  ($msg) { Write-Host "  " -NoNewline; Write-Host "✗" -ForegroundColor Red     -NoNewline; Write-Host " $msg" }

# ── Fetch latest release ──────────────────────────────────────
function Get-DownloadUrl {
    Write-Info "Fetching latest release..."
    try {
        $release = Invoke-RestMethod -Uri $API_URL -Headers @{ 'User-Agent' = 'vg-installer' }
    } catch {
        Write-Fail "Failed to contact GitHub API: $_"
        exit 1
    }

    $asset = $release.assets | Where-Object { $_.name -eq $ARTIFACT } | Select-Object -First 1
    if (-not $asset) {
        Write-Fail "Artifact '$ARTIFACT' not found in latest release."
        Write-Fail "Make sure a release exists at: https://github.com/$REPO/releases"
        exit 1
    }

    Write-Info "Latest version: $($release.tag_name)"
    return $asset.browser_download_url
}

# ── Download & extract ────────────────────────────────────────
function Install-Binary ($url) {
    $tmp = Join-Path $env:TEMP "vg-install-$([System.IO.Path]::GetRandomFileName())"
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null

    $zipPath = Join-Path $tmp $ARTIFACT
    Write-Info "Downloading $ARTIFACT..."
    try {
        $wc = New-Object System.Net.WebClient
        $wc.DownloadFile($url, $zipPath)
    } catch {
        Write-Fail "Download failed: $_"
        exit 1
    }

    Write-Info "Extracting..."
    Expand-Archive -Path $zipPath -DestinationPath $tmp -Force

    $exePath = Join-Path $tmp "vg.exe"
    if (-not (Test-Path $exePath)) {
        Write-Fail "vg.exe not found in archive."
        exit 1
    }

    New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null
    Copy-Item $exePath (Join-Path $INSTALL_DIR $BIN_NAME) -Force
    Remove-Item $tmp -Recurse -Force

    Write-Ok "Installed to $INSTALL_DIR\$BIN_NAME"
}

# ── PATH setup ────────────────────────────────────────────────
function Add-ToPath {
    $current = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($current -notlike "*$INSTALL_DIR*") {
        [Environment]::SetEnvironmentVariable("PATH", "$current;$INSTALL_DIR", "User")
        $env:PATH += ";$INSTALL_DIR"
        Write-Ok "Added $INSTALL_DIR to your PATH (user scope)"
        Write-Info "Restart your terminal for PATH changes to take effect"
    } else {
        Write-Info "$INSTALL_DIR already in PATH"
    }
}

# ── Main ──────────────────────────────────────────────────────
Write-Header

# Check arch
$arch = $env:PROCESSOR_ARCHITECTURE
if ($arch -notin @('AMD64', 'x86_64')) {
    Write-Fail "Unsupported architecture: $arch. Only x64 is supported."
    exit 1
}

$url = Get-DownloadUrl
Install-Binary $url
Add-ToPath

Write-Host ""
Write-Ok "Installation complete!"
Write-Host ""
Write-Host "  Run " -NoNewline
Write-Host "vg --help" -ForegroundColor Cyan -NoNewline
Write-Host " to get started."
Write-Host ""
