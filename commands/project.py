# In genesis/commands/project.py
import os
from pathlib import Path


def build_from_template(root_path, template_string):
    """Parses an indented string template and builds the file/dir structure."""
    if not template_string:
        print("Template is empty. Nothing to build.")
        return

    # Keep track of the current path based on indentation
    path_stack = [Path(root_path)]

    for line in template_string.splitlines():
        if not line.strip():
            continue

        # Calculate indentation level (assuming 4 spaces per level)
        indent_level = (len(line) - len(line.lstrip(' '))) // 4
        item_name = line.strip()

        # Adjust the current path based on the indentation
        while indent_level < len(path_stack) - 1:
            path_stack.pop()

        is_directory = item_name.endswith('/')
        if is_directory:
            item_name = item_name.rstrip('/')

        # Create the new path and the corresponding file/folder
        new_path = path_stack[-1] / item_name

        if is_directory:
            new_path.mkdir(parents=True, exist_ok=True)
            path_stack.append(new_path)
        else:
            new_path.touch()