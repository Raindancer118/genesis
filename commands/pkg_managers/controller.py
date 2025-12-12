import json
import os
import concurrent.futures
from pathlib import Path
from typing import List, Dict, Optional
from rich.console import Console
from .base import PackageManager, PackageResult
from .managers import (
    PacmanManager, PamacManager, YayManager,
    AptManager, DnfManager, ZypperManager,
    BrewManager, ChocolateyManager, WingetManager,
    FlatpakManager, SnapManager
)

CACHE_FILE = Path.home() / ".cache" / "genesis" / "last_search.json"

class PackageManagerController:
    def __init__(self):
        self.managers: List[PackageManager] = []
        # Register all known managers
        possible_managers = [
            PacmanManager(), PamacManager(), YayManager(),
            AptManager(), DnfManager(), ZypperManager(),
            BrewManager(), ChocolateyManager(), WingetManager(),
            FlatpakManager(), SnapManager()
        ]
        
        for pm in possible_managers:
            if pm.is_available():
                self.managers.append(pm)

    def search(self, query: str) -> List[PackageResult]:
        """Searches all available package managers in parallel."""
        all_results = []
        
        with concurrent.futures.ThreadPoolExecutor() as executor:
            future_to_manager = {executor.submit(pm.search, query): pm for pm in self.managers}
            for future in concurrent.futures.as_completed(future_to_manager):
                # pm = future_to_manager[future]
                try:
                    results = future.result()
                    all_results.extend(results)
                except Exception:
                    # Log error ideally
                    pass
                    
        return all_results

    def save_results(self, results: List[PackageResult]):
        """Saves search results to cache with assigned IDs."""
        cache_data = []
        for idx, result in enumerate(results, start=1):
            cache_data.append({
                "id": f"id{idx}",
                "manager": result.manager_name,
                "name": result.name,
                "version": result.version,
                "description": result.description,
                "installed": result.installed,
                "identifier": result.identifier
            })
            
        CACHE_FILE.parent.mkdir(parents=True, exist_ok=True)
        try:
            with open(CACHE_FILE, "w") as f:
                json.dump(cache_data, f, indent=2)
        except Exception as e:
            # Silently fail or log?
            pass

    def get_cached_result(self, result_id: str) -> Optional[Dict]:
        """Retrieves a specific result by ID from the cache."""
        if not CACHE_FILE.exists():
            return None
            
        try:
            with open(CACHE_FILE, "r") as f:
                data = json.load(f)
                for item in data:
                    if item["id"] == result_id:
                        return item
        except Exception:
            pass
        return None

    def install_by_id(self, result_id: str) -> bool:
        """Installs a package given its cached ID."""
        item = self.get_cached_result(result_id)
        if not item:
            print(f"Error: ID '{result_id}' not found in cache. Did you run 'genesis search'?")
            return False
            
        manager_name = item["manager"]
        # Find the manager instance
        target_manager = next((m for m in self.managers if m.name == manager_name), None)
        
        if not target_manager:
            print(f"Error: Package manager '{manager_name}' is not available.")
            return False
            
        # Reconstruct PackageResult
        pkg = PackageResult(
            manager_name=manager_name,
            name=item["name"],
            version=item["version"],
            description=item["description"],
            installed=item["installed"],
            identifier=item["identifier"]
        )
        
        print(f"Installing {pkg.name} via {target_manager.display_name}...")
        return target_manager.install(pkg)
