#!/usr/bin/env bash
set -euo pipefail

# ──────────────────────────────────────────────────────────────
#  Volantic Genesis — Installer
#  Usage: curl -fsSL https://raw.githubusercontent.com/Raindancer118/genesis/main/install.sh | bash
# ──────────────────────────────────────────────────────────────

REPO="Raindancer118/genesis"
BIN_NAME="vg"
INSTALL_DIR="/usr/local/bin"
SERVICE_DIR="${HOME}/.config/systemd/user"
GITHUB_API="https://api.github.com/repos/${REPO}/releases/latest"

# ── Colors ────────────────────────────────────────────────────
BLUE='\033[38;2;96;165;250m'
DIM='\033[38;2;71;85;105m'
GREEN='\033[38;2;74;222;128m'
RED='\033[38;2;239;68;68m'
BOLD='\033[1m'
RESET='\033[0m'

info()    { echo -e "  ${BLUE}·${RESET} $*"; }
success() { echo -e "  ${GREEN}✓${RESET} $*"; }
fail()    { echo -e "  ${RED}✗${RESET} $*" >&2; }
header()  { echo -e "\n${BOLD}${BLUE}  V O L A N T I C   G E N E S I S${RESET}\n  ${DIM}──────────────────────────────────${RESET}\n  $*\n"; }

# ── Platform detection ────────────────────────────────────────
detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    if [[ "${os}" != "Linux" ]]; then
        fail "Unsupported OS: ${os}. Only Linux is supported."
        exit 1
    fi

    case "${arch}" in
        x86_64)  echo "vg-x86_64-linux" ;;
        aarch64) echo "vg-aarch64-linux" ;;
        armv7l)  echo "vg-aarch64-linux" ;;
        *)
            fail "Unsupported architecture: ${arch}"
            exit 1
            ;;
    esac
}

# ── Fetch latest release download URL ────────────────────────
get_download_url() {
    local artifact="$1"
    local url

    if command -v curl &>/dev/null; then
        url="$(curl -fsSL "${GITHUB_API}" \
            | grep "browser_download_url" \
            | grep "${artifact}.tar.gz\"" \
            | head -1 \
            | sed 's/.*"browser_download_url": "\(.*\)".*/\1/')"
    elif command -v wget &>/dev/null; then
        url="$(wget -qO- "${GITHUB_API}" \
            | grep "browser_download_url" \
            | grep "${artifact}.tar.gz\"" \
            | head -1 \
            | sed 's/.*"browser_download_url": "\(.*\)".*/\1/')"
    else
        fail "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    if [[ -z "${url}" ]]; then
        fail "Could not find release artifact '${artifact}.tar.gz' on GitHub."
        fail "Make sure a release exists at: https://github.com/${REPO}/releases"
        exit 1
    fi

    echo "${url}"
}

# ── Download ──────────────────────────────────────────────────
download() {
    local url="$1" dest="$2"
    if command -v curl &>/dev/null; then
        curl -fsSL --progress-bar "${url}" -o "${dest}"
    else
        wget -q --show-progress "${url}" -O "${dest}"
    fi
}

# ── Install services ──────────────────────────────────────────
install_services() {
    local src_dir="$1"

    if [[ ! -d "${src_dir}" ]] || ! ls "${src_dir}"/vg-*.service &>/dev/null 2>&1; then
        return 0
    fi

    echo ""
    read -r -p "  Install systemd user services (vg-greet, vg-sentry)? [y/N] " choice
    if [[ "${choice}" =~ ^[Yy]$ ]]; then
        mkdir -p "${SERVICE_DIR}"
        cp "${src_dir}"/vg-*.service "${src_dir}"/vg-*.timer "${SERVICE_DIR}/" 2>/dev/null || true
        systemctl --user daemon-reload 2>/dev/null || true
        systemctl --user enable --now vg-greet.service 2>/dev/null || true
        systemctl --user enable --now vg-sentry.timer 2>/dev/null || true
        success "Services installed and enabled"
    else
        info "Skipping service installation"
    fi
}

# ── Main ──────────────────────────────────────────────────────
main() {
    header "INSTALLER"

    local artifact
    artifact="$(detect_target)"
    info "Detected target: ${artifact}"

    info "Fetching latest release info..."
    local url
    url="$(get_download_url "${artifact}")"
    local version
    version="$(echo "${url}" | grep -oP 'v[0-9]+\.[0-9]+\.[0-9]+' | head -1)"
    info "Latest version: ${version:-unknown}"

    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "${TMP_DIR}"' EXIT

    info "Downloading ${artifact}.tar.gz..."
    download "${url}" "${TMP_DIR}/${artifact}.tar.gz"

    info "Extracting..."
    tar -xzf "${TMP_DIR}/${artifact}.tar.gz" -C "${TMP_DIR}"

    info "Installing to ${INSTALL_DIR}/${BIN_NAME}..."
    if [[ -w "${INSTALL_DIR}" ]]; then
        cp "${TMP_DIR}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
        chmod +x "${INSTALL_DIR}/${BIN_NAME}"
    else
        # Need sudo — ask once
        echo -e "  ${DIM}(sudo required to write to ${INSTALL_DIR})${RESET}"
        sudo cp "${TMP_DIR}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"
        sudo chmod +x "${INSTALL_DIR}/${BIN_NAME}"
    fi

    success "${BIN_NAME} installed at ${INSTALL_DIR}/${BIN_NAME}"

    # Try to install services from a cloned repo (optional)
    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]:-install.sh}")" 2>/dev/null && pwd || echo "")"
    if [[ -n "${script_dir}" && -f "${script_dir}/vg-greet.service" ]]; then
        install_services "${script_dir}"
    fi

    echo ""
    success "Installation complete!"
    echo ""
    echo -e "  ${DIM}Run ${RESET}${BOLD}vg --help${RESET}${DIM} to get started.${RESET}"
    echo ""
}

main "$@"
