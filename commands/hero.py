"""Hero command: Kill resource-intensive processes to free up system resources."""

from __future__ import annotations
import os
import time
import getpass
from dataclasses import dataclass
from typing import List, Set, Dict
import psutil
from rich.console import Console
from rich.table import Table
from rich.panel import Panel
import questionary

console = Console()

# System-critical processes that should never be killed
SAFE_PROC_NAMES: Set[str] = {
    "plasmashell", "kwin_x11", "kwin_wayland", "Xorg", "Xwayland", "gnome-shell",
    "sddm", "gdm", "systemd", "systemd-logind",
    "pipewire", "wireplumber", "pulseaudio",
    "NetworkManager", "wpa_supplicant", "dhcpcd", "chronyd", "systemd-resolved",
    "dbus-daemon", "polkitd", "udisksd", "upowerd", "bluetoothd", "cupsd", "avahi-daemon",
    "zsh", "bash", "fish", "sh", "python", "python3", "perl", "ruby",
    "kded5", "ksmserver", "kdeinit5", "kdeinit6", "kglobalaccel5", "kglobalaccel",
    "genesis",
    # Windows Critical
    "explorer.exe", "taskmgr.exe", "csrss.exe", "lsass.exe", 
    "winlogon.exe", "dwm.exe", "svchost.exe", "services.exe",
    "wininit.exe", "smss.exe", "spoolsv.exe", "Registry", "System",
}

# PIDs that should never be killed (init and self)
ALWAYS_SAFE_PIDS: Set[int] = {1, os.getpid()}


@dataclass
class Target:
    """Represents a process target for termination."""
    pid: int
    name: str
    username: str
    rss_mb: float
    cpu: float
    cmdline: str

    def score(self) -> float:
        """Calculate priority score for this target (higher = more critical to kill)."""
        # CPU usage has more weight as it causes immediate lag
        return self.cpu * 2.0 + self.rss_mb * 0.5


def _get_memory_mb(proc: psutil.Process) -> float:
    """Get RSS memory usage in MB for a process."""
    try:
        return proc.memory_info().rss / (1024 * 1024)
    except (psutil.NoSuchProcess, psutil.AccessDenied):
        return 0.0


def _get_cmdline(proc: psutil.Process) -> str:
    """Get command line for a process, truncated to 300 chars."""
    try:
        cmdline = " ".join(proc.cmdline())
        return cmdline[:300] if cmdline else proc.name()
    except (psutil.NoSuchProcess, psutil.AccessDenied):
        return ""


def _is_safe_process(proc: psutil.Process) -> bool:
    """Check if a process should be protected from termination."""
    try:
        # Always protect init and self
        if proc.pid in ALWAYS_SAFE_PIDS:
            return True
        
        # Check process name against safe list
        name = (proc.name() or "").strip()
        if name in SAFE_PROC_NAMES:
            return True
        
        # Protect kernel threads (names in brackets)
        if name.startswith("[") and name.endswith("]"):
            return True
            
    except (psutil.NoSuchProcess, psutil.AccessDenied):
        # If we can't determine safety, err on the side of caution
        return True
    
    return False


def _sample_cpu_usage(procs: List[psutil.Process], sleep_duration: float = 0.4) -> Dict[int, float]:
    """
    Sample CPU usage for multiple processes efficiently.
    
    Makes two measurements with a single sleep in between to calculate CPU percentage.
    """
    cpu_values: Dict[int, float] = {}
    alive_procs: List[psutil.Process] = []
    
    # First measurement (priming)
    for proc in procs:
        try:
            proc.cpu_percent(None)
            alive_procs.append(proc)
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            pass
    
    # Wait for interval
    time.sleep(sleep_duration)
    
    # Second measurement (actual values)
    for proc in alive_procs:
        try:
            cpu_values[proc.pid] = proc.cpu_percent(None)
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            cpu_values[proc.pid] = 0.0
    
    return cpu_values


def find_targets(
    scope: str = "user",
    mem_threshold_mb: float = 400.0,
    cpu_threshold: float = 50.0,
    limit: int = 15,
    fast: bool = False,
) -> List[Target]:
    """
    Find processes that exceed resource thresholds.
    
    Args:
        scope: 'user' to only check current user's processes, 'all' for all processes
        mem_threshold_mb: Minimum memory usage in MB to consider
        cpu_threshold: Minimum CPU percentage to consider
        limit: Maximum number of targets to return
        fast: Skip CPU sampling for faster execution
    
    Returns:
        List of Target objects, sorted by priority score
    """
    current_user = getpass.getuser()
    candidate_procs: List[psutil.Process] = []
    
    # Collect candidate processes
    for proc in psutil.process_iter(["pid", "name", "username"]):
        try:
            # Skip safe processes
            if _is_safe_process(proc):
                continue
            
            # Filter by scope
            if scope == "user" and proc.username() != current_user:
                continue
            
            candidate_procs.append(proc)
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
    
    # Sample CPU usage if not in fast mode
    cpu_map: Dict[int, float] = {}
    if not fast:
        cpu_map = _sample_cpu_usage(candidate_procs)
    
    # Build target list
    targets: List[Target] = []
    for proc in candidate_procs:
        try:
            cpu = 0.0 if fast else cpu_map.get(proc.pid, 0.0)
            rss = _get_memory_mb(proc)
            
            # Check if process exceeds thresholds
            if rss >= mem_threshold_mb or cpu >= cpu_threshold:
                targets.append(
                    Target(
                        pid=proc.pid,
                        name=proc.name() or "?",
                        username=proc.username() or "?",
                        rss_mb=rss,
                        cpu=cpu,
                        cmdline=_get_cmdline(proc),
                    )
                )
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            continue
    
    # Sort by priority score and limit results
    targets.sort(key=lambda t: t.score(), reverse=True)
    return targets[:limit]


def _terminate_process_tree(pid: int, graceful_timeout: float = 2.0) -> tuple[bool, str]:
    """
    Terminate a process and its children gracefully, with fallback to force kill.
    
    Args:
        pid: Process ID to terminate
        graceful_timeout: Seconds to wait for graceful termination
    
    Returns:
        Tuple of (success, status_message)
    """
    try:
        proc = psutil.Process(pid)
    except psutil.NoSuchProcess:
        return True, "already terminated"
    
    # Get all child processes
    children = []
    try:
        children = proc.children(recursive=True)
    except (psutil.NoSuchProcess, psutil.AccessDenied):
        pass
    
    # Send SIGTERM to all processes (children first)
    for child in children:
        try:
            if child.pid not in ALWAYS_SAFE_PIDS:
                child.terminate()
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            pass
    
    try:
        proc.terminate()
    except (psutil.NoSuchProcess, psutil.AccessDenied):
        pass
    
    # Wait for graceful termination
    all_procs = [proc] + children
    gone, alive = psutil.wait_procs(all_procs, timeout=graceful_timeout)
    
    # Force kill any remaining processes
    for p in alive:
        try:
            if p.pid not in ALWAYS_SAFE_PIDS:
                p.kill()
        except (psutil.NoSuchProcess, psutil.AccessDenied):
            pass
    
    # Final wait
    if alive:
        psutil.wait_procs(alive, timeout=1.0)
    
    return True, "terminated"


def _display_targets(targets: List[Target], fast: bool):
    """Display targets in a formatted table."""
    table = Table(title="Resource-Intensive Processes", show_header=True, header_style="bold cyan")
    table.add_column("PID", style="yellow", justify="right")
    table.add_column("Name", style="magenta")
    table.add_column("User", style="blue")
    table.add_column("Memory", justify="right")
    table.add_column("CPU", justify="right")
    table.add_column("Command", style="dim", max_width=50)
    
    for t in targets:
        cpu_display = "N/A" if fast else f"{t.cpu:.1f}%"
        table.add_row(
            str(t.pid),
            t.name[:20],
            t.username[:12],
            f"{t.rss_mb:.0f} MB",
            cpu_display,
            t.cmdline[:50],
        )
    
    console.print(table)


def run(
    dry_run: bool = False,
    scope: str = "user",
    mem_threshold_mb: float = 400.0,
    cpu_threshold: float = 50.0,
    limit: int = 15,
    verbose: bool = True,
    fast: bool = False,
) -> int:
    """
    Main entry point for the hero command.
    
    Args:
        dry_run: If True, only show targets without killing them
        scope: 'user' or 'all'
        mem_threshold_mb: Memory threshold in MB
        cpu_threshold: CPU threshold percentage
        limit: Maximum number of processes to target
        verbose: Show detailed output
        fast: Skip CPU sampling
    
    Returns:
        Exit code (0 for success)
    """
    if verbose:
        console.print(Panel(
            f"[bold]Scope:[/bold] {scope} | [bold]Dry Run:[/bold] {dry_run} | [bold]Fast:[/bold] {fast}\n"
            f"[bold]Thresholds:[/bold] Memory ≥ {mem_threshold_mb} MB, CPU ≥ {cpu_threshold}% | "
            f"[bold]Limit:[/bold] {limit}",
            title="[bold cyan]Genesis Hero[/bold cyan]",
            border_style="cyan"
        ))
    
    # Find target processes
    targets = find_targets(
        scope=scope,
        mem_threshold_mb=mem_threshold_mb,
        cpu_threshold=cpu_threshold,
        limit=limit,
        fast=fast,
    )
    
    if not targets:
        if verbose:
            console.print("[green]✓[/green] No processes exceed the specified thresholds.")
        return 0
    
    # Display targets
    if verbose:
        _display_targets(targets, fast)
    
    # In dry-run mode, just show what would be done
    if dry_run:
        if verbose:
            console.print(f"\n[yellow]Dry run mode:[/yellow] Would terminate {len(targets)} process(es).")
        return 0
    
    # Ask for confirmation before killing processes
    if verbose:
        console.print()
        if not questionary.confirm(
            f"Terminate {len(targets)} process(es)?",
            default=False
        ).ask():
            console.print("[yellow]Operation cancelled.[/yellow]")
            return 0
    
    # Terminate processes
    terminated_count = 0
    for t in targets:
        # Double-check safety (paranoid but safe)
        if t.name in SAFE_PROC_NAMES or t.pid in ALWAYS_SAFE_PIDS:
            if verbose:
                console.print(f"[yellow]⊘[/yellow] Skipping PID {t.pid} ({t.name}): protected process")
            continue
        
        success, msg = _terminate_process_tree(t.pid)
        if success:
            terminated_count += 1
            if verbose:
                console.print(f"[green]✓[/green] PID {t.pid} ({t.name}): {msg}")
        else:
            if verbose:
                console.print(f"[red]✗[/red] PID {t.pid} ({t.name}): {msg}")
    
    if verbose:
        console.print(f"\n[green]✓[/green] Terminated {terminated_count} of {len(targets)} processes.")
    
    return 0
