# type: ignore
"""Interactive sorting workflow for Genesis."""

from __future__ import annotations

import json
import shutil
import sys
import time
from collections import Counter
from datetime import datetime
from pathlib import Path
from typing import Dict, Tuple, List, Optional

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
UNDO_NAMESPACE = "sorter_undo"
LEARNING_NAMESPACE = "sorter_learning"

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


def sort_directory(path: str, strategy: str = "auto", undo: bool = False, learn_mode: bool = False, auto_confirm: bool = False) -> None:
    """Sort the contents of *path* into categorised directories.
    
    Args:
        path: Directory path to sort
        strategy: Sorting strategy (type, date, size, name, auto)
        undo: If True, revert the last sort operation
        learn_mode: If True, enter learning mode for manual categorization
        auto_confirm: If True, skip confirmation prompts
    """

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
    
    # Handle undo operation
    if undo:
        _undo_sort(directory, console, auto_confirm=auto_confirm)
        return
    
    # Handle learning mode
    if learn_mode:
        _learning_mode(directory, console)
        return
    
    # Display sorting strategy
    console.print(Panel.fit(
        f"[bold cyan]🎯 Sorting Strategy: {strategy.upper()}[/bold cyan]",
        border_style="cyan"
    ))
    
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
    
    # Initialize undo tracking
    undo_operations = []

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

            suggestion = _suggest_category(item, strategy)
            chosen_category, changed = _decide_category(
                item,
                directory,
                directory_memory,
                suggestion=suggestion,
                interactive=_interactive_session(),
                pause_progress=lambda: progress.stop(),
                resume_progress=lambda: progress.start(),
                strategy=strategy,
            )
            if changed:
                memory_changed = True

            destination_dir = destination_root / chosen_category
            destination_dir.mkdir(parents=True, exist_ok=True)
            target_path = _resolve_collision(destination_dir, item.name)
            
            # Track operation for undo
            undo_operations.append({
                "source": str(item),
                "destination": str(target_path),
                "timestamp": datetime.now().isoformat()
            })
            
            shutil.move(str(item), str(target_path))

    # Save undo information
    _save_undo_info(directory, undo_operations)
    
    if memory_changed:
        console.print("\n[bold blue]🧠 Saving new memories...[/bold blue]")
        _save_memory(directory_key, memory_blob, directory_memory)

    console.print("\n[bold green]🎉 All done![/bold green]")
    console.print("[dim]💡 Tip: Use 'genesis sort --path . --undo' to revert this sort operation.[/dim]")


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
    directory: Path,
    memory: Dict[str, Dict[str, str]],
    *,
    suggestion: str | None,
    interactive: bool,
    pause_progress,
    resume_progress,
    strategy: str = "auto",
) -> Tuple[str, bool]:
    rules_key = "folder_rules" if item.is_dir() else "file_rules"
    memory_bucket = memory.setdefault(rules_key, {})
    # Use name for folders, suffix for files as the lookup key
    lookup_key = item.name.lower() if item.is_dir() else (item.suffix.lower() or item.name.lower())

    # 1. Check learning data first (highest priority for auto strategy)
    if strategy == "auto":
        learning_data = storage.get(LEARNING_NAMESPACE, {})
        directory_key = str(directory)
        if directory_key in learning_data and item.name in learning_data[directory_key]:
            learned_category = learning_data[directory_key][item.name].get("category")
            if learned_category:
                memory_bucket[lookup_key] = learned_category
                return learned_category, True

    # 2. Check memory for a previously saved user choice (only for auto strategy)
    if strategy == "auto" and lookup_key in memory_bucket:
        return memory_bucket[lookup_key], False

    # 3. For non-auto strategies, use the suggestion if available
    if strategy != "auto" and suggestion:
        memory_bucket[lookup_key] = suggestion
        return suggestion, True

    # 4. Apply hardcoded rules (FOLDER_MAPPINGS for dirs, FILE_MAPPINGS for files)
    category = None
    if item.is_dir():
        category = FOLDER_MAPPINGS.get(item.name)
    else:
        category = FILE_MAPPINGS.get(item.suffix.lower())

    # 5. Use content analysis for suggestions (e.g., screenshots)
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

    # 6. If no category found yet, prompt the user
    if category is None:
        if interactive:
            pause_progress()
            category = _prompt_for_category(item)
            resume_progress()
        else:
            category = DEFAULT_CATEGORY

    # 7. Save the final decision and return
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


def _suggest_category(item: Path, strategy: str = "auto") -> str | None:
    """Analyze file content to suggest a category based on the strategy.
    
    Args:
        item: Path to the item to categorize
        strategy: Sorting strategy (type, date, size, name, auto)
    
    Returns:
        Suggested category or None
    """
    # Strategy-based categorization
    if strategy == "date":
        return _categorize_by_date(item)
    elif strategy == "size":
        return _categorize_by_size(item)
    elif strategy == "name":
        return _categorize_by_name(item)
    elif strategy == "type":
        # Type strategy uses existing FILE_MAPPINGS
        if item.is_file():
            return FILE_MAPPINGS.get(item.suffix.lower())
        return None
    
    # Auto strategy uses content analysis
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


def _categorize_by_date(item: Path) -> str:
    """Categorize item by modification date."""
    try:
        mtime = item.stat().st_mtime
        date = datetime.fromtimestamp(mtime)
        year = date.year
        month = date.strftime("%B")
        return f"By_Date/{year}/{month}"
    except Exception:
        return "By_Date/Unknown"


def _categorize_by_size(item: Path) -> str:
    """Categorize item by file size."""
    if not item.is_file():
        return "By_Size/Folders"
    
    try:
        size = item.stat().st_size
        if size < 1024 * 100:  # < 100KB
            return "By_Size/Small"
        elif size < 1024 * 1024 * 10:  # < 10MB
            return "By_Size/Medium"
        elif size < 1024 * 1024 * 100:  # < 100MB
            return "By_Size/Large"
        else:
            return "By_Size/Very_Large"
    except Exception:
        return "By_Size/Unknown"


def _categorize_by_name(item: Path) -> str:
    """Categorize item by first letter of name."""
    name = item.name
    if not name:
        return "By_Name/#"
    
    first_char = name[0].upper()
    if first_char.isalpha():
        return f"By_Name/{first_char}"
    elif first_char.isdigit():
        return "By_Name/0-9"
    else:
        return "By_Name/#"


def _undo_sort(directory: Path, console: Console, auto_confirm: bool = False) -> None:
    """Undo the last sort operation in the directory."""
    console.print(Panel.fit("[bold yellow]⏮️  Undo Last Sort[/bold yellow]", border_style="yellow"))
    
    directory_key = str(directory)
    undo_data = storage.get(UNDO_NAMESPACE, {})
    
    if directory_key not in undo_data or not undo_data[directory_key]:
        console.print("[bold red]❌ No recent sort operation found to undo.[/bold red]")
        return
    
    operations = undo_data[directory_key]
    console.print(f"Found {len(operations)} operations from last sort.")
    
    # Only ask for confirmation if not auto_confirm and in interactive mode
    if not auto_confirm and questionary and _interactive_session():
        try:
            style = Style([("question", "bold yellow"), ("answer", "bold green")])
            confirm = questionary.confirm(
                "Are you sure you want to revert these changes?",
                default=False,
                style=style
            ).ask()
            if not confirm:
                console.print("[dim]Undo cancelled.[/dim]")
                return
        except (KeyboardInterrupt, TypeError, Exception):
            console.print("[yellow]Proceeding with undo without confirmation.[/yellow]")
    
    console.print("[bold blue]Reverting changes...[/bold blue]")
    
    success_count = 0
    failed_count = 0
    
    for op in reversed(operations):  # Reverse order to undo properly
        try:
            source = Path(op["source"])
            destination = Path(op["destination"])
            
            if destination.exists():
                # Make sure source parent directory exists
                source.parent.mkdir(parents=True, exist_ok=True)
                
                # Handle collision at original location
                if source.exists():
                    # Find a new name for the file being moved back
                    source = _resolve_collision(source.parent, source.name)
                
                shutil.move(str(destination), str(source))
                success_count += 1
            else:
                console.print(f"[yellow]⚠️  Skipping: {destination.name} (not found)[/yellow]")
                failed_count += 1
        except Exception as e:
            console.print(f"[red]❌ Error reverting {op.get('source', '?')}: {e}[/red]")
            failed_count += 1
    
    # Clear undo history for this directory
    del undo_data[directory_key]
    storage.set(UNDO_NAMESPACE, undo_data)
    
    console.print(f"\n[bold green]✅ Reverted {success_count} operations.[/bold green]")
    if failed_count > 0:
        console.print(f"[yellow]⚠️  {failed_count} operations could not be reverted.[/yellow]")


def _learning_mode(directory: Path, console: Console) -> None:
    """Enter learning mode where user manually categorizes items."""
    console.print(Panel.fit(
        "[bold cyan]🎓 Learning Mode[/bold cyan]\n"
        "[dim]Manually categorize items to teach Genesis your preferences.[/dim]",
        border_style="cyan"
    ))
    
    if not _interactive_session():
        console.print("[bold red]❌ Learning mode requires an interactive terminal.[/bold red]")
        return
    
    with console.status("[bold green]Scanning for unsorted items..."):
        items_to_process = _discover_items(directory)
    
    if not items_to_process:
        console.print("✅ No items to categorize. Exiting.")
        return
    
    console.print(f"Found {len(items_to_process)} items to categorize.")
    
    # Load existing learning data
    learning_data = storage.get(LEARNING_NAMESPACE, {})
    directory_key = str(directory)
    
    # Get existing categories from learning data or use defaults
    all_categories = set()
    if directory_key in learning_data:
        for item_data in learning_data[directory_key].values():
            all_categories.add(item_data.get("category", ""))
    
    # Add default categories
    all_categories.update(FILE_MAPPINGS.values())
    all_categories.update(FOLDER_MAPPINGS.values())
    all_categories.add(DEFAULT_CATEGORY)
    all_categories = sorted([cat for cat in all_categories if cat])
    
    style = Style([("question", "bold cyan"), ("answer", "bold green")])
    categorized_count = 0
    
    for item in items_to_process:
        console.print(f"\n[bold]Item:[/bold] {item.name}")
        console.print(f"[dim]Type:[/dim] {'Directory' if item.is_dir() else 'File'}")
        
        if item.is_file():
            try:
                size = item.stat().st_size
                console.print(f"[dim]Size:[/dim] {_format_size(size)}")
            except:
                pass
        
        try:
            choice = questionary.select(
                "Select a category:",
                choices=all_categories + ["[Create new category]", "[Skip this item]"],
                style=style
            ).ask()
            
            if choice == "[Skip this item]":
                continue
            elif choice == "[Create new category]":
                new_category = questionary.text(
                    "Enter new category name:",
                    style=style
                ).ask()
                if new_category:
                    choice = new_category.strip()
                    all_categories.append(choice)
                    all_categories.sort()
                else:
                    continue
            
            # Save the learning data
            if directory_key not in learning_data:
                learning_data[directory_key] = {}
            
            learning_data[directory_key][item.name] = {
                "category": choice,
                "type": "directory" if item.is_dir() else "file",
                "timestamp": datetime.now().isoformat()
            }
            
            categorized_count += 1
            
        except (KeyboardInterrupt, TypeError):
            console.print("\n[yellow]Learning mode interrupted.[/yellow]")
            break
    
    # Save learning data
    storage.set(LEARNING_NAMESPACE, learning_data)
    
    console.print(f"\n[bold green]🎉 Categorized {categorized_count} items![/bold green]")
    console.print("[dim]💡 These preferences will be used in future sort operations.[/dim]")


def _format_size(size: int) -> str:
    """Format file size in human-readable format."""
    for unit in ['B', 'KB', 'MB', 'GB', 'TB']:
        if size < 1024.0:
            return f"{size:.1f} {unit}"
        size /= 1024.0
    return f"{size:.1f} PB"


def _save_undo_info(directory: Path, operations: List[Dict]) -> None:
    """Save undo information for the sort operation."""
    directory_key = str(directory)
    undo_data = storage.get(UNDO_NAMESPACE, {})
    undo_data[directory_key] = operations
    storage.set(UNDO_NAMESPACE, undo_data)


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