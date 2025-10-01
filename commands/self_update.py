import subprocess
from rich.console import Console
import questionary

console = Console()
GENESIS_DIR = "/opt/genesis"


def _run_git(args):
    """Helper: run git command in GENESIS_DIR and return CompletedProcess."""
    return subprocess.run(
        ["git", *args],
        cwd=GENESIS_DIR,
        check=True,
        capture_output=True,
        text=True,
    )


def check_for_updates():
    """Prüft, ob Updates verfügbar sind und ob die Working-Tree sauber ist.
    Rückgabe:
      True  -> Updates verfügbar und Working-Tree sauber
      False -> keine Updates ODER lokale Änderungen blockieren Update ODER Fehler
    """
    console.print("🔎 Checking for updates to Genesis...")
    try:
        # Remote-Infos aktualisieren (inkl. Entfernen verwaister Branch-Refs)
        _run_git(["fetch", "--prune"])

        # Prüfen, ob lokale Änderungen vorliegen
        porcelain_result = _run_git(["status", "--porcelain"])
        is_dirty = porcelain_result.stdout.strip() != ""

        # Wieviele Commits ist HEAD hinter origin/main?
        behind_result = _run_git(["rev-list", "--count", "HEAD..origin/main"])
        behind_commits_str = behind_result.stdout.strip()
        behind_commits = int(behind_commits_str) if behind_commits_str else 0

        if is_dirty:
            console.print("[bold yellow]Local changes detected in /opt/genesis.[/bold yellow]")
            if behind_commits:
                console.print(
                    "[yellow]Updates are available but cannot be applied until you commit "
                    "or stash your changes.[/yellow]"
                )
            else:
                console.print(
                    "[yellow]Please commit or stash your work before running self-update.[/yellow]"
                )
            return False

        if behind_commits:
            console.print(f"⬇️  Updates available: {behind_commits} new commit(s) ready to apply.")
            return True

        console.print("✅ Genesis is already up to date.")
        return False

    except (subprocess.CalledProcessError, ValueError) as exc:
        if isinstance(exc, subprocess.CalledProcessError):
            stderr = exc.stderr or ""
            error_message = stderr if stderr else str(exc)
        else:
            error_message = str(exc)
        console.print(f"[bold red]Error checking for updates:[/bold red]\n{error_message}")
        return False


def perform_update():
    """Führt das Self-Update aus, indem der Installer erneut aufgerufen wird."""
    console.print("🚀 An update is available for Genesis.")
    if not questionary.confirm("Do you want to install it now?").ask():
        console.print("Update cancelled.")
        return

    console.print("Updating... The installer will apply the changes.")
    try:
        subprocess.run(['sudo', f'{GENESIS_DIR}/install.sh'], check=True)
        console.print("✅ Update completed successfully.")
    except FileNotFoundError:
        console.print(
            f"[bold red]Installer not found at {GENESIS_DIR}/install.sh. "
            "Please verify your installation.[/bold red]"
        )
    except subprocess.CalledProcessError as exc:
        console.print(
            f"[bold red]Update failed with exit code {exc.returncode}. "
            "Review the installer output above for details.[/bold red]"
        )
