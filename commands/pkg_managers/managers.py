import shutil
import subprocess
import json
import os
import re
from typing import List, Optional
from .base import PackageManager, PackageResult

class PacmanManager(PackageManager):
    @property
    def name(self) -> str:
        return "pacman"

    @property
    def display_name(self) -> str:
        return "Pacman"

    def is_available(self) -> bool:
        return shutil.which("pacman") is not None

    def search(self, query: str) -> List[PackageResult]:
        results = []
        if not self.is_available():
            return results
        
        try:
            # -Ss searches both name and description
            proc = subprocess.run(["pacman", "-Ss", query], capture_output=True, text=True)
            if proc.returncode != 0:
                return results

            lines = proc.stdout.strip().splitlines()
            current_pkg = None
            
            # Pacman output format:
            # core/linux 6.6.1-arch1-1 (base) [installed]
            #     The Linux kernel and modules
            
            for line in lines:
                if not line.startswith("    "):
                    # This is a package line
                    parts = line.split(" ")
                    repo_name = parts[0]
                    repo, name = repo_name.split("/")
                    version = parts[1]
                    installed = "[installed]" in line
                    
                    current_pkg = {
                        "name": name,
                        "version": version,
                        "installed": installed,
                        "description": ""
                    }
                elif current_pkg:
                    # Description line
                    current_pkg["description"] = line.strip()
                    results.append(PackageResult(
                        manager_name="pacman",
                        name=current_pkg["name"],
                        version=current_pkg["version"],
                        description=current_pkg["description"],
                        installed=current_pkg["installed"],
                        identifier=current_pkg["name"]
                    ))
                    current_pkg = None
                    
        except Exception:
            pass
        return results

    def install(self, package: PackageResult) -> bool:
        cmd = ["sudo", "pacman", "-S", package.identifier]
        return subprocess.run(cmd).returncode == 0
        
    def uninstall(self, package: PackageResult) -> bool:
        cmd = ["sudo", "pacman", "-Rs", package.identifier]
        return subprocess.run(cmd).returncode == 0

class PamacManager(PackageManager):
    @property
    def name(self) -> str:
        return "pamac"

    @property
    def display_name(self) -> str:
        return "Pamac (AUR)"

    def is_available(self) -> bool:
        return shutil.which("pamac") is not None

    def search(self, query: str) -> List[PackageResult]:
        results = []
        if not self.is_available():
            return results
            
        try:
            # pamac search -a returns:
            # package-name  version  repo  [installed]
            # Description
            proc = subprocess.run(["pamac", "search", "-a", query], capture_output=True, text=True)
            if proc.returncode != 0:
                return results
                
            lines = proc.stdout.strip().splitlines()
            # Pamac output is a bit harder to parse as it can be multi-line or single line depending on version
            # But typically:
            # firefox  120.0-1  extra  [installed]
            # Web browser
            
            # Let's use a simpler heuristic or assume alternating lines
            current_pkg = None
            for line in lines:
                if not line.startswith("  ") and not line.startswith("\t"):
                    parts = line.split()
                    if len(parts) >= 2:
                        name = parts[0]
                        version = parts[1]
                        installed = "[installed]" in line
                        # Repo is usually 3rd
                        current_pkg = {
                           "name": name,
                           "version": version,
                           "installed": installed,
                           "description": ""
                        }
                elif current_pkg:
                     current_pkg["description"] = line.strip()
                     results.append(PackageResult(
                        manager_name="pamac",
                        name=current_pkg["name"],
                        version=current_pkg["version"],
                        description=current_pkg["description"],
                        installed=current_pkg["installed"],
                        identifier=current_pkg["name"]
                     ))
                     current_pkg = None
        except Exception:
            pass
        return results

    def install(self, package: PackageResult) -> bool:
        cmd = ["pamac", "build", package.identifier]
        return subprocess.run(cmd).returncode == 0

    def uninstall(self, package: PackageResult) -> bool:
        cmd = ["pamac", "remove", package.identifier]
        return subprocess.run(cmd).returncode == 0


class YayManager(PackageManager):
    @property
    def name(self) -> str:
        return "yay"
        
    @property
    def display_name(self) -> str:
        return "Yay (AUR)"
        
    def is_available(self) -> bool:
        return shutil.which("yay") is not None
        
    def search(self, query: str) -> List[PackageResult]:
        results = []
        if not self.is_available():
            return results
            
        try:
            # yay -Ss output is similar to pacman but colorized. 
            # We should suppress color? --nocolor isn't always standard in yay versions but usually works?
            # Or just strip ansi codes.
            env = os.environ.copy()
            env["LC_ALL"] = "C"
            proc = subprocess.run(["yay", "-Ss", query, "--nocolor"], capture_output=True, text=True, env=env)
            
            lines = proc.stdout.strip().splitlines()
            current_pkg = None
            
            for line in lines:
                if not line.startswith("    "):
                     parts = line.split(" ")
                     if "/" in parts[0]:
                         repo, name = parts[0].split("/")
                         version = parts[1]
                         installed = "(Installed)" in line or "[installed]" in line 
                         
                         current_pkg = {
                             "name": name, 
                             "version": version,
                             "installed": installed,
                             "description": ""
                         }
                elif current_pkg:
                     current_pkg["description"] = line.strip()
                     results.append(PackageResult(
                         manager_name="yay",
                         name=current_pkg["name"],
                         version=current_pkg["version"],
                         description=current_pkg["description"],
                         installed=current_pkg["installed"],
                         identifier=current_pkg["name"]
                     ))
                     current_pkg = None
        except Exception:
            pass
        return results
        
    def install(self, package: PackageResult) -> bool:
        cmd = ["yay", "-S", package.identifier]
        return subprocess.run(cmd).returncode == 0

    def uninstall(self, package: PackageResult) -> bool:
        cmd = ["yay", "-R", package.identifier]
        return subprocess.run(cmd).returncode == 0


class AptManager(PackageManager):
    @property
    def name(self) -> str:
        return "apt"

    @property
    def display_name(self) -> str:
        return "APT"

    def is_available(self) -> bool:
        return shutil.which("apt") is not None or shutil.which("apt-get") is not None

    def search(self, query: str) -> List[PackageResult]:
        results = []
        if not self.is_available():
            return results
        
        try:
            # apt search returns:
            # package-name/stable 1.0.0 amd64
            #   Description
            proc = subprocess.run(["apt", "search", query], capture_output=True, text=True)
            lines = proc.stdout.strip().splitlines()
            
            current_pkg = None
            for line in lines:
                if "Sorting..." in line or "Full Text Search..." in line:
                    continue
                if not line.startswith("  "):
                    if "/" in line:
                        parts = line.split("/")
                        name = parts[0]
                        # version info is messy here, maybe use 'apt-cache search' instead?
                        # apt-cache search -n query only gives name and description.
                        # Let's stick to apt search but parse carefully.
                        remain = parts[1] if len(parts) > 1 else ""
                        version_part = remain.split(" ")[1] if " " in remain else ""
                        installed = "[installed]" in line
                        
                        current_pkg = {
                            "name": name,
                            "version": version_part,
                            "installed": installed,
                            "description": ""
                        }
                elif current_pkg:
                    current_pkg["description"] = line.strip()
                    results.append(PackageResult(
                         manager_name="apt",
                         name=current_pkg["name"],
                         version=current_pkg["version"],
                         description=current_pkg["description"],
                         installed=current_pkg["installed"],
                         identifier=current_pkg["name"]
                    ))
                    current_pkg = None
        except Exception:
            pass
        return results

    def install(self, package: PackageResult) -> bool:
        cmd = ["sudo", "apt", "install", package.identifier]
        return subprocess.run(cmd).returncode == 0

    def uninstall(self, package: PackageResult) -> bool:
        cmd = ["sudo", "apt", "remove", package.identifier]
        return subprocess.run(cmd).returncode == 0


class ChocolateyManager(PackageManager):
    @property
    def name(self) -> str:
        return "choco"

    @property
    def display_name(self) -> str:
        return "Chocolatey"

    def is_available(self) -> bool:
        return shutil.which("choco") is not None

    def search(self, query: str) -> List[PackageResult]:
        results = []
        if not self.is_available():
            return results

        try:
            # choco search git --limit-output
            # git|2.43.0|true
            # name|version|installed
            proc = subprocess.run(["choco", "search", query, "--limit-output"], capture_output=True, text=True)
            lines = proc.stdout.strip().splitlines()
            
            for line in lines:
                if "|" in line:
                    parts = line.split("|")
                    if len(parts) >= 2:
                        name = parts[0]
                        version = parts[1]
                        # "true" in 3rd col means installed? No, wait. 
                        # Actually 'choco search --limit-output' doesn't show 'installed' boolean reliably for remote search, 
                        # it shows available versions.
                        # But wait, choco list --local-only shows installed.
                        # Let's assume false for search, or check? checking is expensive.
                        # Actually, looking at docs: name|version
                        # Some versions output 4 cols?
                        # Let's stick to name and version.
                        
                        results.append(PackageResult(
                            manager_name="choco",
                            name=name,
                            version=version,
                            description="", # Choco limit-output doesn't give description
                            installed=False, # Unknown
                            identifier=name
                        ))
        except Exception:
            pass
        return results

    def install(self, package: PackageResult) -> bool:
        # On windows, elevation is tricky. Assuming user runs genesis in elevated shell or choco asks UAC?
        # Choco requires elevation.
        cmd = ["choco", "install", package.identifier, "-y"]
        return subprocess.run(cmd).returncode == 0

    def uninstall(self, package: PackageResult) -> bool:
        cmd = ["choco", "uninstall", package.identifier, "-y"]
        return subprocess.run(cmd).returncode == 0


class WingetManager(PackageManager):
    @property
    def name(self) -> str:
        return "winget"

    @property
    def display_name(self) -> str:
        return "Winget"

    def is_available(self) -> bool:
        return shutil.which("winget") is not None

    def search(self, query: str) -> List[PackageResult]:
        results = []
        if not self.is_available():
            return results

        try:
            # winget search "query"
            # Output is a table.
            # Name      Id          Version    Source
            # ---------------------------------------
            # Git       Git.Git     2.43.0     winget
            
            # This is hard to parse reliably without fixed width or json.
            # Winget doesn't support json output for search easily yet (in some versions).
            # But let's try reading lines.
            
            # Using UTF-8 might be needed on Windows
            env = os.environ.copy()
            env["PYTHONIOENCODING"] = "utf-8"
            
            proc = subprocess.run(["winget", "search", query], capture_output=True, text=True, env=env)
            lines = proc.stdout.strip().splitlines()
            
            header_idx = -1
            for i, line in enumerate(lines):
                if line.startswith("Name") and "Id" in line:
                    header_idx = i
                    break
            
            if header_idx != -1:
                # Find column indices roughly? Or just split by whitespace if names don't have spaces?
                # Names HAVE spaces. "Google Chrome".
                # The columns are usually fixed width based on header.
                # Name (0) | Id (?) | Version (?)
                # Actually, winget output is tricky.
                # Let's try splitting by 2+ spaces.
                
                for line in lines[header_idx+2:]: # Skip header and separator
                     parts = re.split(r'\s{2,}', line.strip())
                     if len(parts) >= 3:
                         name = parts[0]
                         p_id = parts[1]
                         version = parts[2]
                         
                         results.append(PackageResult(
                             manager_name="winget",
                             name=name,
                             version=version,
                             description="",
                             installed=False,
                             identifier=p_id
                         ))
        except Exception:
            pass
        return results

    def install(self, package: PackageResult) -> bool:
        cmd = ["winget", "install", "--id", package.identifier]
        return subprocess.run(cmd).returncode == 0
        
    def uninstall(self, package: PackageResult) -> bool:
        cmd = ["winget", "uninstall", "--id", package.identifier]
        return subprocess.run(cmd).returncode == 0

class FlatpakManager(PackageManager):
    @property
    def name(self) -> str:
        return "flatpak"
        
    @property
    def display_name(self) -> str:
        return "Flatpak"
        
    def is_available(self) -> bool:
        return shutil.which("flatpak") is not None
        
    def search(self, query: str) -> List[PackageResult]:
        results = []
        if not self.is_available():
            return results
            
        try:
            # flatpak search query --columns=name,application,version,description,installed
            # This output is tab separated? Or just aligned? 
            # Flatpak search CLI is meant for humans.
            # But we can assume typical output.
            proc = subprocess.run(["flatpak", "search", query, "--columns=name,application,version,description"], capture_output=True, text=True)
            lines = proc.stdout.strip().splitlines()
            
            # Output:
            # Name    Application ID    Version    Description
            # ...
            
            # Again, split by multiple spaces or tabs? Flatpak usually uses tabs if pipe? No, just spaces.
            # But let's try to be smart.
            
            for line in lines:
                if "Application ID" in line: continue # Header
                
                # Split by \t if connected to pipe? No, flatpak detects tty.
                # Let's try to just split.
                parts = line.split("\t")
                if len(parts) < 2:
                     parts = re.split(r'\s{2,}', line) # Fallback to gaps
                
                if len(parts) >= 3:
                    name = parts[0]
                    app_id = parts[1]
                    version = parts[2]
                    desc = parts[3] if len(parts) > 3 else ""
                    
                    results.append(PackageResult(
                        manager_name="flatpak",
                        name=name,
                        version=version,
                        description=desc,
                        installed=False, # Need separate check
                        identifier=app_id
                    ))
        except Exception:
            pass
        return results
        
    def install(self, package: PackageResult) -> bool:
        cmd = ["flatpak", "install", package.identifier, "-y"]
        return subprocess.run(cmd).returncode == 0
        
    def uninstall(self, package: PackageResult) -> bool:
        cmd = ["flatpak", "uninstall", package.identifier, "-y"]
        return subprocess.run(cmd).returncode == 0


class SnapManager(PackageManager):
    @property
    def name(self) -> str:
        return "snap"
        
    @property
    def display_name(self) -> str:
        return "Snap"
        
    def is_available(self) -> bool:
        return shutil.which("snap") is not None
        
    def search(self, query: str) -> List[PackageResult]:
        results = []
        if not self.is_available():
            return results
            
        try:
            # snap find query
            proc = subprocess.run(["snap", "find", query], capture_output=True, text=True)
            lines = proc.stdout.strip().splitlines()
            
            # Name  Version  Publisher  Notes  Summary
            # ...
            
            for line in lines[1:]: # Skip header
                parts = re.split(r'\s{2,}', line.strip())
                if len(parts) >= 3:
                    name = parts[0]
                    version = parts[1]
                    summary = parts[-1]
                    
                    results.append(PackageResult(
                        manager_name="snap",
                        name=name,
                        version=version,
                        description=summary,
                        installed=False, # snap find doesn't show installation status
                        identifier=name
                    ))
        except Exception:
            pass
        return results
        
    def install(self, package: PackageResult) -> bool:
        cmd = ["sudo", "snap", "install", package.identifier]
        return subprocess.run(cmd).returncode == 0
        
    def uninstall(self, package: PackageResult) -> bool:
        cmd = ["sudo", "snap", "remove", package.identifier]
        return subprocess.run(cmd).returncode == 0


class BrewManager(PackageManager):
    @property
    def name(self) -> str:
        return "brew"
        
    @property
    def display_name(self) -> str:
        return "Homebrew"
        
    def is_available(self) -> bool:
        return shutil.which("brew") is not None
        
    def search(self, query: str) -> List[PackageResult]:
        results = []
        if not self.is_available():
            return results
            
        try:
            # brew search --eval-all --desc query
            # Output: 
            # ==> Formulae
            # git: distributed version control system
            proc = subprocess.run(["brew", "search", "--desc", query], capture_output=True, text=True)
            lines = proc.stdout.strip().splitlines()
            
            for line in lines:
                if line.startswith("==>"): continue
                if ":" in line:
                    parts = line.split(":", 1)
                    name = parts[0].strip()
                    desc = parts[1].strip()
                    results.append(PackageResult(
                        manager_name="brew",
                        name=name,
                        version="", # brew search doesn't show version easily
                        description=desc,
                        installed=False,
                        identifier=name
                    ))
        except Exception:
            pass
        return results
        
    def install(self, package: PackageResult) -> bool:
        cmd = ["brew", "install", package.identifier]
        return subprocess.run(cmd).returncode == 0
        

    def uninstall(self, package: PackageResult) -> bool:
        cmd = ["brew", "uninstall", package.identifier]
        return subprocess.run(cmd).returncode == 0

class DnfManager(PackageManager):
    @property
    def name(self) -> str:
        return "dnf"

    @property
    def display_name(self) -> str:
        return "DNF"

    def is_available(self) -> bool:
        return shutil.which("dnf") is not None

    def search(self, query: str) -> List[PackageResult]:
        results = []
        if not self.is_available():
            return results
        
        try:
            # dnf search query
            # Output format varies, but typically:
            # Package Name : Summary
            # name.arch : Summary
            proc = subprocess.run(["dnf", "search", query, "-q"], capture_output=True, text=True)
            lines = proc.stdout.strip().splitlines()
            
            # Key = Value
            for line in lines:
                if ":" in line:
                    parts = line.split(":", 1)
                    name_part = parts[0].strip()
                    desc = parts[1].strip()
                    
                    # name_part might comprise name.arch
                    name = name_part.split(".")[0]
                    
                    results.append(PackageResult(
                        manager_name="dnf",
                        name=name,
                        version="", # dnf search doesn't show version easily
                        description=desc,
                        installed=False,
                        identifier=name
                    ))
        except Exception:
            pass
        return results

    def install(self, package: PackageResult) -> bool:
        cmd = ["sudo", "dnf", "install", package.identifier, "-y"]
        return subprocess.run(cmd).returncode == 0

    def uninstall(self, package: PackageResult) -> bool:
        cmd = ["sudo", "dnf", "remove", package.identifier, "-y"]
        return subprocess.run(cmd).returncode == 0


class ZypperManager(PackageManager):
    @property
    def name(self) -> str:
        return "zypper"

    @property
    def display_name(self) -> str:
        return "Zypper"

    def is_available(self) -> bool:
        return shutil.which("zypper") is not None

    def search(self, query: str) -> List[PackageResult]:
        results = []
        if not self.is_available():
            return results
            
        try:
            # zypper search query
            # S | Name | Summary | Type
            # --+------+---------+-----
            #   | git  | VCS     | package
            env = os.environ.copy()
            env["LC_ALL"] = "C" # Ensure English
            proc = subprocess.run(["zypper", "search", query], capture_output=True, text=True, env=env)
            lines = proc.stdout.strip().splitlines()
            
            for line in lines:
                if "|" not in line or "--+" in line or "S | Name" in line:
                    continue
                
                parts = line.split("|")
                if len(parts) >= 3:
                     status = parts[0].strip()
                     name = parts[1].strip()
                     summary = parts[2].strip()
                     
                     installed = "i" in status
                     
                     results.append(PackageResult(
                         manager_name="zypper",
                         name=name,
                         version="", 
                         description=summary,
                         installed=installed,
                         identifier=name
                     ))
        except Exception:
            pass
        return results

    def install(self, package: PackageResult) -> bool:
        cmd = ["sudo", "zypper", "install", "-n", package.identifier]
        return subprocess.run(cmd).returncode == 0

    def uninstall(self, package: PackageResult) -> bool:
        cmd = ["sudo", "zypper", "remove", "-n", package.identifier]
        return subprocess.run(cmd).returncode == 0

