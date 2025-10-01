import subprocess
from rich.console import Console
import questionary

console = Console()
GENESIS_DIR = "/opt/genesis"


def check_for_updates():
    """Checks if the local repo is behind the remote. Returns True if updates are available."""
    try:
        # Fetch the latest changes from the remote without merging
        subprocess.run(['git', 'fetch'], cwd=GENESIS_DIR, check=True, capture_output=True)

        # Check the status against the remote branch
        status_result = subprocess.run(
            ['git', 'status', '-uno'],
            cwd=GENESIS_DIR, check=True, capture_output=True, text=True
        )

        return "Your branch is behind" in status_result.stdout
    except subprocess.CalledProcessError:
        return False  # Git command failed, assume no updates


def perform_update():
    """Performs the self-update by re-running the installer script."""
    console.print(f"🚀 An update is available for Genesis.")
    if questionary.confirm("Do you want to install it now?").ask():
        console.print("Updating... Genesis will be restarted by the installer.")
        # The installer handles pulling the latest code and reinstalling
        subprocess.run(['sudo', f'{GENESIS_DIR}/install.sh'])
    else:
        console.print("Update cancelled.")