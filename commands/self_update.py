import subprocess
import time
from rich.console import Console

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

def _run_git_command(args, *, check=True):
    """Runs a git command inside the Genesis directory."""
    return subprocess.run(
        ["git", *args],
        cwd=GENESIS_DIR,
        check=check,
        capture_output=True,
        text=True,
    )


def stash_local_changes():
    """Stashes local changes so the updater can run on a clean tree."""
    label = f"genesis-self-update-{int(time.time())}"
    try:
        _run_git_command(
            [
                "stash",
                "push",
                "--include-untracked",
                "--message",
                label,
            ]
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
        _run_git_command(["stash", "pop"])
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


def run_self_update():
    """Fully automated self-update with automatic stashing and restoration."""
    console.print("🔎 Checking for updates to Genesis...")

    try:
        _run_git_command(["fetch", "--prune"])
    except subprocess.CalledProcessError as exc:
        error_message = exc.stderr or exc.stdout or str(exc)
        console.print(
            f"[bold red]Unable to contact the remote repository:[/bold red]\n{error_message}"
        )
        return
    try:
        porcelain_result = _run_git_command(["status", "--porcelain"])
        is_dirty = porcelain_result.stdout.strip() != ""

        behind_result = _run_git_command(["rev-list", "--count", "HEAD..origin/main"])
        behind_commits = int(behind_result.stdout.strip() or "0")
        ahead_result = _run_git_command(["rev-list", "--count", "origin/main..HEAD"])
        ahead_commits = int(ahead_result.stdout.strip() or "0")
    except (subprocess.CalledProcessError, ValueError) as exc:
        error_message = (
            exc.stderr
            if isinstance(exc, subprocess.CalledProcessError) and exc.stderr
            else str(exc)
        )
        console.print(
            f"[bold red]Error determining repository status:[/bold red]\n{error_message}"
        )
        return

    if behind_commits == 0:
        console.print("✅ Genesis is already up to date.")
        if is_dirty:
            console.print(
                "[yellow]Local changes detected but no updates were required.[/yellow]"
            )
        return

    console.print(
        f"⬇️  Updates available: {behind_commits} new commit(s) ready to apply."
    )

    if ahead_commits:
        console.print(
            "[bold red]Local commits detected.[/bold red] Please push or back them up "
            "before running self-update again."
        )
        return

    stashed_changes = False
    if is_dirty:
        console.print(
            "[yellow]Stashing local changes so the update can proceed automatically.[/yellow]"
        )
        stashed_changes = stash_local_changes()
        if not stashed_changes:
            console.print("[bold red]Update aborted because the stash step failed.[/bold red]")
            return

    console.print("🚀 Applying update via installer script…")
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
