import shutil
import subprocess
import os
import platform
import sys
from typing import List, Tuple, Optional
from rich.console import Console
from rich.panel import Panel
from rich.text import Text
from rich.table import Table
from rich import box
from rich.layout import Layout
from rich.align import Align

console = Console()

def _run_command(cmd: List[str], capture: bool = True) -> subprocess.CompletedProcess:
    """Helper to run a system command safely."""
    try:
        # On Windows, we might need shell=True for some commands if they are built-ins?
        # But generally subprocess.run works.
        return subprocess.run(cmd, capture_output=capture, text=True)
    except Exception as e:
        failed = subprocess.CompletedProcess(args=cmd, returncode=1, stdout="", stderr=str(e))
        return failed

def _run_powershell(script: str) -> subprocess.CompletedProcess:
    """Helper to run a PowerShell script block."""
    try:
        cmd = ["powershell", "-NoProfile", "-NonInteractive", "-Command", script]
        return subprocess.run(cmd, capture_output=True, text=True)
    except Exception as e:
        return subprocess.CompletedProcess(args=[], returncode=1, stdout="", stderr=str(e))

def _print_header() -> None:
    """Prints a beautiful system information header."""
    
    # Gather Info
    node = platform.node()
    system = platform.system()
    release = platform.release()
    version = platform.version()
    machine = platform.machine()
    processor = platform.processor()

    # Try to get better Distro info on Linux
    distro_name = "Unknown"
    if system == "Linux":
        try:
            with open("/etc/os-release") as f:
                for line in f:
                    if line.startswith("PRETTY_NAME="):
                        distro_name = line.split("=", 1)[1].strip().strip('"')
                        break
        except Exception:
            distro_name = "Linux Generic"
    elif system == "Windows":
        distro_name = f"Windows {release}"

    # Create a Grid/Table for info
    table = Table(box=box.SIMPLE_HEAD, show_header=False, expand=True)
    table.add_column("Key", style="cyan bold", justify="right")
    table.add_column("Value", style="white")

    table.add_row("OS / Distro", distro_name)
    table.add_row("Kernel Version", release)
    table.add_row("Architecture", machine)
    table.add_row("Hostname", node)
    if processor:
        table.add_row("Processor", processor)
    
    # Header Panel
    console.print(Panel(
        table, 
        title="[bold white]GENESIS SYSTEM HEALTH AUDIT[/bold white]", 
        subtitle=f"[dim]Checked at {subprocess.run(['date'], capture_output=True, text=True).stdout.strip() if system != 'Windows' else ''}[/dim]",
        style="blue",
        box=box.DOUBLE
    ))
    console.print()


# ==========================================
# LINUX CHECKS
# ==========================================

def _check_linux_kernel() -> None:
    """Checks kernel ring buffer for critical errors (Linux)."""
    console.print(Panel("<b>[KERNEL] Hardware & System Logs</b>", style="cyan", box=box.MINIMAL))
    cmd = ["dmesg", "--level=crit,alert,emerg"]
    if os.geteuid() != 0:
         cmd.insert(0, "sudo")

    res = _run_command(cmd)
    
    if res.returncode != 0 and "permission" in (res.stderr or "").lower():
         console.print("[red]✘ Permission denied. Run with sudo to see kernel errors.[/red]")
         return

    errors = res.stdout.strip()
    if not errors:
        console.print("[green]✔ No critical kernel errors found.[/green]")
    else:
        console.print("[red]WARNUNG: Critical Kernel Messages:[/red]")
        console.print(errors)

def _check_linux_services() -> None:
    """Checks for failed systemd services (Linux)."""
    console.print()
    console.print(Panel("<b>[SERVICES] Systemd Status</b>", style="cyan", box=box.MINIMAL))
    
    if not shutil.which("systemctl"):
        console.print("[yellow]Skipping: systemctl not found.[/yellow]")
        return
        
    res = _run_command(["systemctl", "list-units", "--state=failed", "--no-legend"])
    failed_services = res.stdout.strip()
    
    if not failed_services:
        console.print("[green]✔ All started services are running cleanly.[/green]")
    else:
        console.print("[red]WARNUNG: The following services have failed:[/red]")
        # Show cleaner output
        _run_command(["systemctl", "list-units", "--state=failed"], capture=False)

def _check_linux_packages() -> None:
    """Checks package integrity (Pacman, Dpkg, Rpm)."""
    console.print()
    console.print(Panel("<b>[PACKAGES] Database & File Integrity</b>", style="cyan", box=box.MINIMAL))

    # Arch / Manjaro
    if shutil.which("pacman"):
        console.print("[bold]Pacman (Arch/Manjaro)[/bold]")
        # DB Check
        res = _run_command(["pacman", "-Dk"])
        if res.returncode == 0:
            console.print("  [green]✔ Database consistent.[/green]")
        else:
             console.print("  [red]✘ Database ERROR (pacman -Dk).[/red]")
        
        # Files Check
        console.print("  [dim]Scanning file integrity (pacman -Qk)...[/dim]")
        res = _run_command(["pacman", "-Qk"])
        lines = res.stdout.splitlines()
        problems = [line for line in lines if " 0 missing files" not in line and line.strip()]
        if not problems:
             console.print("  [green]✔ Filesystem 100% intact.[/green]")
        else:
             console.print(f"  [red]✘ Missing files in {len(problems)} packages.[/red]")

    # Debian / Ubuntu
    if shutil.which("dpkg"):
        console.print("[bold]Dpkg (Debian/Ubuntu)[/bold]")
        console.print("  [dim]Verifying installed packages (dpkg -V)...[/dim]")
        # dpkg -V prints nothing if good, or lines like "??5?????? c /etc/..." if changed
        try:
             # This can be spammy if configuration files are changed (common)
             # We might want to filter?
             res = _run_command(["dpkg", "-V"])
             if not res.stdout.strip():
                 console.print("  [green]✔ Integrity check passed.[/green]")
             else:
                 count = len(res.stdout.splitlines())
                 console.print(f"  [yellow]! {count} files have checksum/permission mismatches (often config changes).[/yellow]")
        except Exception:
             pass

    # Fedora / RHEL
    if shutil.which("rpm"):
        console.print("[bold]RPM (Fedora/RHEL)[/bold]")
        console.print("  [dim]Verifying installed packages (rpm -Va)...[/dim]")
        # rpm -Va
        res = _run_command(["rpm", "-Va"])
        if not res.stdout.strip():
             console.print("  [green]✔ Integrity check passed.[/green]")
        else:
             count = len(res.stdout.splitlines())
             console.print(f"  [yellow]! {count} files differ from database.[/yellow]")

def _check_linux_disk() -> None:
    """Checks disk space (Linux)."""
    console.print()
    console.print(Panel("<b>[DISK] Storage Usage</b>", style="cyan", box=box.MINIMAL))
    # Simple df -h /
    res = _run_command(["df", "-h", "/"])
    if res.returncode == 0:
         lines = res.stdout.splitlines()
         if len(lines) > 1:
             console.print(lines[0]) # Header
             console.print(lines[1]) # Root
    else:
         console.print("[red]Failed to check disk.[/red]")

# ==========================================
# WINDOWS CHECKS
# ==========================================

def _check_windows_events() -> None:
    console.print(Panel("<b>[LOGS] Windows System Events</b>", style="cyan", box=box.MINIMAL))
    # Get last 5 Critical or Error events from System log
    script = "Get-EventLog -LogName System -EntryType Error,Warning -Newest 5 | Format-Table -AutoSize"
    res = _run_powershell(script)
    if res.returncode == 0 and res.stdout.strip():
        console.print(res.stdout)
    else:
        console.print("[green]✔ No recent critical errors found in System log.[/green]")

def _check_windows_services() -> None:
    console.print()
    console.print(Panel("<b>[SERVICES] Failed Services</b>", style="cyan", box=box.MINIMAL))
    # Check for automatic services that are stopped
    script = "Get-Service | Where-Object {$_.Status -eq 'Stopped' -and $_.StartType -eq 'Automatic'} | Select-Object Name,DisplayName"
    res = _run_powershell(script)
    if res.returncode == 0:
         if res.stdout.strip():
             console.print("[red]The following Automatic services are STOPPED:[/red]")
             console.print(res.stdout)
         else:
             console.print("[green]✔ All Automatic services are running.[/green]")

def _check_windows_integrity() -> None:
    console.print()
    console.print(Panel("<b>[INTEGRITY] System File Checker</b>", style="cyan", box=box.MINIMAL))
    console.print("[dim]Note: 'sfc /verifyonly' requires Admin and takes time.[/dim]")
    # Warn user
    # If not admin, this fails
    try:
        # check admin
        is_admin = False
        try:
             import ctypes
             is_admin = ctypes.windll.shell32.IsUserAnAdmin() != 0
        except: pass
        
        if not is_admin:
             console.print("[yellow]⚠ Not running as Administrator. Skipping SFC check.[/yellow]")
             return

        console.print("[dim]Starting SFC scan...[/dim]")
        # We might want to stream this or just run it? It's slow.
        # Maybe skip for 'quick' check?
        # User asked for health check, so we do it.
        # But we do verifyonly to avoid changes.
        res = subprocess.run(["sfc", "/verifyonly"], capture_output=True, text=True)
        if res.returncode == 0:
             console.print("[green]✔ No integrity violations found.[/green]")
        else:
             console.print("[red]⚠ Integrity issues found or scan failed![/red]")
             console.print(res.stdout)
    except Exception as e:
        console.print(f"[red]Error running SFC: {e}[/red]")

def _check_windows_disk() -> None:
    console.print()
    console.print(Panel("<b>[DISK] Drive Status</b>", style="cyan", box=box.MINIMAL))
    script = "Get-Volume | Where-Object {$_.DriveType -eq 'Fixed'} | Format-Table DriveLetter,FileSystem,SizeRemaining,Size -AutoSize"
    res = _run_powershell(script)
    console.print(res.stdout)


# ==========================================
# MAIN
# ==========================================

def run_health_check() -> None:
    """Runs the platform-specific health audit."""
    _print_header()
    
    system = platform.system()
    
    if system == "Linux":
        if os.geteuid() != 0:
            console.print("[bold yellow]Note: For best results, run as root (sudo).[/bold yellow]\n")
        _check_linux_kernel()
        _check_linux_services()
        _check_linux_packages()
        _check_linux_disk()
        
        # User/group check from before (re-implementing simplified)
        console.print()
        console.print(Panel("<b>[USERS] Consistency Check</b>", style="cyan", box=box.MINIMAL))
        if shutil.which("pwck"):
            # We suppress output unless error
            res = subprocess.run(["sudo", "pwck", "-r"] if os.geteuid()!=0 else ["pwck", "-r"], capture_output=True)
            if res.returncode == 0:
                 console.print("[green]✔ User DB consistent.[/green]")
            else:
                 console.print("[yellow]! /etc/passwd issues found.[/yellow]")
        else:
            console.print("[dim]pwck not found.[/dim]")

    elif system == "Windows":
        _check_windows_events()
        _check_windows_services()
        _check_windows_disk()
        _check_windows_integrity()
    
    else:
        console.print(f"[red]Unsupported Operating System: {system}[/red]")
    
    console.print("\n[bold blue]Audit complete.[/bold blue]")
