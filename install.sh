#!/bin/bash
set -e

if [ "$EUID" -ne 0 ]; then
  echo "❌ Dieses Skript muss mit sudo ausgeführt werden: sudo $0"
  exit 1
fi

echo "🚀 Installing/Updating Genesis..."

REPO_URL="git@github.com:Raindancer118/genesis.git"
INSTALL_DIR="/opt/genesis"
# ... (rest of your variables)

# --- 1. Install or Update the Git Repository ---
# (Dieser Teil bleibt unverändert)
if [ -d "$INSTALL_DIR" ]; then
    echo "Updating existing Genesis installation from Git..."
    cd "$INSTALL_DIR"
    echo "Pulling updates as user '$SUDO_USER'..."
    sudo -u "$SUDO_USER" git pull origin main
else
    # ... (Clone-Logik)
fi

cd "$INSTALL_DIR"

# --- 2. Check and Install Dependencies (Optimized) ---
echo "Checking dependencies..."

# Eine einzige Liste für alle Pakete
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
# Ein einziger, intelligenter Befehl für alles. --needed überspringt aktuelle Pakete.
sudo -u "$SUDO_USER" pamac install --no-confirm --needed "${ALL_DEPS[@]}"

# --- 3. Create Executable Link ---
echo "Creating system-wide command link..."
sudo chmod +x genesis.py
sudo ln -sf "$INSTALL_DIR/genesis.py" "/usr/local/bin/genesis"


# --- 4. Install Systemd Services ---
echo "Setting up systemd user services..."
USER_SERVICE_DIR="$HOME/.config/systemd/user"
mkdir -p "$USER_SERVICE_DIR"
cp "./genesis-greet.service" "$USER_SERVICE_DIR/"
cp "./genesis-sentry.service" "$USER_SERVICE_DIR/"
cp "./genesis-sentry.timer" "$USER_SERVICE_DIR/"
systemctl --user daemon-reload
systemctl --user enable --now genesis-greet.service
systemctl --user enable --now genesis-sentry.timer


# --- 5. Fix Permissions ---
echo "Setting correct ownership for $INSTALL_DIR..."
# $SUDO_USER ist der Benutzer, der den sudo-Befehl ursprünglich ausgeführt hat
if [ -n "$SUDO_USER" ]; then
    sudo chown -R "$SUDO_USER":"$SUDO_USER" "$INSTALL_DIR"
else
    sudo chown -R "$(whoami)":"$(whoami)" "$INSTALL_DIR"
fi


echo "✅ Genesis installation complete."