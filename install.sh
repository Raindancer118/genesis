#!/bin/bash
set -euo pipefail

# -------------------------------
# 0) Must run as root
# -------------------------------
if [[ "${EUID}" -ne 0 ]]; then
  echo "🚀 Sudo-Rechte erforderlich. Starte Skript neu mit sudo…"
  exec sudo --preserve-env=PATH,BASH_ENV "$0" "$@"
fi
echo "🚀 Installing/Updating Genesis (as root)..."

# -------------------------------
# 1) Resolve target user
# -------------------------------
SUDO_USER_REAL="${SUDO_USER:-}"
if [[ -z "${SUDO_USER_REAL}" || "${SUDO_USER_REAL}" == "root" ]]; then
  # Best effort fallback: aktiver TTY-User
  SUDO_USER_REAL="$(logname 2>/dev/null || true)"
fi
if [[ -z "${SUDO_USER_REAL}" || "${SUDO_USER_REAL}" == "root" ]]; then
  echo "❌ Konnte Zielnutzer nicht ermitteln (SUDO_USER/logname)."
  echo "   Bitte mit 'sudo -u <user> sudo ./install.sh' ausführen."
  exit 1
fi
echo "➡️  Zielnutzer: ${SUDO_USER_REAL}"
USER_UID="$(id -u "${SUDO_USER_REAL}")"

# -------------------------------
# 2) Config
# -------------------------------
REPO_URL="https://github.com/Raindancer118/genesis.git"
INSTALL_DIR="/opt/genesis"
BIN_DIR="/usr/local/bin"
APP_NAME="genesis"

# -------------------------------
# 3) Clone / Pull als User
# -------------------------------
if [[ ! -d "${INSTALL_DIR}" ]]; then
  echo "📦 First-time clone nach ${INSTALL_DIR}…"
  sudo -u "${SUDO_USER_REAL}" git clone "${REPO_URL}" "${INSTALL_DIR}"
fi
cd "${INSTALL_DIR}"

# Fix ownership before git operations to avoid permission errors
if [[ -d "${INSTALL_DIR}/.git" ]]; then
  chown -R "${SUDO_USER_REAL}:${SUDO_USER_REAL}" "${INSTALL_DIR}"
fi

# Determine current branch and tracking branch
CURRENT_BRANCH="$(sudo -u "${SUDO_USER_REAL}" git rev-parse --abbrev-ref HEAD 2>/dev/null)"
if [[ -z "${CURRENT_BRANCH}" || "${CURRENT_BRANCH}" == "HEAD" ]]; then
  # Detached HEAD state - try to get the default branch from remote
  CURRENT_BRANCH="$(sudo -u "${SUDO_USER_REAL}" git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@')"
  if [[ -z "${CURRENT_BRANCH}" ]]; then
    # Fallback to main if we can't determine
    CURRENT_BRANCH="main"
  fi
fi

TRACKING_BRANCH="$(sudo -u "${SUDO_USER_REAL}" git rev-parse --abbrev-ref --symbolic-full-name @{u} 2>/dev/null || echo "origin/${CURRENT_BRANCH}")"

echo "🔄 Pulling updates as '${SUDO_USER_REAL}' (branch: ${CURRENT_BRANCH})…"
if sudo -u "${SUDO_USER_REAL}" git pull --ff-only "${TRACKING_BRANCH%%/*}" "${CURRENT_BRANCH}" 2>/dev/null; then
  echo "✅ Git pull successful."
else
  echo "⚠️  Fast-forward pull failed. Trying standard pull…"
  sudo -u "${SUDO_USER_REAL}" git pull "${TRACKING_BRANCH%%/*}" "${CURRENT_BRANCH}" || {
    echo "❌ Git pull failed. Continuing with current version…"
  }
fi

# -------------------------------
# 4) Dependencies
# -------------------------------
echo "🧩 Checking dependencies…"
ARCH_PACKAGES=(python python-pip python-virtualenv git clamav maven jdk-openjdk)
DEBIAN_PACKAGES=(python3 python3-pip python3-venv git clamav maven default-jdk)
PYTHON_PACKAGES=(click rich pypdf pillow psutil python-docx questionary google-generativeai)

if command -v pamac >/dev/null 2>&1; then
  echo "→ Install via pamac"
  sudo -u "${SUDO_USER_REAL}" pamac install --no-confirm --needed "${ARCH_PACKAGES[@]}" || true
elif command -v pacman >/dev/null 2>&1; then
  echo "→ Install via pacman"
  pacman -Sy --noconfirm --needed "${ARCH_PACKAGES[@]}" || true
elif command -v apt-get >/dev/null 2>&1 || command -v apt >/dev/null 2>&1; then
  echo "→ Install via apt"
  APT_BIN="$(command -v apt-get || command -v apt)"
  "${APT_BIN}" update
  "${APT_BIN}" install -y "${DEBIAN_PACKAGES[@]}"
else
  echo "⚠️ Kein unterstützter Paketmanager gefunden. Bitte Abhängigkeiten manuell installieren."
fi

PYTHON_EXEC="$(command -v python3 || command -v python || true)"
VENV_DIR="${INSTALL_DIR}/.venv"
if [[ -n "${PYTHON_EXEC}" ]]; then
  echo "🧪 Preparing Python virtual environment…"
  if [[ -d "${VENV_DIR}" ]]; then
    sudo -u "${SUDO_USER_REAL}" "${PYTHON_EXEC}" -m venv --upgrade "${VENV_DIR}" \
      || echo "⚠️  Konnte bestehendes Virtualenv nicht aktualisieren."
  else
    sudo -u "${SUDO_USER_REAL}" "${PYTHON_EXEC}" -m venv "${VENV_DIR}" \
      || echo "⚠️  Konnte Virtualenv nicht erstellen."
  fi

  VENV_PIP="${VENV_DIR}/bin/pip"
  if [[ -x "${VENV_PIP}" ]]; then
    sudo -u "${SUDO_USER_REAL}" "${VENV_PIP}" install --upgrade pip \
      || echo "⚠️  Pip-Upgrade im Virtualenv fehlgeschlagen."
    sudo -u "${SUDO_USER_REAL}" "${VENV_PIP}" install "${PYTHON_PACKAGES[@]}" \
      || echo "⚠️  Python-Abhängigkeiten konnten nicht vollständig installiert werden."
  else
    echo "⚠️  Virtualenv wurde erstellt, aber pip fehlt. Bitte prüfen Sie die Python-Installation."
  fi
else
  echo "⚠️ Keine Python-Laufzeit gefunden. Bitte Python 3 installieren."
fi

# -------------------------------
# 5) Initialize persistent storage directories
# -------------------------------
echo "📂 Initializing Genesis storage directories…"
# Use a dedicated Python script to avoid shell injection risks
if ! sudo -u "${SUDO_USER_REAL}" "${VENV_DIR}/bin/python" "${INSTALL_DIR}/init_storage.py" 2>&1; then
  echo "⚠️  Could not initialize storage directories. They will be created on first use."
fi

# -------------------------------
# 6) Symlink anlegen
# -------------------------------
echo "🔗 Creating system-wide command link…"
chmod +x "${INSTALL_DIR}/genesis.py"
ln -sf "${INSTALL_DIR}/genesis.py" "${BIN_DIR}/${APP_NAME}"

# -------------------------------
# 7) systemd USER units zuverlässig aktivieren
#    (Fix für: DBUS_SESSION_BUS_ADDRESS / XDG_RUNTIME_DIR)
# -------------------------------
echo "🛠️  Preparing systemd user environment…"
# Linger erlaubt einen User-Manager ohne aktive Login-Session
loginctl enable-linger "${SUDO_USER_REAL}" >/dev/null 2>&1 || true

# Laufzeitverzeichnis und Bus-Var setzen (existieren ggf. erst nach Linger)
XRD="/run/user/${USER_UID}"
DBUS_ADDR="unix:path=${XRD}/bus"

# Falls das runtime dir nicht existiert, kurz anstoßen
if [[ ! -d "${XRD}" ]]; then
  # Start a minimal user scope to make /run/user/$UID appear
  sudo -u "${SUDO_USER_REAL}" systemd-run --user --scope true >/dev/null 2>&1 || true
  sleep 0.5
fi

# Jetzt die User-Units deployen + aktivieren mit korrekten Env-Variablen
echo "⚙️  Deploy user services…"
sudo -u "${SUDO_USER_REAL}" mkdir -p "/home/${SUDO_USER_REAL}/.config/systemd/user"
sudo -u "${SUDO_USER_REAL}" cp -f \
  "${INSTALL_DIR}/genesis-greet.service" \
  "${INSTALL_DIR}/genesis-sentry.service" \
  "${INSTALL_DIR}/genesis-sentry.timer" \
  "/home/${SUDO_USER_REAL}/.config/systemd/user/"

# Daemon-Reload & Enable/Start (mit expliziten Env-Variablen)
sudo -u "${SUDO_USER_REAL}" \
  XDG_RUNTIME_DIR="${XRD}" \
  DBUS_SESSION_BUS_ADDRESS="${DBUS_ADDR}" \
  systemctl --user daemon-reload

sudo -u "${SUDO_USER_REAL}" \
  XDG_RUNTIME_DIR="${XRD}" \
  DBUS_SESSION_BUS_ADDRESS="${DBUS_ADDR}" \
  systemctl --user enable --now genesis-greet.service

sudo -u "${SUDO_USER_REAL}" \
  XDG_RUNTIME_DIR="${XRD}" \
  DBUS_SESSION_BUS_ADDRESS="${DBUS_ADDR}" \
  systemctl --user enable --now genesis-sentry.timer

# -------------------------------
# 8) Ownership fix
# -------------------------------
echo "🔒 Fix ownership…"
chown -R "${SUDO_USER_REAL}:${SUDO_USER_REAL}" "${INSTALL_DIR}"

echo "✅ Genesis installation complete."
