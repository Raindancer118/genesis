import subprocess
import os
import re
import datetime
import json
from . import self_update
from rich.progress import Progress  # Assuming python-rich is installed
from rich.console import Console
import shutil
import questionary
from typing import List, Tuple
from pathlib import Path
import shlex
from .config import config


PACMAN_AVAILABLE = shutil.which("pacman") is not None
PAMAC_AVAILABLE = shutil.which("pamac") is not None
APT_GET_BIN = shutil.which("apt-get")
APT_BIN = APT_GET_BIN or shutil.which("apt")
APT_CACHE_BIN = shutil.which("apt-cache")
DPKG_BIN = shutil.which("dpkg")
APT_AVAILABLE = APT_BIN is not None
WINGET_BIN = shutil.which("winget")
WINGET_AVAILABLE = WINGET_BIN is not None
CHOCO_BIN = shutil.which("choco")
CHOCO_AVAILABLE = CHOCO_BIN is not None


def _apt_command(action, *packages, assume_yes=False):
    if not APT_AVAILABLE:
        raise RuntimeError("APT is not available on this system.")

    cmd = [APT_BIN, action]
    if assume_yes:
        cmd.append('-y')
    cmd.extend(packages)
    return cmd

console = Console()


def _run_command(command, stream_output=True):
    """Helper to run a command and handle errors."""
    try:
        if stream_output:
            # For commands that need user interaction or show progress
            return subprocess.run(command, check=True)
        else:
            # For commands where we just need the result
            return subprocess.run(command, check=True, capture_output=True, text=True)
    except FileNotFoundError:
        console.print(
            f"[bold red]Error: Command '{command[0]}' not found. Is it installed and in your PATH?[/bold red]")
        return None
    except subprocess.CalledProcessError as e:
        console.print(f"[bold red]An error occurred while running '{' '.join(command)}'.[/bold red]")
        if not stream_output:
            console.print(f"[red]Stderr: {e.stderr}[/red]")
        return None



def _install_chocolatey():
    """Installs Chocolatey on Windows."""
    console.print("[bold cyan]🍫 Installing Chocolatey...[/bold cyan]")
    try:
        # PowerShell command to install Chocolatey
        ps_command = (
            "Set-ExecutionPolicy Bypass -Scope Process -Force; "
            "[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; "
            "iex ((New-Object System.Net.WebClient).DownloadString('https://community.chocolatey.org/install.ps1'))"
        )
        
        # Run command via PowerShell
        proc = subprocess.run(["powershell", "-NoProfile", "-InputFormat", "None", "-ExecutionPolicy", "Bypass", "-Command", ps_command], check=False)
        
        if proc.returncode == 0:
             console.print("[green]Chocolatey installed successfully.[/green]")
             # Attempt to add to PATH for current session
             # Default path is C:\ProgramData\chocolatey\bin
             choco_bin = os.path.join(os.environ.get("ProgramData", r"C:\ProgramData"), r"chocolatey\bin")
             if os.path.exists(choco_bin) and choco_bin not in os.environ["PATH"]:
                 os.environ["PATH"] += os.pathsep + choco_bin
             return True
        else:
             console.print("[red]Chocolatey installation failed.[/red]")
             return False
    except Exception as e:
        console.print(f"[red]Error installing Chocolatey: {e}[/red]")
        return False


def install_packages(packages):
    """Intelligently finds and installs a list of packages."""
    if PACMAN_AVAILABLE:
        to_install_pacman = []
        to_install_pamac = []
        not_found = []

        for package in packages:
            console.print(f"🔎 Searching for [bold magenta]'{package}'[/bold magenta]...")
            # Check official repos with pacman
            pacman_check = subprocess.run(
                ['pacman', '-Si', package],
                capture_output=True,
            )
            if pacman_check.returncode == 0:
                console.print("  -> Found in official repositories.")
                to_install_pacman.append(package)
            elif PAMAC_AVAILABLE and subprocess.run(
                ['pamac', 'info', package], capture_output=True
            ).returncode == 0:
                console.print("  -> Found in the AUR.")
                to_install_pamac.append(package)
            else:
                not_found.append(package)

        if not_found:
            console.print(
                f"[bold yellow]Warning: Could not find package(s): {', '.join(not_found)}[/bold yellow]"
            )

        if not to_install_pacman and not to_install_pamac:
            console.print("No packages to install.")
            return

        # Confirmation
        console.print("\n--- [bold]Installation Plan[/bold] ---")
        if to_install_pacman:
            console.print(f"Official (pacman): [green]{', '.join(to_install_pacman)}[/green]")
        if to_install_pamac:
            console.print(f"AUR (pamac): [cyan]{', '.join(to_install_pamac)}[/cyan]")

        if to_install_pamac:
            console.print(f"AUR (pamac): [cyan]{', '.join(to_install_pamac)}[/cyan]")

        if config.get("system.default_install_confirm") and not questionary.confirm("Proceed with installation?").ask():
             console.print("Installation cancelled.")
             return

        if True: # Indent block placeholder or just continue
            if to_install_pacman:
                _run_command(['sudo', 'pacman', '-S', '--needed', *to_install_pacman])
            if to_install_pamac:
                _run_command(['pamac', 'build', *to_install_pamac])
            console.print("\n✅ Installation complete.")
        else:
            console.print("Installation cancelled.")
        return

    if APT_AVAILABLE:
        to_install = []
        not_found = []

        for package in packages:
            console.print(f"🔎 Checking availability for [bold magenta]'{package}'[/bold magenta]...")
            if APT_CACHE_BIN:
                result = subprocess.run(
                    [APT_CACHE_BIN, 'policy', package],
                    capture_output=True,
                    text=True,
                )
                if result.returncode == 0 and 'Candidate: (none)' not in result.stdout:
                    to_install.append(package)
                    continue
            else:
                result = None

            if result and result.returncode != 0:
                not_found.append(package)
            elif result and 'Candidate: (none)' in result.stdout:
                not_found.append(package)
            else:
                # Fallback: attempt dry-run install to verify
                dry_run = subprocess.run(
                    ['apt-get', 'install', '--dry-run', package],
                    capture_output=True,
                ) if shutil.which('apt-get') else None
                if dry_run and dry_run.returncode == 0:
                    to_install.append(package)
                else:
                    not_found.append(package)

        if not_found:
            console.print(
                f"[bold yellow]Warning: Could not find package(s): {', '.join(not_found)}[/bold yellow]"
            )

        if not to_install:
            console.print("No packages to install.")
            return

        console.print("\n--- [bold]Installation Plan[/bold] ---")
        console.print(f"APT packages: [green]{', '.join(to_install)}[/green]")

        if config.get("system.default_install_confirm") and not questionary.confirm("Proceed with installation?").ask():
            console.print("Installation cancelled.")
            return

        if True: # Proceed
            cmd = _apt_command('install', *to_install, assume_yes=True)
            _run_command(['sudo', *cmd])
            console.print("\n✅ Installation complete.")
        else:
            console.print("Installation cancelled.")
        return

    # Windows Strategy: Prefer Chocolatey > Winget
    # Check if we are on Windows or if Windows tools are detected
    if os.name == 'nt' or WINGET_AVAILABLE or CHOCO_AVAILABLE:
        
        # 1. Chocolatey Handling
        choco_exe = shutil.which("choco")
        
        # If not installed, ask to install (Only on Windows proper)
        if not choco_exe and os.name == 'nt':
             if questionary.confirm("Chocolatey is not installed but recommended. Install now?", default=True).ask():
                 if _install_chocolatey():
                     choco_exe = shutil.which("choco") or "choco" # Retry detection

        if choco_exe:
            to_install = []
            for package in packages:
                 console.print(f"🔎 Searching for [bold magenta]'{package}'[/bold magenta] via choco...")
                 to_install.append(package)

            if not to_install:
                 return

            if config.get("system.default_install_confirm") and not questionary.confirm("Proceed with installation via Chocolatey?").ask():
                 console.print("Installation cancelled.")
                 return

            # Install
            for pkg in to_install:
                console.print(f"Installing {pkg}...")
                _run_command(['choco', 'install', pkg, '-y'])
            console.print("\n✅ Installation complete.")
            return

        # 2. Winget Handling (Fallback)
        if WINGET_AVAILABLE:
            to_install = []
            for package in packages:
                console.print(f"🔎 Searching for [bold magenta]'{package}'[/bold magenta] via winget...")
                to_install.append(package)

            if not to_install:
                return

            if config.get("system.default_install_confirm") and not questionary.confirm("Proceed with installation via Winget?").ask():
                console.print("Installation cancelled.")
                return

            for pkg in to_install:
                console.print(f"Installing {pkg}...")
                _run_command(['winget', 'install', '-e', '--id', pkg])
            console.print("\n✅ Installation complete.")
            return

    console.print("[bold red]No supported package manager found (pacman/pamac, apt, choco, or winget).[/bold red]")


def remove_packages(packages):
    """Finds and removes a list of installed packages."""
    if PACMAN_AVAILABLE:
        to_remove = []
        not_found = []

        for package in packages:
            if subprocess.run(['pacman', '-Qi', package], capture_output=True).returncode == 0:
                to_remove.append(package)
            else:
                not_found.append(package)

        if not_found:
            console.print(
                f"[bold yellow]Warning: Package(s) not installed: {', '.join(not_found)}[/bold yellow]"
            )

        if not to_remove:
            console.print("No packages to remove.")
            return

        console.print(f"\nPackages to be removed: [red]{', '.join(to_remove)}[/red]")
        if questionary.confirm("Proceed with removal? This will also remove dependencies.").ask():
            _run_command(['sudo', 'pacman', '-Rns', *to_remove])
            console.print("\n✅ Removal complete.")
        else:
            console.print("Removal cancelled.")
        return

    if APT_AVAILABLE:
        to_remove = []
        not_found = []

        for package in packages:
            if DPKG_BIN and subprocess.run([DPKG_BIN, '-s', package], capture_output=True).returncode == 0:
                to_remove.append(package)
            else:
                not_found.append(package)

        if not_found:
            console.print(
                f"[bold yellow]Warning: Package(s) not installed: {', '.join(not_found)}[/bold yellow]"
            )

        if not to_remove:
            console.print("No packages to remove.")
            return

        console.print(f"\nPackages to be removed: [red]{', '.join(to_remove)}[/red]")
        if questionary.confirm("Proceed with removal? This will also remove dependencies.").ask():
            cmd = _apt_command('remove', *to_remove, assume_yes=True)
            _run_command(['sudo', *cmd])
            auto_cmd = _apt_command('autoremove', assume_yes=True)
            _run_command(['sudo', *auto_cmd])
            console.print("\n✅ Removal complete.")
        else:
            console.print("Removal cancelled.")
        return

    # Windows Strategy: Prefer Check Chocolatey > Winget
    if os.name == 'nt' or WINGET_AVAILABLE or CHOCO_AVAILABLE:
        # Check Choco first
        choco_exe = shutil.which("choco")
        
        if choco_exe:
            to_remove = [p for p in packages]
            if not to_remove: return
            
            if questionary.confirm("Proceed with removal via Chocolatey?").ask():
                 for pkg in to_remove:
                     console.print(f"Uninstalling {pkg}...")
                     _run_command(['choco', 'uninstall', pkg, '-y'])
                 console.print("\n✅ Removal complete.")
            else:
                 console.print("Removal cancelled.")
            return

        if WINGET_AVAILABLE:
            to_remove = [p for p in packages]
            if not to_remove: return
            
            if questionary.confirm("Proceed with removal via Winget?").ask():
                 for pkg in to_remove:
                     console.print(f"Uninstalling {pkg}...")
                     _run_command(['winget', 'uninstall', '--id', pkg])
                 console.print("\n✅ Removal complete.")
            else:
                 console.print("Removal cancelled.")
            return
            
    console.print("[bold red]No supported package manager found (pacman/pamac, apt, choco, or winget).[/bold red]")


# --- UPDATED update_system FUNCTION ---

def _update_mirrors():
    """Updates mirror list if reflector is available."""
    if not shutil.which('reflector'):
        return

    console.print("\n[bold cyan]🌍 Updating mirrors (Reflector)...[/bold cyan]")
    try:
        # Arch Linux specific: Update /etc/pacman.d/mirrorlist
        # Requires root usually, so strictly speaking we might need sudo.
        # Reflector usually needs root to write to /etc/pacman.d/mirrorlist
        cmd = [
            'sudo', 'reflector',
            '--latest', '20',
            '--protocol', 'https',
            '--sort', 'rate',
            '--save', '/etc/pacman.d/mirrorlist'
        ]
        _run_command(cmd)
        console.print("[green]Mirrors updated.[/green]")
    except Exception as e:
        console.print(f"[red]Failed to update mirrors: {e}[/red]")


def _manage_timeshift():
    """Creates a Timeshift snapshot and deletes the oldest one."""
    if not shutil.which('timeshift'):
        return

    console.print("\n[bold cyan]🕒 Managing Timeshift snapshots...[/bold cyan]")

    # 1. Create Snapshot
    console.print("Creating new snapshot...")
    # Text file busy fix: Temporarily disable swap to avoid BTRFS snapshot errors
    console.print("[dim]Temporarily disabling swap...[/dim]")
    _run_command(['sudo', 'swapoff', '-a'])

    try:
        try:
            _run_command(['sudo', 'timeshift', '--create', '--comments', 'Genesis Update'])
        except Exception:
            console.print("[red]Failed to create snapshot.[/red]")
            # We continue even if creation fails? Maybe better to warn.
            if not questionary.confirm("Snapshot creation failed. Continue with update?", default=False).ask():
                raise KeyboardInterrupt("Update cancelled by user.")
    finally:
        console.print("[dim]Re-enabling swap...[/dim]")
        _run_command(['sudo', 'swapon', '-a'])

    # 2. Delete Oldest (Pruning)
    # We need to parse 'timeshift --list'
    # Output format is typically:
    # Num  Name                 Tags  Description
    # ------------------------------------------------------------------------------
    # 0    >  2023-10-25_10-00-01  O     ...
    # 1    >  2023-11-01_12-00-00  D     ...
    
    console.print("Checking for old snapshots to prune...")
    try:
        res = subprocess.run(['sudo', 'timeshift', '--list'], capture_output=True, text=True)
        lines = res.stdout.splitlines()
        
        snapshots = []
        # Basic parsing strategy: Look for date-like lines
        # Regex for 'YYYY-MM-DD_HH-MM-SS'
        # Captures the timestamp from the typical timeshift output
        timestamp_re = re.compile(r'(\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2})')
        
        for line in lines:
            match = timestamp_re.search(line)
            if match:
                snapshots.append(match.group(1))

        # Check if we have snapshots to delete
        # Policy: "delete the oldest one" implies we want to keep some?
        # The user said "den ältesten löschen, der vorhanden ist" (delete the oldest one that exists).
        # Assuming we just delete ONE oldest snapshot to save space? Or just cleanup?
        # Usually Timeshift handles rotation automatically, but the user explicitly asked for this.
        # I will delete the *single* oldest snapshot found.
        
        if snapshots:
            # Sort just in case, though usually listed chronologically? 
            # String comparison works for ISO-like dates used by Timeshift.
            snapshots.sort()
            oldest = snapshots[0]
            
            # Don't delete the one we just created if it's the only one!
            # If len > 1, allow deleting the oldest.
            if len(snapshots) > 1:
                console.print(f"Deleting oldest snapshot: [bold]{oldest}[/bold]")
                _run_command(['sudo', 'timeshift', '--delete', '--snapshot', oldest])
            else:
                console.print("[dim]Only one snapshot exists (likely the one just created). Skipping deletion.[/dim]")
        else:
            console.print("[dim]No snapshots found to delete.[/dim]")

    except Exception as e:
        console.print(f"[yellow]Error managing timeshift deletion: {e}[/yellow]")


def update_system(affirmative: bool = False):
    """Performs a full system update tailored to ALL available package managers."""
    console.print("🚀 Starting comprehensive system update...")

    if config.get("system.auto_confirm_update"):
        affirmative = True

    if not affirmative and not questionary.confirm("Proceed with full system update?").ask():
        console.print("Update cancelled.")
        return

    # 1. Mirrors
    if config.get("system.update_mirrors"):
        _update_mirrors()
    else:
        console.print("[dim]Skipping mirror update (disabled in config).[/dim]")

    # 2. Timeshift
    if config.get("system.create_timeshift"):
        try:
            _manage_timeshift()
        except KeyboardInterrupt:
            console.print("[bold red]Aborted.[/bold red]")
            return
    else:
        console.print("[dim]Skipping Timeshift snapshot (disabled in config).[/dim]")
    
    # Wait automatically if affirmative, or we could skip wait?
    # Usually mirror updates happen fast. Timeshift takes time.
    # The user asked for "make -y to simply run through all steps automatically"
    
    console.print("\n[bold cyan]📦 Updating Package Managers...[/bold cyan]")

    # Helper to run updates
    def run_update(name, cmd_list, check_start=None):
        """Runs an update command if the tool exists."""
        # Optional: check dependent command first
        bin_name = check_start if check_start else cmd_list[0]
        if bin_name == 'sudo':
             bin_name = cmd_list[1]
             
        if shutil.which(bin_name):
            console.print(f"\n--- [bold magenta]Updating {name} ({bin_name})[/bold magenta] ---")
            try:
                # Add auto-confirm flags if affirmative
                final_cmd = cmd_list.copy()
                if affirmative:
                    if bin_name in ['pacman', 'paru', 'yay']:
                        if '--noconfirm' not in final_cmd:
                            final_cmd.append('--noconfirm')
                    elif bin_name == 'pamac':
                        if '--no-confirm' not in final_cmd:
                            final_cmd.append('--no-confirm')
                    elif bin_name == 'dnf':
                        if '-y' not in final_cmd:
                            final_cmd.append('-y')
                    # apt, flatpak, snap usually handled individually below or have flags added manually

                _run_command(final_cmd)
            except Exception as e:
                console.print(f"[red]Error updating {name}: {e}[/red]")
        
    # --- Arch / System ---
    # Priority: Pamac > Paru > Yay > Pacman
    # Pamac is preferred on Manjaro.
    # For Paru/Yay, we split updates to ensure official packages use pacman (binary)
    # and only AUR updates go through the helper to avoid unwanted compilation.
    arch_updated = False

    if shutil.which("pamac"):
        run_update("Arch System (Pamac)", ["pamac", "upgrade"])
        arch_updated = True
    elif shutil.which("paru"):
        run_update("Arch System (Pacman)", ["sudo", "pacman", "-Syu"])
        run_update("Arch System (Paru AUR)", ["paru", "-Sua"])
        arch_updated = True
    elif shutil.which("yay"):
        run_update("Arch System (Pacman)", ["sudo", "pacman", "-Syu"])
        run_update("Arch System (Yay AUR)", ["yay", "-Sua"])
        arch_updated = True
    elif shutil.which("pacman"):
        run_update("Arch System (Pacman)", ["sudo", "pacman", "-Syu"])
        arch_updated = True

    # --- Debian / Ubuntu ---
    # Nala > Apt
    if not arch_updated: # Don't run apt if we are on Arch, usually. But `genesis` might check availability.
        # Strict separation: If apt exists, run it.
        if shutil.which("nala"):
             run_update("Debian System (Nala)", ["sudo", "nala", "upgrade"]) # nala upgrade includes update normally or asks
        elif shutil.which("apt"):
             console.print("\n--- [bold magenta]Updating Debian System (Apt)[/bold magenta] ---")
             _run_command(['sudo', *_apt_command('update')])
             _run_command(['sudo', *_apt_command('upgrade', assume_yes=True)])
             _run_command(['sudo', *_apt_command('autoremove', assume_yes=True)])

    # --- Fedora ---
    run_update("Fedora System (DNF)", ["sudo", "dnf", "upgrade", "--refresh"])

    # --- Universal ---
    run_update("Flatpak", ["flatpak", "update", "-y"])
    run_update("Snap", ["sudo", "snap", "refresh"])

    # --- Language Specific ---
    
    # Rust (Cargo)
    # Check for cargo-install-update
    if shutil.which("cargo"):
        # Check if cargo-install-update is installed
        # It's a subcommand: `cargo install-update`
        # We can try running it or check `cargo --list`
        try:
            res = subprocess.run(["cargo", "install-update", "--version"], capture_output=True)
            if res.returncode == 0:
                # -a = all, -g = git? usually -a is enough for cargo-update
                run_update("Rust Crates (Cargo)", ["cargo", "install-update", "-a"])
            else:
                 console.print("\n[dim]Cargo found, but 'cargo-update' crate not installed. Skipping crate updates. (Install with `cargo install cargo-update`)[/dim]")
        except FileNotFoundError:
             pass

    # Node (NPM)
    # Global updates
    if shutil.which("npm"):
        # Often requires sudo for global
        # We will try with sudo.
        run_update("Node Packages (NPM Global)", ["sudo", "npm", "update", "-g"])

    # Ruby (Gem)
    run_update("Ruby Gems", ["gem", "update"])
    
    # Python (Pip)
    # Updating pip itself
    if shutil.which("pip") or shutil.which("pip3"):
        pip_cmd = "pip" if shutil.which("pip") else "pip3"
        # Often dangerous to update system pip.
        # But user asked for "everything".
        # Safer: pipx upgrade-all
        if shutil.which("pipx"):
             run_update("Python Tools (Pipx)", ["pipx", "upgrade-all"])
        
        # We'll skip forcing 'pip install --upgrade pip' globally to avoid breaking distro-managed pip.
        console.print(f"\n[dim]Skipping global 'pip' self-update to protect system integrity. Use 'pipx' for global tools.[/dim]")

    # --- Windows ---
    if os.name == 'nt' or WINGET_AVAILABLE or CHOCO_AVAILABLE:
        if shutil.which("choco"):
            run_update("Windows Software (Chocolatey)", ["choco", "upgrade", "all", "-y"])

        if shutil.which("winget"):
            run_update("Windows Software (Winget)", ["winget", "upgrade", "--all"])


    console.print("\n✅ Full system update process complete.")

# =========================
# ClamAV Smart Scan (Profiles)
# =========================

# Aggressive, aber vernünftige Defaults – kannst du jederzeit anpassen:
_EXCLUDE_DIR_PATTERNS = [
    r"^/proc($|/)", r"^/sys($|/)", r"^/dev($|/)", r"^/run($|/)", r"^/snap($|/)",
    r"^/var/lib/docker($|/)", r"^/var/cache/pacman/pkg($|/)", r"^/var/tmp($|/)",
    r"(^|/)\.cache($|/)", r"(^|/)node_modules($|/)", r"(^|/)\.venv($|/)", r"(^|/)venv($|/)",
    r"(^|/)\.git($|/)", r"(^|/)\.idea($|/)", r"(^|/)\.vscode($|/)",
    r"(^|/)\.local/share/Trash($|/)", r"(^|/)\.npm($|/)", r"(^|/)\.gradle($|/)", r"(^|/)\.m2($|/)",
    r"(^|/)\.rustup($|/)", r"(^|/)\.cargo($|/)", r"(^|/)\.steam($|/)", r"(^|/)\.var/app($|/)",
    r"(^|/)\.wine($|/)", r"(^|/)target($|/)", r"(^|/)build($|/)", r"(^|/)dist($|/)",
]
if os.name == 'nt':
    # Adjust regex for Windows paths if needed, or rely solely on Defender excludes
    pass

# Max-Größen für schnelle Profile (verhindert 17h-Läufe an Archiven/Images).
# Full ignoriert diese Limits.
_QUICK_LIMITS = [
    "--max-filesize=50M",
    "--max-scansize=300M",
    "--max-recursion=12",
    "--max-dir-recursion=6",
    "--bytecode-timeout=60000",
]

# =========================
# Helpers
# =========================

def _which(cmd: str) -> str | None:
    return shutil.which(cmd)

def _run(cmd: list[str], check: bool = True, capture: bool = False, text: bool = True) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, check=check, capture_output=capture, text=text)

def _systemctl_exists(unit: str) -> bool:
    if not _which("systemctl"):
        return False
    res = subprocess.run(["systemctl", "status", unit], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return res.returncode in (0, 3)  # 0=active, 3=inactive but present

def _is_root() -> bool:
    try:
        return os.geteuid() == 0
    except AttributeError:
        return False

def _read_xdg_dirs() -> dict[str, Path]:
    """Parst ~/.config/user-dirs.dirs und liefert Mappings (Desktop,Documents,Downloads,Music,Pictures,Videos)."""
    result: dict[str, Path] = {}
    cfg = Path.home() / ".config" / "user-dirs.dirs"
    if cfg.exists():
        for line in cfg.read_text(encoding="utf-8", errors="ignore").splitlines():
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            k, v = line.split("=", 1)
            k = k.strip()
            v = v.strip().strip('"')
            v = v.replace("$HOME", str(Path.home()))
            try:
                result[k] = Path(v)
            except Exception:
                pass

    # Fallbacks (EN/DE)
    defaults = {
        "XDG_DESKTOP_DIR": ["Desktop", "Schreibtisch"],
        "XDG_DOCUMENTS_DIR": ["Documents", "Dokumente"],
        "XDG_DOWNLOAD_DIR": ["Downloads"],
        "XDG_MUSIC_DIR": ["Music", "Musik"],
        "XDG_PICTURES_DIR": ["Pictures", "Bilder"],
        "XDG_VIDEOS_DIR": ["Videos"],
    }
    for key, names in defaults.items():
        if key not in result:
            for name in names:
                p = Path.home() / name
                if p.exists():
                    result[key] = p
                    break
    return result

def _paths_existing(paths: List[Path]) -> List[Path]:
    return [p for p in paths if p.exists()]

def _confirm(prompt: str) -> bool:
    try:
        import questionary
        return bool(questionary.confirm(prompt, default=True).ask())
    except Exception:
        # Non-interaktiv: still zustimmen
        return True

def _get_usb_drives() -> List[Path]:
    """
    Detects and returns a list of mounted USB drive paths.
    Uses lsblk to identify USB devices and their mount points.
    """
    usb_mounts: List[Path] = []
    
    def _add_mountpoint(mountpoint: str | None) -> None:
        """Helper to validate and add a mountpoint to the list."""
        if mountpoint:
            path = Path(mountpoint)
            if path.exists() and path.is_dir():
                usb_mounts.append(path)
    
    try:
        # Use lsblk with JSON output for reliable parsing
        result = subprocess.run(
            ["lsblk", "-J", "-o", "NAME,TRAN,MOUNTPOINT"],
            capture_output=True,
            text=True,
            check=False
        )
        
        if result.returncode != 0:
            return usb_mounts
        
        # Parse JSON output
        data = json.loads(result.stdout)
        
        for device in data.get("blockdevices", []):
            # Check if parent device is USB
            is_usb_device = device.get("tran") == "usb"
            
            if is_usb_device:
                _add_mountpoint(device.get("mountpoint"))
                
                # Check children (partitions) - they inherit USB transport from parent
                for child in device.get("children", []):
                    _add_mountpoint(child.get("mountpoint"))
        
    except Exception as e:
        console.print(f"[yellow]Warning: Could not detect USB drives: {e}[/yellow]")
    
    # Remove duplicates while preserving order
    seen = set()
    unique_mounts = []
    for mount in usb_mounts:
        if mount not in seen:
            seen.add(mount)
            unique_mounts.append(mount)
    
    return unique_mounts


def _get_windows_drives() -> List[Path]:
    """Detects fixed and removable drives on Windows."""
    drives = []
    bitmask = subprocess.run(["wmic", "logicaldisk", "get", "name"], capture_output=True, text=True)
    # Generic check or use psutil in future
    # Using psutil for cross-platform is better if available (it is installed)
    try:
        import psutil
        for part in psutil.disk_partitions():
            if 'cdrom' in part.opts or part.fstype == '':
                continue
            drives.append(Path(part.mountpoint))
    except Exception:
        pass
    return drives

# =========================
# freshclam
# =========================

def _run_freshclam() -> None:
    """
    Aktualisiert die Signaturen VOR JEDEM Scan.
    - Wenn der freshclam-Dienst existiert, wird er bevorzugt neu angestoßen.
    - Fallback: `sudo freshclam` direkt.
    """
    console.print("\n[bold cyan]🔄 Updating virus definitions (freshclam)…[/bold cyan]")

    service_name = "clamav-freshclam.service"
    if _systemctl_exists(service_name):
        # try-restart → aktualisiert ohne doppelte Instanzen
        cmd = ["sudo", "systemctl", "try-restart", service_name]
        res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        if res.returncode == 0:
            console.print("[green]freshclam service refreshed.[/green]")
            return
        # Fallback falls try-restart fehlschlägt:
        subprocess.run(["sudo", "systemctl", "restart", service_name], check=False)
        console.print("[yellow]freshclam service restarted.[/yellow]")
        return

    # Kein systemd-Service → direkt ausführen
    freshclam = _which("freshclam")
    if not freshclam:
        console.print("[red]freshclam not found. Install 'clamav' first.[/red]")
        return
    # Root notwendig (Standard DB-Pfad /var/lib/clamav)
    cmd = ["sudo", freshclam, "-v"]
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if res.returncode == 0:
        console.print("[green]Signatures up to date.[/green]")
    else:
        # Häufig: „Another freshclam daemon is running“ → tolerieren
        if "another freshclam" in (res.stderr or "").lower():
            console.print("[yellow]freshclam daemon already running; proceeding.[/yellow]")
        else:
            console.print(f"[bold red]freshclam error:[/bold red]\n{res.stderr.strip() or res.stdout.strip()}")

def _run_defender_scan(paths: List[Path]) -> None:
    """Run Windows Defender Scan on specific paths."""
    # MpCmdRun.exe path
    mpcmd = r"C:\Program Files\Windows Defender\MpCmdRun.exe"
    if not os.path.exists(mpcmd):
        # Checks various locations?
        candidate = shutil.which("MpCmdRun.exe")
        if candidate:
            mpcmd = candidate
        else:
             console.print("[red]Windows Defender (MpCmdRun.exe) not found.[/red]")
             return

    console.print(f"\n[bold cyan]🛡️  Windows Defender Scan[/bold cyan]")
    
    for p in paths:
        console.print(f"Scanning {p}...")
        # -Scan -ScanType 3 -File <path>
        # Note: ScanType 3 is Custom Scan
        cmd = [mpcmd, "-Scan", "-ScanType", "3", "-File", str(p)]
        res = subprocess.run(cmd, capture_output=True, text=True)
        
        if res.returncode == 0:
            console.print(f"[green]Clean: {p}[/green]")
        elif res.returncode == 2:
             console.print(f"[bold red]THREAT DETECTED in: {p}[/bold red]")
             console.print(res.stdout)
        else:
             console.print(f"[yellow]Scan error or cancellation for {p} (Code {res.returncode})[/yellow]")
             if res.stdout: console.print(res.stdout)

# =========================
# Build Profiles
# =========================

def _profile_paths(profile: str) -> Tuple[str, List[Path], List[str]]:
    """
    Liefert (title, include_paths, extra_flags) je Profil.
    extra_flags: z.B. Limits und Recursion für schnelle Scans.
    """
    home = Path.home()
    xdg = _read_xdg_dirs()

    # System Core – relativ kompakt halten; libs sind riesig und werden bei Full abgedeckt
    system_core = _paths_existing([
        Path("/etc"),
        Path("/bin"),
        Path("/sbin"),
        Path("/usr/bin"),
        Path("/usr/sbin"),
        Path("/usr/local/bin"),
        Path("/boot"),
        Path("/opt"),
    ])

    # Daily Use – XDG/DE/EN Standards
    daily = _paths_existing([
        xdg.get("XDG_DESKTOP_DIR", home / "Desktop"),
        xdg.get("XDG_DOCUMENTS_DIR", home / "Documents"),
        xdg.get("XDG_DOWNLOAD_DIR", home / "Downloads"),
        xdg.get("XDG_PICTURES_DIR", home / "Pictures"),
        xdg.get("XDG_VIDEOS_DIR", home / "Videos"),
        xdg.get("XDG_MUSIC_DIR", home / "Music"),
        Path(home / "Dokumente"),
        Path(home / "Bilder"),
        Path(home / "Schreibtisch"),
    ])

    # Rest – top-level im Home, die NICHT in daily sind; keine Dotdirs
    daily_set = {p.resolve() for p in daily}
    rest_candidates = []
    for item in sorted(home.iterdir()):
        try:
            if item.name.startswith("."):
                continue
            if item.resolve() in daily_set:
                continue
            if item.is_dir() or item.is_file():
                rest_candidates.append(item)
        except Exception:
            continue
    rest = _paths_existing(rest_candidates)

    # Full – breite Abdeckung mit vernünftigen Excludes
    full = _paths_existing([
        Path("/etc"),
        Path("/usr"),
        Path("/var"),
        Path("/opt"),
        Path("/boot"),
        Path("/home"),
        Path("/mnt"),
        Path("/media"),
    ])

    if profile == "core":
        return ("System Core", system_core, list(_QUICK_LIMITS))
    if profile == "daily":
        return ("Daily Use", daily, list(_QUICK_LIMITS))
    if profile == "rest":
        return ("Home (Rest)", rest, list(_QUICK_LIMITS))
    if profile == "full":
        return ("Full System", full, [])  # keine Größenlimits bei Full

    if profile == "full":
        return ("Full System", full, [])  # keine Größenlimits bei Full

    if os.name == 'nt':
        # Windows specific profiles overrides if needed
        # We try to map the same logic to Windows paths mostly works via pathlib
        # but 'Full System' should be C:\
        if profile == "full":
             return ("Full System (C:)", [Path("C:/")], [])
        # 'core' on Windows? C:\Windows, C:\Program Files?
        if profile == "core":
             return ("System Core", [Path("C:/Windows"), Path("C:/Program Files")], [])

    raise ValueError(f"Unknown profile: {profile}")

def _exclude_args() -> List[str]:
    """Erzeugt mehrere --exclude-dir Flags aus der Patternliste."""
    args: List[str] = []
    for pat in _EXCLUDE_DIR_PATTERNS:
        args += ["--exclude-dir", pat]
    return args

def _best_scanner() -> Tuple[str, List[str]]:
    """
    Bevorzugt 'clamdscan', wenn verfügbar & clamd läuft (viel schneller).
    Fallback: 'clamscan'.
    JETZT GEÄNDERT: Erzwingt clamscan, um Berechtigungsprobleme im Home-Dir zu umgehen.
    """
    # clamdscan = _which("clamdscan")
    # if clamdscan and _systemctl_exists("clamav-daemon.service") or _systemctl_exists("clamd.service"):
    #     return (clamdscan, ["--fdpass"])
    clamscan = _which("clamscan")
    if not clamscan:
        raise RuntimeError("Neither clamdscan nor clamscan found. Please install clamav.")
    # Das Flag "-r" bedeutet "rekursiv", was für das Scannen von Verzeichnissen notwendig ist.
    return (clamscan, ["-r"])

def _summarize_scan(stdout: str) -> str:
    marker = "----------- SCAN SUMMARY -----------"
    if marker in stdout:
        return stdout.split(marker, 1)[-1].strip()
    return stdout[-2000:].strip()  # Fallback: letzte 2k Zeichen

# =========================
# Public API
# =========================

def smart_scan(profile: str | None = None) -> None:
    """
    Interaktiver ClamAV-Scan mit Profilen.
    - profile in {"core","daily","rest","full"} oder None → Auswahlmenü.
    - Führt IMMER vorher freshclam aus.
    """
    valid = {"core": "System Core", "daily": "Daily Use", "rest": "Home (Rest)", "full": "Full System"}
    if profile is None:
        try:
            import questionary
            choice = questionary.select(
                "Choose a scan profile:",
                choices=[
                    ("System Core (fast, essential areas)", "core"),
                    ("Daily Use (fast, your common dirs)", "daily"),
                    ("Home: Rest (fast, remaining top-level in $HOME)", "rest"),
                    ("Full System (slow, widest coverage)", "full"),
                ],
            ).ask()
            profile = choice or "core"
        except Exception:
            profile = "core"

    if profile not in valid:
        console.print(f"[red]Invalid profile '{profile}'. Using 'core'.[/red]")
        profile = "core"

    title, include_paths, extra_flags = _profile_paths(profile)

    if not include_paths:
        console.print(f"[yellow]No existing paths for profile '{title}'. Nothing to scan.[/yellow]")
        return

    console.print(f"\n[bold cyan]🛡️  ClamAV Smart Scan — {title}[/bold cyan]")
    console.print("[dim]Will exclude common cache/build dirs to keep it fast.[/dim]")

    # Übersicht anzeigen
    for p in include_paths:
        console.print(f" • {p}")

    if not _confirm("Proceed with this selection?"):
        console.print("[yellow]Scan cancelled.[/yellow]")
        return

    # Always update signatures first
    _run_freshclam()

    # Build command
    scanner, base_flags = _best_scanner()
    args = [scanner] + base_flags + ["--stdout", "--infected"]  # nur Funde ausgeben
    args += _exclude_args()
    args += extra_flags
    # clamdscan akzeptiert keine --max-* Optionen; ignoriert still → okay.
    # Wichtig: Pfade anhängen
    args += [str(p) for p in include_paths]

    if os.name == 'nt':
        # Use Defender logic instead
        # For 'core', 'daily' etc profiles, pass the list of paths to defender loop
        _run_defender_scan(include_paths)
        return

    console.print("\n[bold cyan]🚀 Scanning…[/bold cyan]")

    console.print("\n[bold cyan]🚀 Scanning…[/bold cyan]")
    console.print(f"[dim]{shlex.join(args)}[/dim]")

    # Streaming-Ausgabe (nicht vorher Dateien zählen → schnell starten!)
    proc = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    found_counter = 0
    try:
        assert proc.stdout is not None
        for line in proc.stdout:
            line = line.rstrip("\n")
            # Nur Funde oder Warnungen hervorheben
            if line.endswith("FOUND"):
                found_counter += 1
                console.print(f"[bold red]{line}[/bold red]")
            elif "WARNING:" in line:
                console.print(f"[yellow]{line}[/yellow]")
    finally:
        stdout, stderr = proc.communicate()

    if proc.returncode not in (0, 1):  # 0=clean, 1=found infected
        console.print(f"[bold red]Scanner error (exit {proc.returncode}).[/bold red]")
        if stderr:
            console.print(stderr.strip())
        return

    # Summary
    summary = _summarize_scan(stdout or "")
    console.print("\n[bold green]— Scan Summary —[/bold green]")
    console.print(summary if summary else f"Found: {found_counter}")

    if found_counter > 0:
        console.print("\n[bold yellow]Action:[/bold yellow] Review detections. "
                      "You can quarantine or delete with clamscan options like --remove or move to a quarantine dir. "
                      "I kept scans read-only by default.")

def scan_usb_drives() -> None:
    """
    Scans all connected USB drives for viruses.
    """
    usb_drives = _get_usb_drives()
    
    if not usb_drives:
        console.print("[yellow]No USB drives detected.[/yellow]")
        return
    
    console.print("\n[bold cyan]🔌 Detected USB drives:[/bold cyan]")
    for drive in usb_drives:
        console.print(f" • {drive}")
    
    if not _confirm("\nProceed with scanning these USB drives?"):
        console.print("[yellow]Scan cancelled.[/yellow]")
        return
    
    # Always update signatures first
    _run_freshclam()
    
    # Build command
    scanner, base_flags = _best_scanner()
    args = [scanner] + base_flags + ["--stdout", "--infected"]
    if os.name == 'nt':
        _run_defender_scan(usb_drives)
        return
    args += _exclude_args()
    args += list(_QUICK_LIMITS)
    args += [str(p) for p in usb_drives]
    
    console.print("\n[bold cyan]🚀 Scanning USB drives…[/bold cyan]")
    console.print(f"[dim]{shlex.join(args)}[/dim]")
    
    # Streaming-Ausgabe
    proc = subprocess.Popen(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    found_counter = 0
    try:
        assert proc.stdout is not None
        for line in proc.stdout:
            line = line.rstrip("\n")
            if line.endswith("FOUND"):
                found_counter += 1
                console.print(f"[bold red]{line}[/bold red]")
            elif "WARNING:" in line:
                console.print(f"[yellow]{line}[/yellow]")
    finally:
        stdout, stderr = proc.communicate()
    
    if proc.returncode not in (0, 1):
        console.print(f"[bold red]Scanner error (exit {proc.returncode}).[/bold red]")
        if stderr:
            console.print(stderr.strip())
        return
    
    # Summary
    summary = _summarize_scan(stdout or "")
    console.print("\n[bold green]— Scan Summary —[/bold green]")
    console.print(summary if summary else f"Found: {found_counter}")
    
    if found_counter > 0:
        console.print("\n[bold yellow]Action:[/bold yellow] Review detections. "
                      "You can quarantine or delete with clamscan options like --remove or move to a quarantine dir. "
                      "I kept scans read-only by default.")

# Optional: Behalte deine alte Funktion, aber leite um:
def scan_directory(path: str) -> None:
    """
    Backward-compat Wrapper (nicht interaktiv).
    Behält dein altes Command bei, aber schneller:
    - nutzt 'daily' Profil, wenn Pfad == $HOME
    - sonst scannt es nur genau diesen Pfad, mit Excludes & Limits
    - macht vorher freshclam
    """
    target = Path(path).expanduser().resolve()
    if not target.exists():
        print(f"Path '{target}' does not exist.")
        return

    # Update signatures
    _run_freshclam()

    # Build minimal command
    scanner, base_flags = _best_scanner()
    args = [scanner] + base_flags + ["--stdout", "--infected"]
    args += _exclude_args()
    args += list(_QUICK_LIMITS)
    args.append(str(target))

    args.append(str(target))

    if os.name == 'nt':
        _run_defender_scan([target])
        return

    args.append(str(target))

    if os.name == 'nt':
        _run_defender_scan([Path(target)])
        return

    print(" ".join(shlex.quote(a) for a in args))
    proc = subprocess.run(args, capture_output=True, text=True)
    print("\n--- Scan Summary ---")
    print(_summarize_scan(proc.stdout))


def interactive_scan_menu() -> None:
    """
    Presents an interactive menu for scan options using arrow keys for navigation.
    """
    console.print("\n[bold cyan]🛡️  Genesis Virus Scanner[/bold cyan]")
    
    try:
        choice = questionary.select(
            "Choose a scan option:",
            choices=[
                ("Scan USB drives (connected USB devices)", "usb"),
                ("Scan daily use areas (Downloads, Documents, Desktop, etc.)", "daily"),
                ("Scan system core (fast, essential system areas)", "core"),
                ("Scan home directory (remaining areas in your home)", "rest"),
                ("Scan root directory (detailed scan of /)", "root"),
                ("Full system scan (slow, comprehensive scan)", "full"),
                ("Cancel", "cancel"),
            ],
        ).ask()
        
        if choice == "usb":
            scan_usb_drives()
        elif choice == "daily":
            smart_scan("daily")
        elif choice == "core":
            smart_scan("core")
        elif choice == "rest":
            smart_scan("rest")
        elif choice == "root":
            console.print("\n[bold cyan]Scanning root directory /[/bold cyan]")
            scan_directory("/")
        elif choice == "full":
            smart_scan("full")
        elif choice == "cancel" or choice is None:
            console.print("[yellow]Scan cancelled.[/yellow]")
        else:
            console.print(f"[red]Invalid choice '{choice}'.[/red]")
    except (KeyboardInterrupt, EOFError):
        console.print("\n[yellow]Scan cancelled.[/yellow]")
    except Exception as e:
        console.print(f"[red]Error displaying interactive menu: {e}[/red]")
        console.print("[yellow]Falling back to 'core' scan.[/yellow]")
        smart_scan("core")


def install_package(package):
    """Finds and installs a package."""
    if PACMAN_AVAILABLE and subprocess.run(['pacman', '-Si', package], capture_output=True).returncode == 0:
        print(f"Package '{package}' found in official repositories. Using pacman...")
        subprocess.run(['sudo', 'pacman', '-S', package])
        return

    if PAMAC_AVAILABLE and subprocess.run(['pamac', 'info', package], capture_output=True).returncode == 0:
        print(f"Package '{package}' found in the AUR. Using pamac...")
        subprocess.run(['pamac', 'build', package])
        return

    if APT_AVAILABLE:
        if APT_CACHE_BIN:
            result = subprocess.run([APT_CACHE_BIN, 'policy', package], capture_output=True, text=True)
            candidate_exists = result.returncode == 0 and 'Candidate: (none)' not in result.stdout
        else:
            candidate_exists = True

        if candidate_exists:
            print(f"Package '{package}' found in APT repositories. Installing...")
            cmd = _apt_command('install', package, assume_yes=True)
            subprocess.run(['sudo', *cmd], check=False)
            return

    print(f"Package '{package}' not found via available package managers.")
