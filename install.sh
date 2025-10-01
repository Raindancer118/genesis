#!/bin/bash
set -e

echo "🚀 Installing/Updating Genesis..."

# --- Konfiguration ---
REPO_URL="git@github.com:Raindancer118/genesis.git"
INSTALL_DIR="/opt/genesis"
BIN_DIR="/usr/local/bin"
APP_NAME="genesis"
USER=$(whoami) # Hol den aktuellen Benutzernamen

# --- 1. Abhängigkeiten installieren (als Benutzer) ---
echo "Checking dependencies..."
ALL_DEPS=(
    python-click python-rich python-pypdf python-pillow python-psutil
    clamav python-docx python-questionary python-google-generativeai
)
# pamac wird als normaler Benutzer ausgeführt und fragt bei Bedarf selbst nach dem sudo-Passwort.
# Das erhält den korrekten Kontext.
echo "-> Installing all required packages with pamac (skipping up-to-date)..."
pamac install --no-confirm --needed "${ALL_DEPS[@]}"


# --- 2. Git Repository aktualisieren (mit sudo für Dateirechte) ---
if [ -d "$INSTALL_DIR" ]; then
    echo "Updating existing Genesis installation from Git..."
    cd "$INSTALL_DIR"
    echo "Pulling updates as user '$USER'..."
    # 'sudo git pull' ist hier immer noch problematisch, wir korrigieren die Rechte danach
    sudo git pull origin main
else
    echo "Performing first-time install of Genesis from Git..."
    git clone "$REPO_URL" "/tmp/genesis"
    sudo mv "/tmp/genesis" "$INSTALL_DIR"
fi

cd "$INSTALL_DIR"

# --- 3. System-Links und Berechtigungen setzen (mit sudo) ---
echo "Creating system-wide command link and setting permissions..."
sudo chmod +x genesis.py
sudo ln -sf "$INSTALL_DIR/genesis.py" "$BIN_DIR/$APP_NAME"
sudo chown -R "$USER":"$USER" "$INSTALL_DIR" # Wichtig: Besitz zurück an den Benutzer geben

# --- 4. Systemd User Services einrichten (als Benutzer) ---
echo "Setting up systemd user services..."
USER_SERVICE_DIR="$HOME/.config/systemd/user"
mkdir -p "$USER_SERVICE_DIR"
# Kopiere die Dateien aus dem (jetzt wieder dir gehörenden) Verzeichnis
cp "./genesis-greet.service" "$USER_SERVICE_DIR/"
cp "./genesis-sentry.service" "$USER_SERVICE_DIR/"
cp "./genesis-sentry.timer" "$USER_SERVICE_DIR/"

# Diese Befehle werden jetzt als du selbst ausgeführt und funktionieren
systemctl --user daemon-reload
systemctl --user enable --now genesis-greet.service
systemctl --user enable --now genesis-sentry.timer

echo "✅ Genesis installation complete."