
import os
import tomllib
from pathlib import Path
from typing import Any, Dict

class ConfigManager:
    """
    Manages loading, saving, and accessing system configurations.
    Stores config in ~/.config/genesis/config.toml.
    """
    
    DEFAULTS = {
        "general": {
            "language": "en"
        },
        "system": {
            "package_manager_priority": ["pamac", "paru", "yay", "pacman"],
            "default_install_confirm": True,
            "update_mirrors": True,
            "create_timeshift": True,
            "auto_confirm_update": False
        },
        "hero": {
            "cpu_threshold": 50.0,
            "mem_threshold_mb": 400.0,
            "default_scope": "user"
        },
        "project": {
            "default_author": "",
            "default_email": "",
            "default_license": "MIT",
            "use_git_init": True
        }
    }

    def __init__(self):
        self.config_dir = Path.home() / ".config" / "genesis"
        self.config_file = self.config_dir / "config.toml"
        self._config = self.DEFAULTS.copy()
        self.load()

    def load(self):
        """Loads configuration from file, falling back to defaults if missing."""
        if not self.config_file.exists():
            return

        try:
            with open(self.config_file, "rb") as f:
                loaded = tomllib.load(f)
                # Deep merge defaults with loaded data to ensure new keys exist
                self._deep_merge(self._config, loaded)
        except Exception as e:
            print(f"Warning: Failed to load config file: {e}. Using defaults.")

    def _deep_merge(self, base: Dict, update: Dict):
        """Recursively updates base dict with values from update dict."""
        for k, v in update.items():
            if k in base and isinstance(base[k], dict) and isinstance(v, dict):
                self._deep_merge(base[k], v)
            else:
                base[k] = v

    def save(self):
        """Saves current configuration to file."""
        self.config_dir.mkdir(parents=True, exist_ok=True)
        with open(self.config_file, "w") as f:
            f.write(self._to_toml(self._config))

    def _to_toml(self, data: Dict) -> str:
        """
        Simple TOML dumper. 
        Note: Python 3.11+ has tomllib for reading, but no writer.
        Since our config is simple, we write a basic serializer.
        """
        lines = []
        # Write sections
        for section, values in data.items():
            lines.append(f"[{section}]")
            for k, v in values.items():
                lines.append(f"{k} = {self._format_value(v)}")
            lines.append("") # Empty line between sections
        return "\n".join(lines)

    def _format_value(self, value: Any) -> str:
        if isinstance(value, bool):
            return "true" if value else "false"
        if isinstance(value, str):
            return f'"{value}"'
        if isinstance(value, list):
            # Formats list of strings usually
            items = [self._format_value(i) for i in value]
            return f"[{', '.join(items)}]"
        return str(value)

    def get(self, key_path: str, default: Any = None) -> Any:
        """
        Retrieves a config value using dot notation (e.g. 'system.auto_confirm').
        """
        keys = key_path.split(".")
        val = self._config
        try:
            for k in keys:
                val = val[k]
            return val
        except (KeyError, TypeError):
            return default

    def set(self, key_path: str, value: Any):
        """
        Sets a config value using dot notation.
        """
        keys = key_path.split(".")
        last_key = keys.pop()
        target = self._config
        for k in keys:
            target = target.setdefault(k, {})
        target[last_key] = value

    def reset(self):
        """Resets to default configuration."""
        self._config = self.DEFAULTS.copy()
        self.save()

# Global instance
config = ConfigManager()
