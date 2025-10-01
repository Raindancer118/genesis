import subprocess
import time

from rich.console import Console

import questionary

console = Console()
GENESIS_DIR = "/opt/genesis"


def stash_local_changes():
    """Stashes local changes so the updater can run on a clean tree."""
    label = f"genesis-self-update-{int(time.time())}"
    try:
        subprocess.run(
            [
                "git",
                "stash",
                "push",
                "--include-untracked",
                "--message",
                label,
            ],
            cwd=GENESIS_DIR,
            check=True,
            capture_output=True,
            text=True,
        )
        console.print("🧺 Local changes stashed temporarily for the update.")
        return True
    except subprocess.CalledProcessError as exc:
        error_message = exc.stderr or exc.stdout or str(exc)
        console.print(
            "[bold red]Failed to stash local changes automatically.[/bold red]"
        )
        console.print(error_message)
        return False


def restore_stash():
    """Attempts to restore the most recent stash entry."""
    try:
        subprocess.run(
            ["git", "stash", "pop"],
            cwd=GENESIS_DIR,
            check=True,
            capture_output=True,
            text=True,
        )
        console.print("♻️  Restored stashed changes.")
    except subprocess.CalledProcessError as exc:
        error_message = exc.stderr or exc.stdout or str(exc)
        console.print(
            "[bold yellow]Stashed changes were not automatically restored.[/bold yellow]"
        )
        console.print(
            "Run `git stash pop` manually to recover them if needed."
        )
        console.print(error_message)


def check_for_updates(interactive=True):
    """Checks if updates are available and ensures the worktree is clean."""
    console.print("🔎 Checking for updates to Genesis...")
    stashed_changes = False
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
                    "[yellow]Updates are available. Genesis can stash your changes "
                    "automatically and restore them afterwards.[/yellow]"
                )
                if interactive:
                    if questionary.confirm(
                        "Stash local changes and continue with the update?"
                    ).ask():
                        stashed_changes = stash_local_changes()
                        if not stashed_changes:
                            return False, False
                        is_dirty = False
                    else:
                        console.print(
                            "Update cancelled. Your changes remain untouched."
                        )
                        return False, False
                else:
                    console.print(
                        "[yellow]Run 'git stash' or 'git commit' before self-updating to avoid "
                        "losing changes.[/yellow]"
                    )
            else:
                console.print(
                    "[yellow]Please commit or stash your work before running self-update.[/yellow]"
                )
                return False, False

        if behind_commits:
            console.print(
                f"⬇️  Updates available: {behind_commits} new commit(s) ready to apply."
            )
            return True, stashed_changes

        console.print("✅ Genesis is already up to date.")
        return False, False
    except (subprocess.CalledProcessError, ValueError) as exc:
        error_message = (
            exc.stderr
            if isinstance(exc, subprocess.CalledProcessError) and exc.stderr
            else str(exc)
        )
        console.print(f"[bold red]Error checking for updates:[/bold red]\n{error_message}")
        return False, False


def perform_update(stashed_changes=False):
    """Performs the self-update by re-running the installer script."""
    console.print("🚀 An update is available for Genesis.")
    if not questionary.confirm("Do you want to install it now?").ask():
        console.print("Update cancelled.")
        if stashed_changes:
            restore_stash()
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
    finally:
        if stashed_changes:
            restore_stash()
