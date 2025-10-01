#!/bin/bash
set -e

echo "🚀 Installing/Updating Genesis..."

REPO_URL="git@github.com:Raindancer118/genesis.git"
INSTALL_DIR="/opt/genesis"
# ... (rest of your variables)

# --- 1. Install or Update the Git Repository ---
if [ -d "$INSTALL_DIR" ]; then
    echo "Updating existing Genesis installation from Git..."
    cd "$INSTALL_DIR"

    # --- KORREKTUR HIER ---
    # Führe 'git pull' als der ursprüngliche Benutzer aus, der sudo aufgerufen hat.
    # $SUDO_USER wird automatisch zu 'tom' (oder wer auch immer sudo ausführt).
    echo "Pulling updates as user '$SUDO_USER'..."
    sudo -u "$SUDO_USER" git pull origin main

else
    echo "Performing first-time install of Genesis from Git..."
    # Klone als der aktuelle Benutzer, um die korrekten SSH-Schlüssel zu verwenden
    git clone "$REPO_URL" "/tmp/genesis"
    # Verschiebe den Ordner dann mit sudo an den Zielort
    sudo mv "/tmp/genesis" "$INSTALL_DIR"
fi

cd "$INSTALL_DIR"

# --- 2. Check and Install Dependencies ---
echo "Checking dependencies..."
OFFICIAL_DEPS=(python-click python-rich python-pypdf python-pillow python-psutil python-google-generativeai clamav)
AUR_DEPS=(python-docx python-questionary)

echo "-> Installing official packages with pacman..."
sudo pacman -S --needed --noconfirm "${OFFICIAL_DEPS[@]}"

echo "-> Installing AUR packages with pamac..."
pamac build --no-confirm "${AUR_DEPS[@]}"


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