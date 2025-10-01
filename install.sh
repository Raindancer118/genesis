#!/bin/bash

echo "🚀 Installing Genesis..."

# --- Configuration ---
INSTALL_DIR="/opt/genesis"
BIN_DIR="/usr/local/bin"
APP_NAME="genesis"
SERVICE_FILE="genesis-greet.service"
USER_SERVICE_DIR="$HOME/.config/systemd/user"

# --- 1. Check Dependencies ---
echo "Checking dependencies..."
sudo pacman -S --needed python-click python-rich # Add other deps here

# --- 2. Create Directories and Copy Files ---
echo "Installing application files to $INSTALL_DIR..."
sudo mkdir -p $INSTALL_DIR
sudo cp -r ./commands "$INSTALL_DIR/"
sudo cp ./genesis.py "$INSTALL_DIR/"

# --- 3. Create Executable Link ---
echo "Creating system-wide command link..."
sudo ln -sf "$INSTALL_DIR/genesis.py" "$BIN_DIR/$APP_NAME"
sudo chmod +x "$INSTALL_DIR/genesis.py"

# --- 4. Install and Enable Systemd User Service ---
echo "Setting up startup greeting service..."
mkdir -p "$USER_SERVICE_DIR"
cp "./$SERVICE_FILE" "$USER_SERVICE_DIR/"
systemctl --user enable --now "$SERVICE_FILE"

echo "✅ Genesis installation complete."
echo "Try running 'genesis --help' or log out and back in to see the greeting."