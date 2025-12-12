<#
.SYNOPSIS
    Installs dependencies and builds Genesis (Rust) on Windows.
.DESCRIPTION
    Checks for Git, Rust (Cargo), installs them if missing (via Winget/Rustup).
    Builds the project in Release mode.
    Adds the binary to PATH or creates a shim.
    Must run as Administrator.
#>

$ErrorActionPreference = "Stop"

# -------------------------------
# 0) Administrator Check
# -------------------------------
$currentPrincipal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host ">>> Administrator privileges required. Restarting script as Admin..." -ForegroundColor Yellow
    Start-Process powershell.exe -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`"" -Verb RunAs
    exit
}

Write-Host ">>> Installing/Updating Genesis (Rust Edition)..." -ForegroundColor Cyan

# -------------------------------
# 1) Config
# -------------------------------
$InstallDir = "C:\Program Files\Genesis"
$RepoUrl = "https://github.com/Raindancer118/genesis.git"

# -------------------------------
# 2) Dependencies Check
# -------------------------------
Write-Host ">>> Checking dependencies..." -ForegroundColor Cyan

# Git
if (-not (Get-Command "git" -ErrorAction SilentlyContinue)) {
    Write-Host "Git not found." -ForegroundColor Red
    if (Get-Command "winget" -ErrorAction SilentlyContinue) {
        $install = Read-Host "Install Git via winget? (Y/n)"
        if ($install -ne 'n') {
            winget install --id Git.Git -e --source winget
            # Refresh env vars roughly? Or tell user to restart.
            Write-Host "Please restart the script after Git installation to update PATH." -ForegroundColor Yellow
            exit
        } else {
            exit 1
        }
    } else {
        exit 1
    }
}

# Rust
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Host "Rust (Cargo) not found." -ForegroundColor Yellow
    $install = Read-Host "Install Rust via rustup? (Y/n)"
    if ($install -ne 'n') {
        # Download rustup-init
        $rustupUrl = "https://win.rustup.rs/x86_64"
        $rustupExe = "$env:TEMP\rustup-init.exe"
        Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupExe
        Write-Host "Running rustup-init..."
        Start-Process -FilePath $rustupExe -ArgumentList "-y" -Wait
        
        # Add to current session PATH
        $env:PATH += ";$env:USERPROFILE\.cargo\bin"
        if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
             Write-Host "Cargo installed but not found in PATH yet. You may need to restart the script." -ForegroundColor Yellow
             exit
        }
    } else {
        exit 1
    }
}

# -------------------------------
# 3) Clone / Pull
# -------------------------------
if (-not (Test-Path $InstallDir)) {
    Write-Host ">>> First-time clone into $InstallDir..." -ForegroundColor Cyan
    git clone "$RepoUrl" "$InstallDir"
}

Set-Location "$InstallDir"
Write-Host ">>> Pulling updates..." -ForegroundColor Cyan
git pull

# -------------------------------
# 4) Build
# -------------------------------
Write-Host ">>> Building Release..." -ForegroundColor Cyan
cargo build --release

$TargetBin = "$InstallDir\target\release\genesis.exe"
if (-not (Test-Path $TargetBin)) {
    Write-Host "Build failed. Binary not found." -ForegroundColor Red
    exit 1
}

# -------------------------------
# 5) Shim / Path
# -------------------------------
Write-Host ">>> Setting up command..." -ForegroundColor Cyan
# Create a .bat shim in Windows directory for global access
$ShimPath = "C:\Windows\genesis.bat"
$ShimContent = "@echo off`r`n`"$TargetBin`" %*"
Set-Content -Path $ShimPath -Value $ShimContent

Write-Host "✅ Genesis installation complete. Type 'genesis' to start." -ForegroundColor Green
