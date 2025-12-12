"""Persistent storage helpers for Genesis."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict


class GenesisStorage:
    """Provides simple JSON-backed key/value persistence.

    Data is stored under ``~/.config/genesis/state.json`` so it survives
    installer runs and repository resets.  The helper keeps the in-memory
    representation in sync with the on-disk file and writes through on every
    change to minimise the chance of data loss if the process is interrupted.
    """

    def __init__(self, storage_path: Path | None = None) -> None:
        config_dir = Path.home() / ".config" / "genesis"
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

__all__ = ["storage", "GenesisStorage"]
