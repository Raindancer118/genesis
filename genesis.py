#!/usr/bin/env python3
import os
import sys
import subprocess
import shutil
from pathlib import Path

def main():
    """
    Genesis Migration Shim
    ----------------------
    This script replaces the legacy Python entry point. 
    It detects that Genesis has upgraded to the Rust edition, 
    builds the new binary, and updates the installation.
    """
    # Color codes
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    RESET = '\033[0m'
    BOLD = '\033[1m'

    print(f"{CYAN}{BOLD}🚀 Genesis is migrating to Rust Edition...{RESET}")

    project_root = Path(__file__).parent.resolve()
    target_bin = project_root / "target" / "release" / "genesis"
    
    # Check for Rust toolchain
    if not shutil.which("cargo"):
        print(f"{RED}Error: Rust (cargo) is not installed.{RESET}")
        print(f"{YELLOW}Please install Rust to continue using Genesis:{RESET}")
        print("  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh")
        sys.exit(1)

    # Build
    print(f"{YELLOW}Building Genesis binary (optimization enabled)...{RESET}")
    try:
        subprocess.run(["cargo", "build", "--release"], cwd=project_root, check=True)
    except subprocess.CalledProcessError:
        print(f"{RED}Build failed. Please check the errors above.{RESET}")
        sys.exit(1)

    if not target_bin.exists():
        print(f"{RED}Binary not found at {target_bin}{RESET}")
        sys.exit(1)

    print(f"{GREEN}Build successful.{RESET}")

    # Update Symlink
    # We try to find where 'genesis' command points to.
    # Usually /usr/local/bin/genesis -> /opt/genesis/genesis.py
    
    genesis_cmd = shutil.which("genesis")
    if genesis_cmd:
        genesis_path = Path(genesis_cmd)
        if genesis_path.is_symlink():
            link_target = genesis_path.resolve()
            # If it points to this script, we want to change it to the binary
            if link_target == Path(__file__).resolve():
                print(f"{CYAN}Updating symlink {genesis_path} -> {target_bin}{RESET}")
                try:
                    # Need sudo? probably.
                    if os.access(genesis_path.parent, os.W_OK):
                         genesis_path.unlink()
                         genesis_path.symlink_to(target_bin)
                    else:
                         print(f"{YELLOW}Sudo required to update symlink.{RESET}")
                         subprocess.run(["sudo", "ln", "-sf", str(target_bin), str(genesis_path)], check=True)
                except Exception as e:
                     print(f"{RED}Failed to update symlink: {e}{RESET}")
                     print(f"You may need to manually link: sudo ln -sf {target_bin} /usr/local/bin/genesis")

    # Pass execution to the new binary for this run
    print(f"{GREEN}Migration complete. Launching Genesis...{RESET}\n")
    
    # Forward arguments
    args = [str(target_bin)] + sys.argv[1:]
    os.execv(str(target_bin), args)

if __name__ == "__main__":
    main()
