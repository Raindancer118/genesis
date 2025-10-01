import subprocess
import time
from dataclasses import dataclass
from typing import Any, Dict, Optional, Tuple
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

@dataclass
class RepoStatus:
    is_dirty: bool
    behind_commits: int
    ahead_commits: int

    @property
    def has_updates(self) -> bool:
        return self.behind_commits > 0


class GitCommandError(RuntimeError):
    """Raised when a git command fails while checking for updates."""


def _run_git_command(args, *, check=True):
    """Runs a git command inside the Genesis directory."""
    return subprocess.run(
        ["git", *args],
        cwd=GENESIS_DIR,
        check=check,
        capture_output=True,
        text=True,
    )


def _parse_int(value: str) -> int:
    try:
        return int(value.strip() or "0")
    except ValueError as exc:  # pragma: no cover - defensive guard
        raise GitCommandError(str(exc)) from exc


def _collect_repo_status(*, fetch_remote: bool = True) -> RepoStatus:
    try:
        if fetch_remote:
            _run_git_command(["fetch", "--prune"])

        porcelain_result = _run_git_command(["status", "--porcelain"])
        behind_result = _run_git_command(["rev-list", "--count", "HEAD..origin/main"])
        ahead_result = _run_git_command(["rev-list", "--count", "origin/main..HEAD"])
    except subprocess.CalledProcessError as exc:  # pragma: no cover - passes through
        error_message = exc.stderr or exc.stdout or str(exc)
        raise GitCommandError(error_message) from exc

    return RepoStatus(
        is_dirty=porcelain_result.stdout.strip() != "",
        behind_commits=_parse_int(behind_result.stdout),
        ahead_commits=_parse_int(ahead_result.stdout),
    )


def check_for_updates(*, interactive: bool = True) -> Tuple[bool, Dict[str, Any]]:
    """Return whether updates are available alongside the repo status."""

    if interactive:
        console.print("🔎 Checking for updates to Genesis...")

    try:
        status = _collect_repo_status(fetch_remote=True)
    except GitCommandError as exc:
        if interactive:
            console.print(
                "[bold red]Unable to contact the remote repository:[/bold red]"
            )
            console.print(str(exc))
        return False, {"error": str(exc)}

    if not status.has_updates:
        if interactive:
            console.print("✅ Genesis is already up to date.")
            if status.is_dirty:
                console.print(
                    "[yellow]Local changes detected but no updates were required.[/yellow]"
                )
        return False, {"status": status}

    if interactive:
        console.print(
            f"⬇️  Updates available: {status.behind_commits} new commit(s) ready to apply."
        )
        console.print(
            "Run `git stash pop` manually to recover them if needed."
        )
        console.print(error_message)
    return True, {"status": status}

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


        stash_list = _run_git_command(["stash", "list"])
        for line in stash_list.stdout.splitlines():
            if label in line:
                stash_ref = line.split(":", 1)[0].strip()
                console.print("🧺 Local changes stashed temporarily for the update.")
                return stash_ref

        console.print(
            "[bold yellow]Stashed changes recorded but stash reference could not be resolved.[/bold yellow]"
        )
    except subprocess.CalledProcessError as exc:
        error_message = exc.stderr or exc.stdout or str(exc)
        console.print(
            "[bold red]Failed to stash local changes automatically.[/bold red]"
        )
        console.print(error_message)

    return None


def restore_stash(stash_ref: Optional[str]):
    """Attempts to restore the provided stash entry, if any."""

    if not stash_ref:
        return

    try:
        _run_git_command(["stash", "pop", stash_ref])
        console.print("♻️  Restored stashed changes.")
    except subprocess.CalledProcessError:
        # Attempt a non-destructive apply so the user does not lose work.
        apply_failed = False
        try:
            _run_git_command(["stash", "apply", stash_ref])
        except subprocess.CalledProcessError as exc:
            apply_failed = True
            error_message = exc.stderr or exc.stdout or str(exc)
            console.print(error_message)

        console.print(
            "[bold yellow]Stashed changes were not automatically restored.[/bold yellow]"
        )
        command_hint = "git stash pop"
        if stash_ref:
            command_hint += f" {stash_ref}"
        console.print(
            f"Run `{command_hint}` manually to recover them if needed."
        )
        if not apply_failed:
            console.print(
                "[yellow]Changes were applied without dropping the stash so nothing was lost.[/yellow]"
            )


def run_self_update():
    """Fully automated self-update with automatic stashing and restoration."""

    update_available, payload = check_for_updates(interactive=True)

    if "error" in payload:
        return

    status = payload.get("status")
    if not isinstance(status, RepoStatus):
        return

    if not update_available:
        return

    if status.ahead_commits:
        console.print(
            "[bold red]Local commits detected.[/bold red] Please push or back them up "
            "before running self-update again."
        )
        return

    stash_ref: Optional[str] = None
    if status.is_dirty:
        console.print(
            "[yellow]Stashing local changes so the update can proceed automatically.[/yellow]"
        )
        stash_ref = stash_local_changes()
        if not stash_ref:
            console.print("[bold red]Update aborted because the stash step failed.[/bold red]")
            return

    console.print("🚀 Applying update via installer script…")
    try:
        subprocess.run(["sudo", f"{GENESIS_DIR}/install.sh"], check=True)
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
        restore_stash(stash_ref)
