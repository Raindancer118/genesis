#!/bin/bash
set -e

# --- 1. Sudo-Erzwingung ---
# Stellt sicher, dass das Skript als root läuft und $SUDO_USER gesetzt ist.
if [ "$EUID" -ne 0 ]; then
  echo "🚀 Sudo-Rechte erforderlich. Starte Skript neu mit sudo..."
  # %q sorgt für sicheres Quoting von Pfad und Argumenten
  sudo bash -c "$(printf "%q " "$0" "$@")"
  exit 0 # Beendet das ursprüngliche Skript ohne sudo
fi
# Ab hier läuft das Skript garantiert als root.

echo "🚀 Installing/Updating Genesis (as root)..."

# --- Konfiguration ---
REPO_URL="git@github.com:Raindancer118/genesis.git"
INSTALL_DIR="/opt/genesis"
BIN_DIR="/usr/local/bin"
APP_NAME="genesis"

# --- 2. Git Repository aktualisieren ---
if [ ! -d "$INSTALL_DIR" ]; then
    echo "Performing first-time install of Genesis from Git..."
    sudo -u "$SUDO_USER" git clone "$REPO_URL" "$INSTALL_DIR"
fi

cd "$INSTALL_DIR"
echo "Pulling updates as user '$SUDO_USER'..."
sudo -u "$SUDO_USER" git pull origin main

# --- 3. Abhängigkeiten installieren ---
echo "Checking dependencies as user '$SUDO_USER'..."
ALL_DEPS=(
    python-click python-rich python-pypdf python-pillow python-psutil
    clamav python-docx python-questionary python-google-generativeai
)
# pamac muss als der Benutzer ausgeführt werden, um auf den D-Bus zugreifen zu können
sudo -u "$SUDO_USER" pamac install --no-confirm --needed "${ALL_DEPS[@]}"

# --- 4. System-Links erstellen ---
echo "Creating system-wide command link..."
chmod +x genesis.py
ln -sf "$INSTALL_DIR/genesis.py" "$BIN_DIR/$APP_NAME"

# --- 5. Systemd User Services einrichten ---
echo "Setting up systemd user services as user '$SUDO_USER'..."
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

# --- 6. Berechtigungen korrigieren ---
echo "Setting correct ownership for $INSTALL_DIR..."
chown -R "$SUDO_USER":"$SUDO_USER" "$INSTALL_DIR"

echo "✅ Genesis installation complete."