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
    """Checks if updates are available and ensures the worktree is clean."""
    console.print("🔎 Checking for updates to Genesis...")
    try:
        subprocess.run(
            ["git", "fetch", "--prune"],
            cwd=GENESIS_DIR,
            check=True,
            capture_output=True,
            text=True,
        )

        porcelain_result = subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=GENESIS_DIR,
            check=True,
            capture_output=True,
            text=True,
        )
        is_dirty = porcelain_result.stdout.strip() != ""

        behind_result = subprocess.run(
            ["git", "rev-list", "--count", "HEAD..origin/main"],
            cwd=GENESIS_DIR,
            check=True,
            capture_output=True,
            text=True,
        )
        behind_commits = int(behind_result.stdout.strip() or "0")

        if is_dirty:
            console.print(
                "[bold yellow]Local changes detected in /opt/genesis.[/bold yellow]"
            )

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
            console.print(
                f"⬇️  Updates available: {behind_commits} new commit(s) ready to apply."
            )
            return True

        console.print("✅ Genesis is already up to date.")
        return False
    except (subprocess.CalledProcessError, ValueError) as exc:
        error_message = (
            exc.stderr
            if isinstance(exc, subprocess.CalledProcessError) and exc.stderr
            else str(exc)
        )

        console.print(f"[bold red]Error checking for updates:[/bold red]\n{error_message}")
        return False


def perform_update():
    """Performs the self-update by re-running the installer script."""
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
            f"[bold red]Installer not found at {GENESIS_DIR}/install.sh."
            " Please verify your installation.[/bold red]"
        )
    except subprocess.CalledProcessError as exc:
        console.print(
            f"[bold red]Update failed with exit code {exc.returncode}."
            " Review the installer output above for details.[/bold red]"

        )
