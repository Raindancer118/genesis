#!/usr/bin/env python3
"""Initialize Genesis storage directories during installation."""

import sys
from pathlib import Path

# Add the install directory to the path
install_dir = Path(__file__).parent
sys.path.insert(0, str(install_dir))

try:
    from commands.storage import (
        initialize_storage_directories,
        get_storage_root,
        get_config_dir
    )
    
    # Initialize all storage directories
    initialize_storage_directories()
    
    # Report success
    print(f"✅ Storage initialized at: {get_storage_root()}")
    print(f"✅ Configuration at: {get_config_dir()}")
    
except Exception as e:
    print(f"⚠️  Warning: Could not initialize storage directories: {e}", file=sys.stderr)
    print("They will be created automatically on first use.", file=sys.stderr)
    sys.exit(1)
