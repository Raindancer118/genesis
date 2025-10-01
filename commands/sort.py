"""Interactive sorting workflow for Genesis."""

from __future__ import annotations

import shutil
import sys
import time
from collections import Counter
from pathlib import Path
from typing import Dict, Tuple

from .storage import storage

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

try:
    from PIL import Image
except ImportError:  # pragma: no cover - optional dependency
    Image = None  # type: ignore[assignment]

# ``pypdf`` and ``docx`` are currently unused in the heuristics but may be
# leveraged in future enhancements. Import them defensively so users see a clear
# error the moment we attempt to expand the analyser.
try:  # pragma: no cover - optional dependency
    import pypdf  # noqa: F401
    import docx  # noqa: F401
except ImportError:
    pass

MEMORY_NAMESPACE = "sorter_memory"

FILE_MAPPINGS = {
    ".jpg": "Images",
    ".jpeg": "Images",
    ".png": "Images",
    ".gif": "Images",
    ".bmp": "Images",
    ".webp": "Images",
    ".pdf": "Documents",
    ".doc": "Documents",
    ".docx": "Documents",
    ".txt": "Documents",
    ".md": "Documents",
    ".ppt": "Presentations",
    ".pptx": "Presentations",
    ".csv": "Spreadsheets",
    ".xls": "Spreadsheets",
    ".xlsx": "Spreadsheets",
    ".zip": "Archives",
    ".rar": "Archives",
    ".7z": "Archives",
    ".tar": "Archives",
}

DEFAULT_CATEGORY = "Other"
DESTINATION_FOLDER = "Sorted_Output"


def sort_directory(path: str) -> None:
    """Sort the contents of *path* into categorised directories.

    The workflow mirrors the reference script requested by the user: it renders
    a three phase TUI, remembers decisions between runs, performs simple content
    analysis and gently guides the user through outstanding classification
    choices.
    """

    directory = Path(path).expanduser().resolve()

    if not directory.exists():
        print(f"Path '{directory}' does not exist.")
        return
    if not directory.is_dir():
        print(f"Path '{directory}' is not a directory.")
        return

    if _IMPORT_ERROR is not None or Console is None or Panel is None or Progress is None:
        print(
            "ERROR: Optional dependencies for the interactive sorter are missing. "
            "Install 'rich' and re-run the command.",
            file=sys.stderr,
        )
        if _IMPORT_ERROR is not None:
            print(f"Missing module: {_IMPORT_ERROR}", file=sys.stderr)
        return

    console = Console()
    directory_key, memory_blob, directory_memory = _load_memory(directory)
    memory_changed = False

    console.print(Panel.fit("[bold cyan]🔎 Phase 1: Discovery[/bold cyan]", border_style="cyan"))
    with console.status("[bold green]Scanning for unsorted items..."):
        items_to_process = _discover_items(directory)
        time.sleep(0.4)  # Provide a subtle pause for user feedback

    if not items_to_process:
        console.print("✅ No new files or folders to sort. Exiting.")
        return

    console.print(f"Found {len(items_to_process)} items to sort.")

    console.print(Panel.fit("\n[bold cyan]🔎 Phase 2: Preparing Destination[/bold cyan]", border_style="cyan"))
    destination_root = directory / DESTINATION_FOLDER
    destination_root.mkdir(exist_ok=True)
    console.print(f"Destination is [bold magenta]'%s'[/bold magenta]" % destination_root.name)

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
    lookup_key = (item.suffix.lower() or item.name.lower()) if rules_key == "file_rules" else item.name.lower()

    if lookup_key in memory_bucket:
        return memory_bucket[lookup_key], False

    category = FILE_MAPPINGS.get(item.suffix.lower()) if not item.is_dir() else None

    if suggestion and suggestion != category:
        if interactive:
            pause_progress()
            try:
                style = Style(
                    [
                        ("question", "bold yellow"),
                        ("answer", "bold green"),
                        ("pointer", "bold cyan"),
                    ]
                ) if Style is not None else None
                use_suggestion = questionary.confirm(  # type: ignore[operator]
                    f"My analysis suggests '{item.name}' is a '{suggestion}'. Use this category?",
                    default=True,
                    style=style,
                ).ask()
            except (KeyboardInterrupt, EOFError):
                use_suggestion = False
                console = Console() if Console is not None else None
                if console:
                    console.print("[bold red]Skipping decision.[/bold red]")
            finally:
                resume_progress()
            if use_suggestion:
                category = suggestion
                memory_bucket[lookup_key] = category
                return category, True
        else:
            category = suggestion

    if category is None:
        category = _prompt_for_category(item, interactive)

    if category:
        memory_bucket[lookup_key] = category
    else:
        category = DEFAULT_CATEGORY
    return category, True


def _prompt_for_category(item: Path, interactive: bool) -> str:
    if not interactive:
        return DEFAULT_CATEGORY

    options = sorted({*FILE_MAPPINGS.values(), DEFAULT_CATEGORY})
    try:
        style = Style(
            [
                ("question", "bold cyan"),
                ("answer", "bold green"),
                ("pointer", "bold magenta"),
            ]
        ) if Style is not None else None
        choice = questionary.select(  # type: ignore[operator]
            f"How should Genesis file '{item.name}'?",
            choices=options + ["Create new category"],
            style=style,
        ).ask()
    except (KeyboardInterrupt, EOFError):
        return DEFAULT_CATEGORY

    if choice == "Create new category":
        try:
            new_category = questionary.text(  # type: ignore[operator]
                "Enter a custom category name:",
                style=style,
            ).ask()
        except (KeyboardInterrupt, EOFError):
            return DEFAULT_CATEGORY
        return (new_category or DEFAULT_CATEGORY).strip() or DEFAULT_CATEGORY

    return choice or DEFAULT_CATEGORY


def _suggest_category(item: Path) -> str | None:
    if not item.is_file():
        return None

    if Image is not None and item.suffix.lower() in {".png", ".jpg", ".jpeg", ".webp"}:
        try:
            with Image.open(item) as img:
                width, height = img.size
        except Exception:  # pragma: no cover - image inspection errors
            return None
        if width > 0 and height > 0:
            aspect_ratio = width / height
            if width > 1200 and abs(aspect_ratio - (16 / 9)) < 0.1:
                return "Images/Screenshots"

    return None


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


def _load_memory(directory: Path) -> Tuple[str, Dict[str, Dict[str, Dict[str, str]]], Dict[str, Dict[str, str]]]:
    blob: Dict[str, Dict[str, Dict[str, str]]] = storage.get(MEMORY_NAMESPACE, {})
    directory_key = str(directory)
    raw = blob.get(directory_key, {}) or {}
    directory_memory = {
        "file_rules": dict(raw.get("file_rules", {})),
        "folder_rules": dict(raw.get("folder_rules", {})),
    }
    return directory_key, blob, directory_memory


def _save_memory(
    directory_key: str,
    blob: Dict[str, Dict[str, Dict[str, str]]],
    directory_memory: Dict[str, Dict[str, str]],
) -> None:
    blob[directory_key] = directory_memory
    storage.set(MEMORY_NAMESPACE, blob)


__all__ = ["sort_directory"]
