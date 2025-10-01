import os
import subprocess
from pathlib import Path
from rich.console import Console
from rich.panel import Panel
from rich.text import Text
import questionary

console = Console()


def _create_python_scaffold(path, use_git):
    """Creates the files and folders for a simple Python project."""
    (path / "src").mkdir()
    (path / "src" / "main.py").write_text(
        "def main():\n    print(\"Hello, World!\")\n\nif __name__ == \"__main__\":\n    main()\n")
    (path / "tests").mkdir()
    (path / "tests" / "test_main.py").touch()
    (path / ".gitignore").write_text("# Python standard\n__pycache__/\n*.pyc\n.env\n/venv/\n")
    (path / "README.md").write_text(f"# {path.name}\n\nA new Python project created by Genesis.")
    if use_git:
        subprocess.run(['git', 'init'], cwd=path, capture_output=True)
    console.print(f"  ✅ [green]Created basic Python structure in '{path.name}'[/green]")


# You can add more functions like _create_java_scaffold, etc. here

def run_project_wizard():
    """Guides the user through an interactive project creation wizard."""
    console.print(Panel.fit("[bold cyan]🚀 Genesis Project Builder[/bold cyan]", border_style="cyan"))

    # --- Frage 1: Projektname ---
    project_name = questionary.text(
        "What is the name of your project?",
        validate=lambda text: True if len(text) > 0 and not Path(
            text).exists() else "Name cannot be empty or already exist."
    ).ask()
    if not project_name: return console.print("[red]Cancelled.[/red]")

    # --- Frage 2: Projekt-Vorlage ---
    templates = {
        "Python (Simple)": _create_python_scaffold,
        # Add more templates here, e.g., "Java (Maven)": _create_java_scaffold
        "Empty Project": None
    }
    template_choice = questionary.select(
        "Choose a project template:",
        choices=templates.keys()
    ).ask()
    if not template_choice: return console.print("[red]Cancelled.[/red]")

    # --- Frage 3: Git Initialisierung ---
    use_git = questionary.confirm("Initialize a Git repository?", default=True).ask()
    if use_git is None: return console.print("[red]Cancelled.[/red]")

    # --- Zusammenfassung und Bestätigung ---
    summary = Text.assemble(
        ("Project Name: ", "bold"), (f"{project_name}\n"),
        ("Template:     ", "bold"), (f"{template_choice}\n"),
        ("Use Git:      ", "bold"), ("Yes" if use_git else "No")
    )
    console.print(Panel(summary, title="[yellow]Summary[/yellow]", expand=False))

    proceed = questionary.confirm("Proceed with creation?").ask()
    if not proceed: return console.print("[red]Cancelled.[/red]")

    # --- Erstellungsprozess ---
    console.print("\n[bold]Building project...[/bold]")
    project_path = Path(project_name)
    project_path.mkdir()

    scaffold_function = templates[template_choice]
    if scaffold_function:
        scaffold_function(project_path, use_git)
    else:
        console.print(f"  ✅ [green]Created empty directory '{project_name}'[/green]")

    console.print(f"\n[bold green]🎉 Project '{project_name}' created successfully![/bold green]")