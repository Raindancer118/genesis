import shutil
import subprocess
import os
from typing import List, Tuple, Optional
from rich.console import Console
from rich.panel import Panel
from rich.text import Text
from rich.table import Table
from rich import box

console = Console()

def _run_command(cmd: List[str], capture: bool = True) -> subprocess.CompletedProcess:
    """Helper to run a system command safely."""
    try:
        # Use sudo if strictly processing system-level audits that require root?
        # Many of these checks require root (dmesg - unprivileged might valid, pwck/grpck/pacman -Dk need root or write access/read access to secure files).
        # We'll try running as is, and let the user handle permissions or checking if sudo is needed.
        # But specifically `dmesg` might be restricted. `pacman -Dk` usually doesn't need root to *read*?
        # Actually `dmesg` on modern systemd often restricts access to root/adm.
        # The user's script checked for UID==0. We should probably warn or try sudo if it fails.
        
        # However, calling 'sudo' inside python scripts that might be run by user might be annoying if interactive password prompt messes up UI.
        # For now we will run them directly. If they fail due to permissions, we report it.
        # But wait, the bash script specifically checks `if [ "$EUID" -ne 0 ]; then exit 1`.
        # So it implies this command MUST be run as root.
        
        return subprocess.run(cmd, capture_output=capture, text=True)
    except Exception as e:
        # Return a dummy failed process object
        failed = subprocess.CompletedProcess(args=cmd, returncode=1, stdout="", stderr=str(e))
        return failed

def check_kernel() -> None:
    """Checks kernel ring buffer for critical errors."""
    console.print(Panel("<b>[1] Kernel & Hardware Logs</b>", style="cyan", box=box.MINIMAL))
    
    # Needs root?
    cmd = ["dmesg", "--level=crit,alert,emerg"]
    # Check if we are root? Only if user didn't run genesis with sudo.
    # We'll try running it. If it fails (permission denied), we might retry with sudo or just warn.
    if os.geteuid() != 0:
         cmd.insert(0, "sudo")

    res = _run_command(cmd)
    
    if res.returncode != 0 and "permission" in (res.stderr or "").lower():
         console.print("[red]✘ Permission denied reading kernel logs. Run with sudo.[/red]")
         return

    errors = res.stdout.strip()
    if not errors:
        console.print("[green]✔ No critical kernel errors found.[/green]")
    else:
        console.print("[red]WARNUNG: Critical Kernel Messages:[/red]")
        console.print(errors)

def check_services() -> None:
    """Checks for failed systemd services."""
    console.print("\n" + Panel("<b>[2] Systemd Services</b>", style="cyan", box=box.MINIMAL).render(console))
    
    if not shutil.which("systemctl"):
        console.print("[yellow]Skipping: systemctl not found.[/yellow]")
        return
        
    res = _run_command(["systemctl", "list-units", "--state=failed", "--no-legend"])
    failed_services = res.stdout.strip()
    
    if not failed_services:
        console.print("[green]✔ All started services are running cleanly.[/green]")
    else:
        console.print("[red]WARNUNG: The following services have failed:[/red]")
        # Re-run without no-legend for better output, or just print what we have
        # Better to show full output
        _run_command(["systemctl", "list-units", "--state=failed"], capture=False)

def check_pacman_db() -> None:
    """Checks Pacman database consistency."""
    if not shutil.which("pacman"):
        return

    console.print("\n" + Panel("<b>[3] Pacman Database Integrity</b>", style="cyan", box=box.MINIMAL).render(console))
    
    # pacman -Dk
    cmd = ["pacman", "-Dk"]
    if os.geteuid() != 0: # Pacman db check usually works as user, but better safe? No, actually it often needs write lock even for reading if bad impl?
        # Actually -Dk check validity.
        pass
        
    res = _run_command(cmd)
    if res.returncode == 0:
        console.print("[green]✔ Package database is consistent.[/green]")
    else:
        console.print("[red]✘ ERROR in package database![/red]")
        console.print("[yellow]Recommendation: Run 'sudo pacman -Dk' manually.[/yellow]")

def check_filesystem() -> None:
    """Checks filesystem integrity via pacman (missing files)."""
    if not shutil.which("pacman"):
        return

    console.print("\n" + Panel("<b>[4] Filesystem Integrity</b>", style="cyan", box=box.MINIMAL).render(console))
    console.print("[dim]Scanning installed packages... (This may take a moment)[/dim]")

    # pacman -Qk
    # We pipe stderr to null in bash script "2>/dev/null", here we just ignore stderr?
    # grep -v " 0 missing files"
    
    res = _run_command(["pacman", "-Qk"])
    
    # Filter lines
    lines = res.stdout.splitlines()
    problems = [line for line in lines if " 0 missing files" not in line and line.strip()]
    
    if not problems:
         console.print("[green]✔ Filesystem is 100% intact. No package files missing.[/green]")
    else:
         console.print(f"[red]ATTENTION: Files missing in {len(problems)} packages:[/red]")
         # Limit output if too many?
         if len(problems) > 10:
             for p in problems[:10]:
                 console.print(f"  {p}")
             console.print(f"  [dim]... and {len(problems)-10} more.[/dim]")
         else:
             for p in problems:
                 console.print(f"  {p}")
         
         console.print("[yellow]-> Often harmless (e.g. empty log folders), but critical for binaries.[/yellow]")

def check_users_groups() -> None:
    """Checks consistency of /etc/passwd and /etc/group."""
    console.print("\n" + Panel("<b>[5] User & Group ID Check</b>", style="cyan", box=box.MINIMAL).render(console))
    
    # pwck -r (read-only)
    # usually requires root for full detail but -r might be okay? 
    # pwck operates on /etc/passwd and /etc/shadow. Shadow is not readable by normal user.
    # So this likely needs sudo.
    cmd_sudo = ["sudo"] if os.geteuid() != 0 else []
    
    # pwck
    if shutil.which("pwck"):
        res_pw = _run_command(cmd_sudo + ["pwck", "-r"])
        if res_pw.returncode == 0:
            console.print("[green]✔ User files (/etc/passwd) are clean.[/green]")
        else:
            console.print("[red]WARNUNG: Inconsistencies found in /etc/passwd (pwck).[/red]")
            if res_pw.stdout or res_pw.stderr:
                 console.print(Panel(res_pw.stdout or res_pw.stderr, title="pwck output", expand=False))
    else:
        console.print("[dim]Skipping pwck (tool not found).[/dim]")

    # grpck
    if shutil.which("grpck"):
        res_gr = _run_command(cmd_sudo + ["grpck", "-r"])
        if res_gr.returncode == 0:
            console.print("[green]✔ Group files (/etc/group) are clean.[/green]")
        else:
            console.print("[red]WARNUNG: Inconsistencies found in /etc/group (grpck).[/red]")
            if res_gr.stdout or res_gr.stderr:
                 console.print(Panel(res_gr.stdout or res_gr.stderr, title="grpck output", expand=False))
    else:
        console.print("[dim]Skipping grpck (tool not found).[/dim]")

def check_disk_space() -> None:
    """Checks disk space usage for root."""
    console.print("\n" + Panel("<b>[6] Disk Space (Root)</b>", style="cyan", box=box.MINIMAL).render(console))
    
    res = _run_command(["df", "-h", "/"])
    if res.returncode == 0:
        lines = res.stdout.splitlines()
        # Header is line 0, data is line 1
        if len(lines) >= 2:
            # Filesystem      Size  Used Avail Use% Mounted on
            # /dev/sda1       100G   50G   50G  50% /
            parts = lines[1].split()
            # typically: parts[0]=FS, [1]=Size, [2]=Used, [3]=Avail, [4]=Use%, [5]=Mount
            if len(parts) >= 6:
                used = parts[2]
                total = parts[1]
                mount = parts[5]
                percent = parts[4]
                
                # Check if full
                pct_val = int(percent.strip('%'))
                color = "green"
                if pct_val > 80: color = "yellow"
                if pct_val > 90: color = "red"
                
                console.print(f"Used: [{color}]{used}[/{color}] of {total} (Mounted on {mount})")
                
                # Draw a simple bar?
                width = 40
                filled = int(width * pct_val / 100)
                bar = f"[{color}]" + "█" * filled + "[dim]" + "░" * (width - filled) + "[/dim][/" + color + "]"
                console.print(f"{bar} {percent}")
            else:
                console.print(lines[1])
        else:
            console.print(res.stdout)
    else:
        console.print("[red]Failed to check disk space.[/red]")

def run_health_check() -> None:
    """Runs the full system health audit."""
    console.print(Panel("[bold white]MANJARO LINUX SYSTEM INTEGRITY AUDIT[/bold white]", style="blue", box=box.DOUBLE))
    
    if os.geteuid() != 0:
        console.print("[bold yellow]Note: This tool works best as root (sudo). Some checks might fail or be skipped.[/bold yellow]\n")

    check_kernel()
    check_services()
    check_pacman_db()
    check_filesystem()
    check_users_groups()
    check_disk_space()
    
    console.print("\n[bold blue]Audit complete.[/bold blue]")
