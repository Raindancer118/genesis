#!/bin/bash
set -e

if [ "$EUID" -ne 0 ]; then
  echo "❌ Dieses Skript muss mit sudo ausgeführt werden: sudo $0"
  exit 1
fi

echo "🚀 Installing/Updating Genesis..."

REPO_URL="git@github.com:Raindancer118/genesis.git"
INSTALL_DIR="/opt/genesis"
BIN_DIR="/usr/local/bin"
APP_NAME="genesis"

# --- 1. Install or Update the Git Repository ---
if [ -d "$INSTALL_DIR" ]; then
    echo "Updating existing Genesis installation from Git..."
    cd "$INSTALL_DIR"
    echo "Pulling updates as user '$SUDO_USER'..."
    sudo -u "$SUDO_USER" git pull origin main
else
    # --- DIESER TEIL HAT GEFEHLT ---
    echo "Performing first-time install of Genesis from Git..."
    # Klone als der ursprüngliche Benutzer, um die korrekten SSH-Schlüssel zu verwenden
    sudo -u "$SUDO_USER" git clone "$REPO_URL" "/tmp/genesis"
    # Verschiebe den Ordner dann an den Zielort
    sudo mv "/tmp/genesis" "$INSTALL_DIR"
fi

cd "$INSTALL_DIR"

# --- 2. Check and Install Dependencies ---
echo "Checking dependencies..."
ALL_DEPS=(
    python-click
    python-rich
    python-pypdf
    python-pillow
    python-psutil
    clamav
    python-docx
    python-questionary
    python-google-generativeai
)
echo "-> Installing all required packages with pamac (skipping up-to-date)..."
sudo -u "$SUDO_USER" pamac install --no-confirm --needed "${ALL_DEPS[@]}"

# --- 3. Create Executable Link ---
echo "Creating system-wide command link..."
chmod +x genesis.py
ln -sf "$INSTALL_DIR/genesis.py" "$BIN_DIR/$APP_NAME"

# --- 4. Install Systemd User Services ---
echo "Setting up systemd user services (running as user '$SUDO_USER')..."
sudo -u "$SUDO_USER" bash -c '
    set -e
    USER_SERVICE_DIR="$HOME/.config/systemd/user"
    mkdir -p "$USER_SERVICE_DIR"
    cp /opt/genesis/genesis-greet.service "$USER_SERVICE_DIR/"
    cp /opt/genesis/genesis-sentry.service "$USER_SERVICE_DIR/"
    cp /opt/genesis/genesis-sentry.timer "$USER_SERVICE_DIR/"

    systemctl --user daemon-reload
    systemctl --user enable --now genesis-greet.service
    systemctl --user enable --now genesis-sentry.timer
'

# --- 5. Fix Permissions ---
echo "Setting correct ownership for $INSTALL_DIR..."
chown -R "$SUDO_USER":"$SUDO_USER" "$INSTALL_DIR"

echo "✅ Genesis installation complete."