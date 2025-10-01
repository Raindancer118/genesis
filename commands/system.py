import subprocess
import os
from rich.progress import Progress  # Assuming python-rich is installed


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
    # Check official repos first
    if subprocess.run(['pacman', '-Si', package], capture_output=True).returncode == 0:
        print(f"Package '{package}' found in official repositories. Using pacman...")
        subprocess.run(['sudo', 'pacman', '-S', package])
        return

    # Check AUR
    if subprocess.run(['pamac', 'info', package], capture_output=True).returncode == 0:
        print(f"Package '{package}' found in the AUR. Using pamac...")
        subprocess.run(['pamac', 'build', package])
        return

    print(f"Package '{package}' not found in pacman or the AUR.")