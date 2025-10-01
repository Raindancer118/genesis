import subprocess
import os
from . import self_update
from rich.progress import Progress  # Assuming python-rich is installed
from rich.console import Console
import shutil
import questionary


PACMAN_AVAILABLE = shutil.which("pacman") is not None
PAMAC_AVAILABLE = shutil.which("pamac") is not None
APT_GET_BIN = shutil.which("apt-get")
APT_BIN = APT_GET_BIN or shutil.which("apt")
APT_CACHE_BIN = shutil.which("apt-cache")
DPKG_BIN = shutil.which("dpkg")
APT_AVAILABLE = APT_BIN is not None


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

        if questionary.confirm("Proceed with installation?").ask():
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

        if questionary.confirm("Proceed with installation?").ask():
            cmd = _apt_command('install', *to_install, assume_yes=True)
            _run_command(['sudo', *cmd])
            console.print("\n✅ Installation complete.")
        else:
            console.print("Installation cancelled.")
        return

    console.print("[bold red]No supported package manager found (pacman/pamac or apt).[/bold red]")


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

    console.print("[bold red]No supported package manager found (pacman/pamac or apt).[/bold red]")


# --- UPDATED update_system FUNCTION ---
def update_system():
    """Performs a full system update tailored to the available package manager."""
    console.print("🚀 Starting comprehensive system update...")

    if not questionary.confirm("Proceed with full system update?").ask():
        console.print("Update cancelled.")
        return

    if PAMAC_AVAILABLE:
        console.print("\n--- [bold cyan]Updating Pacman/AUR packages with Pamac[/bold cyan] ---")
        _run_command(['pamac', 'upgrade'])
    elif PACMAN_AVAILABLE:
        console.print("\n--- [bold cyan]Updating Pacman packages[/bold cyan] ---")
        _run_command(['sudo', 'pacman', '-Syu'])
    elif APT_AVAILABLE:
        console.print("\n--- [bold cyan]Updating APT packages[/bold cyan] ---")
        _run_command(['sudo', *_apt_command('update')])
        _run_command(['sudo', *_apt_command('upgrade', assume_yes=True)])
        _run_command(['sudo', *_apt_command('autoremove', assume_yes=True)])
    else:
        console.print("[bold red]No supported package manager found for updates.[/bold red]")

    if shutil.which('flatpak'):
        console.print("\n--- [bold cyan]Updating Flatpak packages[/bold cyan] ---")
        _run_command(['flatpak', 'update', '-y'])
    else:
        console.print("\n[dim]Flatpak not found, skipping.[/dim]")

    if shutil.which('snap'):
        console.print("\n--- [bold cyan]Updating Snap packages[/bold cyan] ---")
        _run_command(['sudo', 'snap', 'refresh'])
    else:
        console.print("\n[dim]Snap not found, skipping.[/dim]")

    console.print("\n✅ Full system update process complete.")


def scan_directory(path):
    """Wrapper for clamscan that estimates progress."""
    print(f"Counting files in '{path}' for progress estimation...")
    # This is a bit slow for huge directories, but necessary for a progress bar
    total_files = int(subprocess.check_output(f"find '{path}' -type f | wc -l", shell=True).strip())

    print(f"Starting scan of {total_files} files...")
    process = subprocess.Popen(
        ['clamscan', '-r', '--stdout', path],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )

    scanned_files = 0
    with Progress() as progress:
        task = progress.add_task("[green]Scanning...", total=total_files)
        # We can't read line-by-line easily as clamscan buffers its output,
        # so we'll just show a generic "running" progress bar. A true line-by-line
        # progress bar would require more advanced process handling.
        process.wait()  # Wait for the scan to complete
        progress.update(task, completed=total_files)

    print("\n--- Scan Summary ---")
    stdout, stderr = process.communicate()
    print(stdout.split('----------- SCAN SUMMARY -----------')[-1].strip())


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
