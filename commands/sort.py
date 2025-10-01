"""Directory sorting utilities for Genesis."""

from __future__ import annotations

import shutil
import sys
from collections import Counter
from pathlib import Path
from typing import Dict

from .storage import storage

CATEGORY_RULES: Dict[str, set[str]] = {
    "Documents": {".pdf", ".doc", ".docx", ".txt", ".md", ".odt", ".rtf"},
    "Spreadsheets": {".xls", ".xlsx", ".ods", ".csv"},
    "Presentations": {".ppt", ".pptx", ".key", ".odp"},
    "Images": {".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tiff", ".svg", ".webp"},
    "Audio": {".mp3", ".wav", ".flac", ".aac", ".ogg"},
    "Video": {".mp4", ".mkv", ".mov", ".avi", ".webm"},
    "Archives": {".zip", ".tar", ".gz", ".bz2", ".xz", ".rar", ".7z"},
    "Code": {
        ".py",
        ".js",
        ".ts",
        ".java",
        ".go",
        ".rs",
        ".c",
        ".cpp",
        ".h",
        ".hpp",
        ".json",
        ".yaml",
        ".yml",
        ".toml",
    },
    "Installers": {".deb", ".rpm", ".pkg.tar.zst", ".msi", ".exe", ".dmg"},
}

DEFAULT_CATEGORY = "Other"
PREFERENCE_KEY = "sort_preferences"


def sort_directory(path: str) -> None:
    """Sorts files into category sub-directories.

    Files are grouped by extension.  When Genesis encounters a new extension it
    asks the user which category to use (if the session is interactive) and
    persists the answer so future runs behave consistently even after
    self-update operations.
    """

    directory = Path(path).expanduser().resolve()
    if not directory.exists():
        print(f"Path '{directory}' does not exist.")
        return
    if not directory.is_dir():
        print(f"Path '{directory}' is not a directory.")
        return

    files = sorted(p for p in directory.iterdir() if p.is_file())
    if not files:
        print("Nothing to sort – the directory contains no files.")
        return

    stored_preferences: Dict[str, str] = storage.get(PREFERENCE_KEY, {})
    # keep a working copy so we can detect changes before writing back
    preferences = dict(stored_preferences)
    category_counter: Counter[str] = Counter()

    for file_path in files:
        category = _determine_category(file_path, preferences)
        destination_dir = directory / category
        destination_dir.mkdir(exist_ok=True)
        destination_path = _resolve_collision(destination_dir, file_path.name)
        shutil.move(str(file_path), str(destination_path))
        category_counter[category] += 1

    if preferences != stored_preferences:
        storage.set(PREFERENCE_KEY, preferences)

    _print_summary(directory, category_counter)


def _determine_category(file_path: Path, preferences: Dict[str, str]) -> str:
    extension = file_path.suffix.lower()
    name_key = f":name:{file_path.name.lower()}"
    if extension and extension in preferences:
        return preferences[extension]
    if not extension and name_key in preferences:
        return preferences[name_key]

    for category, extensions in CATEGORY_RULES.items():
        if extension in extensions:
            return category

    category = _prompt_for_category(file_path) if sys.stdin.isatty() else DEFAULT_CATEGORY
    key = extension if extension else name_key
    preferences[key] = category
    return category


def _prompt_for_category(file_path: Path) -> str:
    available = sorted({*CATEGORY_RULES.keys(), DEFAULT_CATEGORY})
    print(f"How should Genesis file '{file_path.name}'?")
    for idx, category in enumerate(available, start=1):
        print(f"  {idx}. {category}")
    print("  0. Enter a custom category")

    while True:
        choice = input("Select a category [default: Other]: ").strip()
        if not choice:
            return DEFAULT_CATEGORY
        if choice.isdigit():
            option = int(choice)
            if option == 0:
                break
            if 1 <= option <= len(available):
                return available[option - 1]
        else:
            # Allow typing the category name directly
            normalised = choice.title()
            if normalised in available:
                return normalised
        print("Invalid selection. Please try again.")

    custom = input("Enter a custom category name: ").strip()
    return custom or DEFAULT_CATEGORY


def _resolve_collision(destination_dir: Path, filename: str) -> Path:
    destination = destination_dir / filename
    if not destination.exists():
        return destination

    stem = Path(filename).stem
    suffix = Path(filename).suffix
    counter = 1
    while True:
        candidate = destination_dir / f"{stem} ({counter}){suffix}"
        if not candidate.exists():
            return candidate
        counter += 1


def _print_summary(directory: Path, counter: Counter[str]) -> None:
    print(f"Sorted files in '{directory}'.")
    for category, count in counter.most_common():
        label = "file" if count == 1 else "files"
        print(f"  • {category}: {count} {label}")


__all__ = ["sort_directory"]
