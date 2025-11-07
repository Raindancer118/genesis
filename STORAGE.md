# Genesis Storage Structure

Genesis uses a persistent storage directory structure **outside of the git repository** to ensure that user data, configuration, and state survive updates, reinstalls, and repository resets.

## Storage Locations

Genesis follows the XDG Base Directory specification where possible:

### Configuration Directory
- **Default**: `~/.config/genesis/`
- **Override**: Set `GENESIS_CONFIG` environment variable or `XDG_CONFIG_HOME` to customize
- **Contents**: 
  - `state.json` - Persistent application state and user preferences
  - User-specific configuration files

### Data Directory
- **Default**: `~/.local/share/genesis/`
- **Override**: Set `GENESIS_STORAGE` environment variable or `XDG_DATA_HOME` to customize
- **Contents**:
  - `cache/` - Temporary and cached data
  - `logs/` - Application log files
  - `data/` - Other persistent data files

## Installation

The storage directories are automatically created during installation by the `install.sh` script. If for any reason they weren't created during installation, they will be created automatically on first use.

## Repository vs Storage

**Repository** (`/opt/genesis/` by default):
- Contains only the Genesis program code
- Managed by git
- Updated via `genesis self-update`
- Should not contain any user data

**Storage** (`~/.config/genesis/` and `~/.local/share/genesis/`):
- Contains all user data and configuration
- Persists across updates
- Independent of the git repository
- Backed by the storage API in `commands/storage.py`

## For Developers

To use the storage API in your Genesis commands:

```python
from commands.storage import (
    storage,                        # Global storage instance
    get_storage_root,              # Get data directory path
    get_config_dir,                # Get config directory path
    initialize_storage_directories, # Create all directories
    STORAGE_CACHE_DIR,             # "cache" subdirectory name
    STORAGE_LOGS_DIR,              # "logs" subdirectory name
    STORAGE_DATA_DIR,              # "data" subdirectory name
)

# Store and retrieve data
storage.set("my_key", {"some": "data"})
data = storage.get("my_key")

# Get storage locations
data_dir = get_storage_root()  # e.g., ~/.local/share/genesis
config_dir = get_config_dir()  # e.g., ~/.config/genesis

# Store files in appropriate locations using constants
log_file = get_storage_root() / STORAGE_LOGS_DIR / "my_feature.log"
cache_file = get_storage_root() / STORAGE_CACHE_DIR / "temp_data.json"
data_file = get_storage_root() / STORAGE_DATA_DIR / "persistent.db"
```

## Benefits

1. **Survives Updates**: User data is never affected by `genesis self-update`
2. **Clean Repository**: The git repository contains only code, no data files
3. **User Control**: Users can easily backup, restore, or reset their configuration
4. **XDG Compliant**: Follows Linux filesystem standards
5. **Portable**: Easy to move Genesis data by copying the storage directories
