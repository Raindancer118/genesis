#!/usr/bin/env python3
"""Genesis CLI entry point."""

from __future__ import annotations

import site
import sys
from pathlib import Path

def _bootstrap_local_venv() -> None:
    """Ensure the bundled virtualenv is importable without activation."""

    install_dir = Path(__file__).resolve().parent
    venv_dir = install_dir / ".venv"
    if not venv_dir.exists():
        return

    if sys.platform == "win32":  # pragma: no cover - Windows is not primary target
        site_packages = venv_dir / "Lib" / "site-packages"
        if site_packages.exists():
            site.addsitedir(str(site_packages))
        return

    lib_dir = venv_dir / "lib"
    if not lib_dir.exists():
        return

    for path in lib_dir.iterdir():
        candidate = path / "site-packages"
        if candidate.exists():
            site.addsitedir(str(candidate))


_bootstrap_local_venv()

import click
# --- KORRIGIERT: Allen potenziellen Konflikten einen Alias geben ---
from commands import greet as greet_module
from commands import project
from commands import sort as sort_module
from commands import system
from commands import self_update as self_update_module
from commands import status as status_module
from commands import hero as hero_module
from commands import health as health_module
from commands import setup as setup_module
from commands.config import config
from commands.pkg_managers.controller import PackageManagerController
from rich.console import Console
from rich.table import Table


@click.group()
def genesis():
    """Genesis is your personal command-line assistant for Manjaro."""
    pass


@genesis.command()
def greet():
    """Displays a custom morning greeting."""
    # KORRIGIERT
    greet_module.say_good_morning()


@genesis.command()
def setup():
    """Opens the interactive configuration wizard."""
    setup_module.run_setup_wizard()


@genesis.command()
@click.option('--name', help='Project name. Prompts if omitted.')
@click.option(
    '--template',
    type=click.Choice(project.get_template_choices()),
    help='Project template to use. Prompts if omitted.'
)
@click.option(
    '--git/--no-git',
    'use_git',
    default=None,
    help='Initialize a Git repository. Prompts if omitted.'
)
@click.option(
    '--yes',
    is_flag=True,
    help='Skip the final confirmation step when enough details are provided.'
)
@click.option(
    '--structure',
    help='JSON string or file path defining the project structure. If provided, ignores --template.'
)
def new(name, template, use_git, yes, structure):
    """Initializes a new project using an interactive wizard or provided options.
    
    Can create projects from templates or from a custom JSON structure.
    
    JSON structure format:
        {
          "project-name": {
            "src": {
              "main.py": "print('Hello')",
              "utils": {}
            },
            "README.md": null
          }
        }
    """
    if structure:
        # Use JSON structure mode
        if not name:
            name = click.prompt("Project name", type=str)
        
        # Check if structure is a file path or JSON string
        # First check if it looks like JSON (starts with { or [)
        if structure.strip().startswith('{') or structure.strip().startswith('['):
            # Treat as JSON string
            structure_data = structure
        else:
            # Treat as file path
            from pathlib import Path
            structure_path = Path(structure)
            if structure_path.exists():
                structure_data = structure_path.read_text()
            else:
                click.echo(f"Error: File '{structure}' not found.")
                return
        
        # Determine use_git if not specified
        if use_git is None:
            default_git = config.get("project.use_git_init", True)
            use_git = click.confirm("Initialize a Git repository?", default=default_git)
        
        project.build_from_json(name, structure_data, use_git=use_git)
    else:
        # Use template mode (existing behavior)
        project.create_project(name=name, template_key=template, use_git=use_git, auto_confirm=yes)

@genesis.command()
@click.argument('name')
def build(name):
    """Builds a project structure from a live text template."""
    template_string = click.edit(
        "# Enter your project blueprint below.\n"
        "# Use 4-space indentation for nesting.\n"
        "# End directory names with a forward slash '/'.\n"
    )
    if template_string is None:
        click.echo("Build cancelled. No template was provided.")
        return

    project.build_from_template(name, template_string)


@genesis.command()
@click.option('--path', default='.', help='The directory to sort. Defaults to the current directory.')
def sort(path):
    """Sorts files in a directory using an intelligent, interactive engine."""
    # KORRIGIERT
    sort_module.sort_directory(path)


@genesis.command()
@click.argument('path', required=False)
def scan(path):
    """Scans for viruses. If no path is given, shows an interactive menu.
    
    You can also use 'genesis scan usb' to scan USB drives."""
    if path is None:
        # No argument given - show interactive menu
        system.interactive_scan_menu()
    elif path.lower() == "usb":
        # Special case for USB scanning
        system.scan_usb_drives()
    else:
        # Specific path given - validate and scan that path
        target = Path(path).resolve()
        if not target.exists():
            click.echo(f"Error: Path '{path}' does not exist.")
            raise click.Abort()
        system.scan_directory(str(target))


@genesis.command()
@click.argument('query')
def search(query):
    """Searches for packages across all installed package managers."""
    console = Console()
    with console.status(f"[bold green]Searching for '{query}'...[/bold green]"):
        controller = PackageManagerController()
        results = controller.search(query)
        controller.save_results(results)

    if not results:
        console.print(f"[yellow]No results found for '{query}'.[/yellow]")
        return

    table = Table(title=f"Search Results for '{query}'")
    table.add_column("ID", style="cyan", no_wrap=True)
    table.add_column("Source", style="magenta")
    table.add_column("Name", style="green")
    table.add_column("Version", style="yellow")
    table.add_column("State", style="blue")

    for i, res in enumerate(results, start=1):
        state = "Installed" if res.installed else ""
        table.add_row(f"id{i}", res.manager_name, res.name, res.version, state)

    console.print(table)
    console.print("[dim]Use 'genesis install id<N>' to install a specific package.[/dim]")


@genesis.command()
@click.argument('packages', nargs=-1, required=True)
def install(packages):
    """Finds and installs package(s) using pacman, pamac (AUR), winget, or choco.
    
    You can also use 'id<N>' to install a result from the last 'genesis search'.
    """
    # Check for ID format
    # If ANY package arg is an ID, we try to install by ID via controller.
    # Mixing IDs and names is tricky, but let's handle IDs specifically.
    
    # Separate IDs from regular names
    ids = [p for p in packages if p.lower().startswith('id') and p[2:].isdigit()]
    names = [p for p in packages if not (p.lower().startswith('id') and p[2:].isdigit())]
    
    if ids:
        controller = PackageManagerController()
        for pkg_id in ids:
            if not controller.install_by_id(pkg_id):
                 # If install failed or ID invalid, maybe user meant literal package named 'id4'?
                 # Highly unlikely but possible.
                 pass
    
    if names:
        system.install_packages(names)


@genesis.command()
@click.argument('packages', nargs=-1, required=True)
def remove(packages):
    """Finds and removes installed package(s)."""
    system.remove_packages(packages)


@genesis.command()
@click.option('-y', '--yes', is_flag=True, help='Automatically answer yes to all prompts.')
def update(yes):
    """Performs a full system update (official repos + AUR)."""
    system.update_system(affirmative=yes)


@genesis.command()
def self_update():
    """Checks for and applies updates to Genesis itself."""
    # KORRIGIERT
    self_update_module.run_self_update()


@genesis.command()
def status():
    """Performs a comprehensive, AI-driven system health check."""
    # KORRIGIERT
    status_module.run_health_check()


@genesis.command(hidden=True)
def monitor():
    """Background monitor task for the systemd service."""
    # KORRIGIERT
    status_module.run_background_check()


@genesis.command()
def health():
    """Performs a deep system integrity and quality audit."""
    health_module.run_health_check()


@genesis.command()
@click.option('--dry-run', is_flag=True, help='Preview targets without killing them.')
@click.option('--scope', type=click.Choice(['user', 'all']), default='user', help='Process scope: user (default) or all.')
@click.option('--mem-threshold', type=float, default=None, help='Memory threshold in MB (default: 400).')
@click.option('--cpu-threshold', type=float, default=None, help='CPU threshold in percent (default: 50).')
@click.option('--limit', type=int, default=15, help='Maximum number of targets (default: 15).')
@click.option('--quiet', is_flag=True, help='Suppress verbose output.')
@click.option('--fast', is_flag=True, help='Skip CPU sampling for faster execution.')
def hero(dry_run, scope, mem_threshold, cpu_threshold, limit, quiet, fast):
    """Kill resource-intensive processes to free up system resources."""
    # Resolve defaults from config if not provided
    if mem_threshold is None:
        mem_threshold = config.get("hero.mem_threshold_mb", 400.0)
    if cpu_threshold is None:
        cpu_threshold = config.get("hero.cpu_threshold", 50.0)
    
    hero_module.run(
        dry_run=dry_run,
        scope=scope,
        mem_threshold_mb=mem_threshold,
        cpu_threshold=cpu_threshold,
        limit=limit,
        verbose=not quiet,
        fast=fast,
    )


if __name__ == '__main__':
    genesis()
