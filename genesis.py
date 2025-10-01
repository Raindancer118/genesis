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
def new(name, template, use_git, yes):
    """Initializes a new project using an interactive wizard or provided options."""
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


if __name__ == '__main__':
    genesis()
