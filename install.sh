#!/bin/bash

echo "🚀 Installing/Updating Genesis..."

set -e # Exit immediately if a command exits with a non-zero status.

# --- Configuration ---
# !!! IMPORTANT: Change this URL to your actual public GitHub repository URL !!!
REPO_URL="https://github.com/Raindancer118/genesis.git"
INSTALL_DIR="/opt/genesis"
BIN_DIR="/usr/local/bin"
APP_NAME="genesis"
SERVICE_FILE="genesis-greet.service"
CONFIG_DIR="$HOME/.config/genesis"
USER_SERVICE_DIR="$HOME/.config/systemd/user"

# --- 1. Install or Update the Main Application ---
if [ -d "$INSTALL_DIR" ]; then
    echo "Updating existing Genesis installation..."
    cd "$INSTALL_DIR"
    sudo git pull origin main
else
    echo "Performing first-time install of Genesis..."
    sudo git clone "$REPO_URL" "$INSTALL_DIR"
fi

cd "$INSTALL_DIR"

# --- 2. Check Dependencies ---
echo "Checking dependencies..."
sudo pacman -S --needed --noconfirm python-questionary python-click python-rich python-pypdf python-docx python-pillow clamav

# --- 3. Create Directories and Copy Files ---
echo "Installing application files to $INSTALL_DIR..."
sudo rm -rf "$INSTALL_DIR" # Clean up old installation
sudo mkdir -p "$INSTALL_DIR"
sudo cp -r ./commands "$INSTALL_DIR/"
sudo cp ./genesis.py "$INSTALL_DIR/"
sudo mkdir -p "$CONFIG_DIR"

# --- 4. Create Executable Link ---
echo "Creating system-wide command link..."
sudo ln -sf "$INSTALL_DIR/genesis.py" "$BIN_DIR/$APP_NAME"
sudo chmod +x "$INSTALL_DIR/genesis.py"

# --- 5. Install and Enable Systemd User Service ---
echo "Setting up startup greeting service..."
mkdir -p "$USER_SERVICE_DIR"
cp "./$SERVICE_FILE" "$USER_SERVICE_DIR/"
systemctl --user daemon-reload
systemctl --user enable --now "$SERVICE_FILE"

# --- 6. Install and Enable Background Sentry Service ---
echo "Setting up background sentry service..."
cp "./genesis-sentry.service" "$USER_SERVICE_DIR/"
cp "./genesis-sentry.timer" "$USER_SERVICE_DIR/"
systemctl --user daemon-reload
systemctl --user enable --now genesis-sentry.timer

echo "✅ Genesis installation complete."
echo "Try running 'genesis --help' or log out and back in to see the greeting."