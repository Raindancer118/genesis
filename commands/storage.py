"""Persistent storage helpers for Genesis."""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any, Dict


# Storage subdirectory names
STORAGE_CACHE_DIR = "cache"
STORAGE_LOGS_DIR = "logs"
STORAGE_DATA_DIR = "data"


def get_storage_root() -> Path:
    """Get the root storage directory for Genesis.
    
    Returns the directory where all Genesis persistent data is stored.
    This is always outside the git repository to survive updates and resets.
    
    The location is determined by:
    1. GENESIS_STORAGE environment variable (if set)
    2. XDG_DATA_HOME/genesis (if XDG_DATA_HOME is set)
    3. ~/.local/share/genesis (fallback)
    
    Returns:
        Path to the storage root directory
    """
    if env_storage := os.environ.get("GENESIS_STORAGE"):
        return Path(env_storage)
    
    if xdg_data := os.environ.get("XDG_DATA_HOME"):
        return Path(xdg_data) / "genesis"
    
    return Path.home() / ".local" / "share" / "genesis"


def get_config_dir() -> Path:
    """Get the configuration directory for Genesis.
    
    Returns the directory where Genesis configuration files are stored.
    
    The location is determined by:
    1. GENESIS_CONFIG environment variable (if set)
    2. XDG_CONFIG_HOME/genesis (if XDG_CONFIG_HOME is set)
    3. ~/.config/genesis (fallback)
    
    Returns:
        Path to the config directory
    """
    if env_config := os.environ.get("GENESIS_CONFIG"):
        return Path(env_config)
    
    if xdg_config := os.environ.get("XDG_CONFIG_HOME"):
        return Path(xdg_config) / "genesis"
    
    return Path.home() / ".config" / "genesis"


def initialize_storage_directories() -> None:
    """Create all necessary storage directories for Genesis.
    
    This function ensures that all storage directories exist and are properly
    initialized. It's safe to call multiple times.
    """
    # Create main storage directories
    storage_root = get_storage_root()
    config_dir = get_config_dir()
    
    directories = [
        storage_root,
        config_dir,
        storage_root / STORAGE_CACHE_DIR,
        storage_root / STORAGE_LOGS_DIR,
        storage_root / STORAGE_DATA_DIR,
    ]
    
    for directory in directories:
        directory.mkdir(parents=True, exist_ok=True)


class GenesisStorage:
    """Provides simple JSON-backed key/value persistence.

    Data is stored under the Genesis configuration directory (typically
    ``~/.config/genesis/state.json``) so it survives installer runs and
    repository resets. The helper keeps the in-memory representation in
    sync with the on-disk file and writes through on every change to
    minimise the chance of data loss if the process is interrupted.
    """

    def __init__(self, storage_path: Path | None = None) -> None:
        config_dir = get_config_dir()
        self._path = storage_path or (config_dir / "state.json")
        self._data: Dict[str, Any] = {}
        self._load()

    def _load(self) -> None:
        if not self._path.exists():
            return

        try:
            self._data = json.loads(self._path.read_text("utf-8"))
        except json.JSONDecodeError:
            # Corrupt data should not break the CLI.  Preserve the broken file
            # for manual inspection and continue with a clean slate.
            backup_path = self._path.with_suffix(".corrupt")
            self._path.replace(backup_path)
            self._data = {}

    def _write(self) -> None:
        self._path.parent.mkdir(parents=True, exist_ok=True)
        tmp_path = self._path.with_suffix(".tmp")
        tmp_path.write_text(json.dumps(self._data, indent=2, sort_keys=True), "utf-8")
        tmp_path.replace(self._path)

    def get(self, key: str, default: Any | None = None) -> Any:
        return json.loads(json.dumps(self._data.get(key, default)))

    def set(self, key: str, value: Any) -> None:
        self._data[key] = value
        self._write()

    def delete(self, key: str) -> None:
        if key in self._data:
            del self._data[key]
            self._write()

    def update_namespace(self, namespace: str, updates: Dict[str, Any]) -> None:
        bucket = self._data.setdefault(namespace, {})
        bucket.update(updates)
        self._write()


storage = GenesisStorage()

__all__ = [
    "storage",
    "GenesisStorage",
    "get_storage_root",
    "get_config_dir",
    "initialize_storage_directories",
    "STORAGE_CACHE_DIR",
    "STORAGE_LOGS_DIR",
    "STORAGE_DATA_DIR",
]
