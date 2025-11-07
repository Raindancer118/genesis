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
            use_git = click.confirm("Initialize a Git repository?", default=True)
        
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
@click.argument('path', type=click.Path(exists=True, resolve_path=True))
def scan(path):
    """Scans a directory for viruses with clamscan, showing progress."""
    system.scan_directory(path)


@genesis.command()
@click.argument('packages', nargs=-1, required=True)
def install(packages):
    """Finds and installs package(s) using pacman or pamac (AUR)."""
    system.install_packages(packages)


@genesis.command()
@click.argument('packages', nargs=-1, required=True)
def remove(packages):
    """Finds and removes installed package(s)."""
    system.remove_packages(packages)


@genesis.command()
def update():
    """Performs a full system update (official repos + AUR)."""
    system.update_system()


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
@click.option('--dry-run', is_flag=True, help='Preview targets without killing them.')
@click.option('--scope', type=click.Choice(['user', 'all']), default='user', help='Process scope: user (default) or all.')
@click.option('--mem-threshold', type=float, default=400.0, help='Memory threshold in MB (default: 400).')
@click.option('--cpu-threshold', type=float, default=50.0, help='CPU threshold in percent (default: 50).')
@click.option('--limit', type=int, default=15, help='Maximum number of targets (default: 15).')
@click.option('--quiet', is_flag=True, help='Suppress verbose output.')
@click.option('--fast', is_flag=True, help='Skip CPU sampling for faster execution.')
def hero(dry_run, scope, mem_threshold, cpu_threshold, limit, quiet, fast):
    """Kill resource-intensive processes to free up system resources."""
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
