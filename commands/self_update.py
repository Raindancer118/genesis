import subprocess
from rich.console import Console
import questionary

console = Console()
GENESIS_DIR = "/opt/genesis"


def check_for_updates():
    """Checks if the local repo is behind the remote. Returns True if updates are available."""
    console.print("🔎 Checking for updates to Genesis...")
    try:
        # --- KORREKTUR: 'sudo' wurde von den git-Befehlen entfernt ---
        subprocess.run(
            ['git', 'fetch'],
            cwd=GENESIS_DIR, check=True, capture_output=True
        )

        status_result = subprocess.run(
            ['git', 'status', '-uno'],
            cwd=GENESIS_DIR, check=True, capture_output=True, text=True
        )

        return "Your branch is behind" in status_result.stdout
    except subprocess.CalledProcessError as e:
        console.print(f"[bold red]Error checking for updates:[/bold red]\n{e.stderr.decode()}")
        return False


def perform_update():
    """Performs the self-update by re-running the installer script."""
    console.print(f"🚀 An update is available for Genesis.")
    if questionary.confirm("Do you want to install it now?").ask():
        console.print("Updating... The installer will apply the changes.")
        # Der Installer selbst wird weiterhin mit sudo ausgeführt
        subprocess.run(['sudo', f'{GENESIS_DIR}/install.sh'])
    else:
        console.print("Update cancelled.")