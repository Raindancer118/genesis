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


TEMPLATES = {
    "Python (Simple)": _create_python_scaffold,
    # Add more templates here, e.g., "Java (Maven)": _create_java_scaffold
    "Empty Project": None,
}


def get_template_choices():
    """Return the available template names."""
    return list(TEMPLATES.keys())


def run_project_wizard():
    """Backwards compatible wrapper for the interactive project creator."""
    create_project()


def create_project(name=None, template_key=None, use_git=None, auto_confirm=False):
    """Create a new project, optionally using provided parameters instead of prompts."""
    console.print(Panel.fit("[bold cyan]🚀 Genesis Project Builder[/bold cyan]", border_style="cyan"))

    project_name = name
    if project_name:
        if Path(project_name).exists():
            console.print(f"[red]A project or directory named '{project_name}' already exists.[/red]")
            return
    else:
        project_name = questionary.text(
            "What is the name of your project?",
            validate=lambda text: True if len(text) > 0 and not Path(text).exists()
            else "Name cannot be empty or already exist."
        ).ask()
        if not project_name:
            console.print("[red]Cancelled.[/red]")
            return

    template_choice = template_key
    if template_choice:
        if template_choice not in TEMPLATES:
            console.print(
                f"[red]Unknown template '{template_choice}'. Available options: {', '.join(TEMPLATES)}[/red]"
            )
            return
    else:
        template_choice = questionary.select(
            "Choose a project template:",
            choices=get_template_choices()
        ).ask()
        if not template_choice:
            console.print("[red]Cancelled.[/red]")
            return

    if use_git is None:
        use_git = questionary.confirm("Initialize a Git repository?", default=True).ask()
        if use_git is None:
            console.print("[red]Cancelled.[/red]")
            return

    summary = Text.assemble(
        ("Project Name: ", "bold"), (f"{project_name}\n"),
        ("Template:     ", "bold"), (f"{template_choice}\n"),
        ("Use Git:      ", "bold"), ("Yes" if use_git else "No")
    )
    console.print(Panel(summary, title="[yellow]Summary[/yellow]", expand=False))

    proceed = auto_confirm or questionary.confirm("Proceed with creation?").ask()
    if not proceed:
        console.print("[red]Cancelled.[/red]")
        return

    console.print("\n[bold]Building project...[/bold]")
    project_path = Path(project_name)
    try:
        project_path.mkdir()
    except FileExistsError:
        console.print(f"[red]A directory named '{project_name}' already exists.[/red]")
        return

    scaffold_function = TEMPLATES[template_choice]
    if scaffold_function:
        scaffold_function(project_path, use_git)
    else:
        console.print(f"  ✅ [green]Created empty directory '{project_name}'[/green]")

    console.print(f"\n[bold green]🎉 Project '{project_name}' created successfully![/bold green]")


def build_from_template(project_name, template_string):
    """Build a project structure from an indented text template."""
    if not template_string:
        console.print("[red]No template provided. Nothing to build.[/red]")
        return

    project_path = Path(project_name)
    try:
        project_path.mkdir()
    except FileExistsError:
        console.print(f"[red]A directory named '{project_name}' already exists.[/red]")
        return

    stack = [project_path]
    for line_number, raw_line in enumerate(template_string.splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith('#'):
            continue

        indent = len(raw_line) - len(raw_line.lstrip(' '))
        if indent % 4 != 0:
            console.print(
                f"[red]Invalid indentation on line {line_number}: '{raw_line.strip()}'. Use multiples of four spaces.[/red]"
            )
            continue

        depth = indent // 4
        if depth >= len(stack):
            console.print(
                f"[red]Line {line_number} is indented too deeply without a parent directory: '{raw_line.strip()}'[/red]"
            )
            continue

        while len(stack) > depth + 1:
            stack.pop()

        parent = stack[depth]
        if stripped.endswith('/'):
            directory_name = stripped.rstrip('/').strip()
            target = parent / directory_name
            target.mkdir(exist_ok=True)
            stack.append(target)
        else:
            file_path = parent / stripped
            file_path.parent.mkdir(parents=True, exist_ok=True)
            if not file_path.exists():
                file_path.touch()

    console.print(f"\n[bold green]📁 Built template structure in '{project_name}'.[/bold green]")
