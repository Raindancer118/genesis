#!/bin/bash

# This script handles the installation and updates for the Genesis tool.
# It ensures the Git repository is in place, installs all dependencies from the
# correct sources (pacman/AUR), and sets up system links and services.

set -e # Exit immediately if a command exits with a non-zero status.

echo "🚀 Installing/Updating Genesis..."

# --- Configuration ---
# Using the SSH URL is recommended for automated tasks (like self-updating).
# Make sure you've added your SSH key to your GitHub account.
REPO_URL="git@github.com:Raindancer118/genesis.git"
INSTALL_DIR="/opt/genesis"
BIN_DIR="/usr/local/bin"
APP_NAME="genesis"
CONFIG_DIR="$HOME/.config/genesis"
USER_SERVICE_DIR="$HOME/.config/systemd/user"

# --- 1. Install or Update the Git Repository ---
if [ -d "$INSTALL_DIR" ]; then
    echo "Updating existing Genesis installation from Git..."
    cd "$INSTALL_DIR"
    sudo git pull origin main
else
    echo "Performing first-time install of Genesis from Git..."
    # We clone as the current user, then change ownership to root
    # This helps with SSH key authentication.
    git clone "$REPO_URL" "/tmp/genesis"
    sudo mv "/tmp/genesis" "$INSTALL_DIR"
fi

# All subsequent commands run from the installation directory
cd "$INSTALL_DIR"

# --- 2. Check and Install Dependencies ---
echo "Checking dependencies..."

# Separate lists for official repositories (pacman) and the AUR (pamac)
OFFICIAL_DEPS=(
    python-click
    python-rich
    python-pypdf
    python-pillow
    python-psutil
    clamav
)
AUR_DEPS=(
    python-docx
    python-questionary
    python-google-generativeai
)

echo "-> Installing official packages with pacman..."
sudo pacman -S --needed --noconfirm "${OFFICIAL_DEPS[@]}"

echo "-> Installing AUR packages with pamac..."
# Pamac does not need sudo
pamac build --no-confirm "${AUR_DEPS[@]}"


# --- 3. Create Executable Link ---
echo "Creating system-wide command link at $BIN_DIR/$APP_NAME..."
# The main python script should be executable
sudo chmod +x genesis.py
# Link it to a location in the system's PATH
sudo ln -sf "$INSTALL_DIR/genesis.py" "$BIN_DIR/$APP_NAME"


# --- 4. Install and Enable Systemd User Services ---
echo "Setting up systemd user services..."
mkdir -p "$USER_SERVICE_DIR"
# The service files are now being copied from the cloned repo
cp "./genesis-greet.service" "$USER_SERVICE_DIR/"
cp "./genesis-sentry.service" "$USER_SERVICE_DIR/"
cp "./genesis-sentry.timer" "$USER_SERVICE_DIR/"

# Reload systemd to recognize the new files and enable them
systemctl --user daemon-reload
systemctl --user enable --now genesis-greet.service
systemctl --user enable --now genesis-sentry.timer


# --- 5. Create Config Directory ---
# This directory is for user-specific configurations, like the sorter memory
mkdir -p "$CONFIG_DIR"


echo "✅ Genesis installation complete."
echo "Try running 'genesis --help' or log out and back in to see the greeting."