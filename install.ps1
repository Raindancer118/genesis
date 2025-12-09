<#
.SYNOPSIS
    Installs various dependencies and sets up Genesis on Windows.
.DESCRIPTION
    This script checks for Python and Git, creates a virtual environment,
    installs Python dependencies, and creates a 'genesis' command shim.
    It MUST be run as Administrator.
#>

$ErrorActionPreference = "Stop"

# -------------------------------
# 0) Must run as Administrator
# -------------------------------
$currentPrincipal = [Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
if (-not $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "🚀 Administrator privileges required. Restarting script as Admin..." -ForegroundColor Yellow
    Start-Process powershell.exe -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`"" -Verb RunAs
    exit
}

Write-Host "🚀 Installing/Updating Genesis (Windows)..." -ForegroundColor Cyan

# -------------------------------
# 1) Config
# -------------------------------
$InstallDir = "C:\Program Files\Genesis"
$BinDir = "$env:USERPROFILE\AppData\Local\Microsoft\WindowsApps" # Often in PATH
# Alternative: System32 or user-defined path. Using AppData...WindowsApps might be restricted or strictly for Store apps.
# Let's use a dedicated folder and add it to PATH if needed, or just %USERPROFILE%\bin
# Better standard: $env:LOCALAPPDATA\Programs\Genesis ?
# Since we are admin, let's put code in Program Files and the shim in a system folder or prompt user.
# For simplicity, let's assume C:\ProgramData\Genesis for repo and C:\Windows\System32 for shim is too aggressive.
# Let's stick to installing in Program Files and adding a shim to Windows folder (if we are admin)
# or just creating a bat file in C:\Windows (since we are admin).

$RepoUrl = "https://github.com/Raindancer118/genesis.git"

# -------------------------------
# 2) Dependencies Check
# -------------------------------
Write-Host "🧩 Checking dependencies..." -ForegroundColor Cyan

if (-not (Get-Command "git" -ErrorAction SilentlyContinue)) {
    Write-Host "Git not found. Please install Git for Windows." -ForegroundColor Red
    # Optional: winget install Git.Git
    if (Get-Command "winget" -ErrorAction SilentlyContinue) {
        $installGit = Read-Host "Install Git via winget? (Y/n)"
        if ($installGit -ne 'n') {
            winget install --id Git.Git -e --source winget
        } else {
            exit 1
        }
    } else {
        exit 1
    }
}

if (-not (Get-Command "python" -ErrorAction SilentlyContinue)) {
    Write-Host "Python not found. Please install Python 3." -ForegroundColor Red
    if (Get-Command "winget" -ErrorAction SilentlyContinue) {
        $installPy = Read-Host "Install Python 3.11 via winget? (Y/n)"
        if ($installPy -ne 'n') {
            winget install --id Python.Python.3.11 -e --source winget
        } else {
            exit 1
        }
    } else {
        exit 1
    }
}

# -------------------------------
# 3) Clone / Pull
# -------------------------------
if (-not (Test-Path $InstallDir)) {
    Write-Host "📦 First-time clone to $InstallDir..." -ForegroundColor Cyan
    git clone "$RepoUrl" "$InstallDir"
}

Set-Location "$InstallDir"

Write-Host "🔄 Pulling updates..." -ForegroundColor Cyan
git pull

# -------------------------------
# 4) Python Venv
# -------------------------------
$VenvDir = "$InstallDir\.venv"
$PythonExec = "python"

Write-Host "🧪 Preparing Python virtual environment..." -ForegroundColor Cyan

if (-not (Test-Path $VenvDir)) {
    & $PythonExec -m venv "$VenvDir"
}

$PipExec = "$VenvDir\Scripts\pip.exe"
if (Test-Path $PipExec) {
    & $PipExec install --upgrade pip
    # Install dependencies
    # Note: excluding system-specific ones like 'psutil' usually has wheels.
    # 'clamav' might be tricky if it's a binding.
    # We will try to install them.
    # List matches install.sh largely
    $Packages = @("click", "rich", "pypdf", "pillow", "psutil", "python-docx", "questionary", "google-generativeai")
    
    & $PipExec install $Packages
} else {
    Write-Host "⚠️  Virtualenv created but pip not found." -ForegroundColor Yellow
}

# -------------------------------
# 5) Shim / Command Link
# -------------------------------
Write-Host "🔗 Creating system-wide command..." -ForegroundColor Cyan

$ShimPath = "C:\Windows\genesis.bat" 
# Writing to C:\Windows is standard for 'system-wide' binaries in simple scripts if Admin.
# Content of the shim:
$ShimContent = "@echo off`r`n`"$VenvDir\Scripts\python.exe`" `"$InstallDir\genesis.py`" %*"
Set-Content -Path $ShimPath -Value $ShimContent
# Remove .py potentially if user typed genesis.py before? No need.

Write-Host "✅ Genesis installation complete. Type 'genesis' to start." -ForegroundColor Green
