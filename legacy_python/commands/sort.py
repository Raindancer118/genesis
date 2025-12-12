# type: ignore
"""Interactive sorting workflow for Genesis."""

from __future__ import annotations

import shutil
import sys
import time
from collections import Counter
from pathlib import Path
from typing import Dict, Tuple

from .storage import storage

# --- Defensive TUI & Content Analysis Library Imports ---
try:
    from rich.console import Console
    from rich.panel import Panel
    from rich.progress import BarColumn, Progress, SpinnerColumn, TextColumn
except ImportError as error:  # pragma: no cover - rich is an optional runtime dep
    Console = None  # type: ignore[assignment]
    Panel = None  # type: ignore[assignment]
    Progress = None  # type: ignore[assignment]
    _IMPORT_ERROR = error
else:
    _IMPORT_ERROR = None

try:  # Optional helper libraries for content suggestions
    import questionary
    from questionary import Style
except ImportError:  # pragma: no cover - optional dependency
    questionary = None  # type: ignore[assignment]
    Style = None  # type: ignore[assignment]

try:  # Optional libraries for content analysis
    from PIL import Image
    import pypdf # noqa: F401
    import docx # noqa: F401
except ImportError:
    Image = None  # type: ignore[assignment]


# --- CONFIGURATION ---
MEMORY_NAMESPACE = "sorter_memory"

# Rules for files based on their extension
FILE_MAPPINGS = {
    ".jpg": "Images", ".jpeg": "Images", ".png": "Images", ".gif": "Images",
    ".bmp": "Images", ".webp": "Images", ".pdf": "Documents", ".doc": "Documents",
    ".docx": "Documents", ".txt": "Documents", ".md": "Documents", ".ppt": "Presentations",
    ".pptx": "Presentations", ".csv": "Spreadsheets", ".xls": "Spreadsheets",
    ".xlsx": "Spreadsheets", ".zip": "Archives", ".rar": "Archives",
    ".7z": "Archives", ".tar": "Archives",
}

# --- NEW: Rules for folders based on their name ---
FOLDER_MAPPINGS = {
    "Code": "Programming",
    "Obsidian": "Notes",
    "BlueJ": "Programming",
    "Sicherungen": "Backups",
    "AnalysisUndStochastik": "University",
    "IT Orga Folien": "University",
    "Fonts": "Assets/Fonts",
}

DEFAULT_CATEGORY = "Other"
DESTINATION_FOLDER = "Sorted_Output"


def sort_directory(path: str) -> None:
    """Sort the contents of *path* into categorised directories."""

    directory = Path(path).expanduser().resolve()

    if not directory.exists() or not directory.is_dir():
        print(f"ERROR: Path '{directory}' is not a valid directory.", file=sys.stderr)
        return

    if _IMPORT_ERROR is not None or Console is None:
        print(
            "ERROR: Optional dependency 'rich' is missing for the sorter TUI.",
            file=sys.stderr,
        )
        if _IMPORT_ERROR is not None:
            print(f"Missing module: {_IMPORT_ERROR}", file=sys.stderr)
        return

    console = Console()
    directory_key, memory_blob, directory_memory = _load_memory(directory)
    memory_changed = False

    # --- PHASE 1: DISCOVERY ---
    console.print(Panel.fit("[bold cyan]🔎 Phase 1: Discovery[/bold cyan]", border_style="cyan"))
    with console.status("[bold green]Scanning for unsorted items..."):
        items_to_process = _discover_items(directory)
        time.sleep(0.4)

    if not items_to_process:
        console.print("✅ No new files or folders to sort. Exiting.")
        return
    console.print(f"Found {len(items_to_process)} items to sort.")

    # --- PHASE 2: PREPARING DESTINATION ---
    console.print(Panel.fit("\n[bold cyan]🔎 Phase 2: Preparing Destination[/bold cyan]", border_style="cyan"))
    destination_root = directory / DESTINATION_FOLDER
    destination_root.mkdir(exist_ok=True)
    console.print(f"Destination is [bold magenta]'{destination_root.name}'[/bold magenta]")

    # --- PHASE 3: SORTING ---
    console.print(Panel.fit("\n[bold cyan]🚀 Phase 3: Sorting[/bold cyan]", border_style="cyan"))

    progress = Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        BarColumn(),
        TextColumn("[progress.percentage]{task.percentage:>3.0f}%"),
        console=console,
    )

    with progress:
        task_id = progress.add_task("[green]Sorting...", total=len(items_to_process))
        for item in items_to_process:
            progress.update(task_id, advance=1)
            time.sleep(0.05)

            if not item.exists() or item == destination_root:
                continue

            suggestion = _suggest_category(item)
            chosen_category, changed = _decide_category(
                item,
                directory_memory,
                suggestion=suggestion,
                interactive=_interactive_session(),
                pause_progress=lambda: progress.stop(),
                resume_progress=lambda: progress.start(),
            )
            if changed:
                memory_changed = True

            destination_dir = destination_root / chosen_category
            destination_dir.mkdir(parents=True, exist_ok=True)
            target_path = _resolve_collision(destination_dir, item.name)
            shutil.move(str(item), str(target_path))

    if memory_changed:
        console.print("\n[bold blue]🧠 Saving new memories...[/bold blue]")
        _save_memory(directory_key, memory_blob, directory_memory)

    console.print("\n[bold green]🎉 All done![/bold green]")


def _interactive_session() -> bool:
    return sys.stdin.isatty() and sys.stdout.isatty() and questionary is not None


def _discover_items(directory: Path) -> list[Path]:
    ignore_names = {DESTINATION_FOLDER}
    return [
        item
        for item in sorted(directory.iterdir(), key=lambda path: path.name.lower())
        if item.name not in ignore_names and not item.name.startswith("Sorted_")
    ]


def _decide_category(
    item: Path,
    memory: Dict[str, Dict[str, str]],
    *,
    suggestion: str | None,
    interactive: bool,
    pause_progress,
    resume_progress,
) -> Tuple[str, bool]:
    rules_key = "folder_rules" if item.is_dir() else "file_rules"
    memory_bucket = memory.setdefault(rules_key, {})
    # Use name for folders, suffix for files as the lookup key
    lookup_key = item.name.lower() if item.is_dir() else (item.suffix.lower() or item.name.lower())

    # 1. Check memory for a previously saved user choice
    if lookup_key in memory_bucket:
        return memory_bucket[lookup_key], False

    # 2. Apply hardcoded rules (FOLDER_MAPPINGS for dirs, FILE_MAPPINGS for files)
    category = None
    if item.is_dir():
        category = FOLDER_MAPPINGS.get(item.name)
    else:
        category = FILE_MAPPINGS.get(item.suffix.lower())

    # 3. Use content analysis for suggestions (e.g., screenshots)
    if suggestion and suggestion != category:
        if interactive:
            pause_progress()
            try:
                style = Style([("question", "bold yellow"), ("answer", "bold green")])
                use_suggestion = questionary.confirm(
                    f"My analysis suggests '{item.name}' is a '{suggestion}'. Use this category?",
                    default=True, style=style
                ).ask()
                if use_suggestion:
                    category = suggestion
            except (KeyboardInterrupt, TypeError):
                Console().print("[bold red]Skipping decision.[/bold red]")
            finally:
                resume_progress()
        else: # In non-interactive mode, trust the suggestion
            category = suggestion

    # 4. If no category found yet, prompt the user
    if category is None:
        if interactive:
            pause_progress()
            category = _prompt_for_category(item)
            resume_progress()
        else:
            category = DEFAULT_CATEGORY

    # 5. Save the final decision and return
    final_category = category or DEFAULT_CATEGORY
    memory_bucket[lookup_key] = final_category
    return final_category, True


def _prompt_for_category(item: Path) -> str:
    """Ask the user to choose a category for an item."""
    try:
        options = sorted({*FILE_MAPPINGS.values(), *FOLDER_MAPPINGS.values(), DEFAULT_CATEGORY})
        style = Style([("question", "bold cyan"), ("answer", "bold green")])
        choice = questionary.select(
            f"How should Genesis file '{item.name}'?",
            choices=options + ["Create new category"], style=style
        ).ask()

        if choice == "Create new category":
            new_category = questionary.text("Enter a custom category name:", style=style).ask()
            return (new_category or DEFAULT_CATEGORY).strip() or DEFAULT_CATEGORY
        return choice or DEFAULT_CATEGORY
    except (KeyboardInterrupt, TypeError):
        return DEFAULT_CATEGORY


def _suggest_category(item: Path) -> str | None:
    """Analyze file content to suggest a category."""
    if not item.is_file() or Image is None:
        return None

    if item.suffix.lower() in {".png", ".jpg", ".jpeg", ".webp"}:
        try:
            with Image.open(item) as img:
                width, height = img.size
            if width > 1200 and abs(width / height - (16 / 9)) < 0.1:
                return "Images/Screenshots"
        except Exception:
            return None
    return None


def _resolve_collision(destination_dir: Path, filename: str) -> Path:
    """Find a unique filename if the target path already exists."""
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


def _load_memory(directory: Path) -> Tuple[str, Dict, Dict]:
    blob = storage.get(MEMORY_NAMESPACE, {})
    directory_key = str(directory)
    raw = blob.get(directory_key, {}) or {}
    directory_memory = {
        "file_rules": dict(raw.get("file_rules", {})),
        "folder_rules": dict(raw.get("folder_rules", {})),
    }
    return directory_key, blob, directory_memory


def _save_memory(directory_key: str, blob: Dict, directory_memory: Dict) -> None:
    blob[directory_key] = directory_memory
    storage.set(MEMORY_NAMESPACE, blob)


__all__ = ["sort_directory"]