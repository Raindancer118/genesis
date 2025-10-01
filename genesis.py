#!/usr/bin/env python3
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
def new(project_type, name):
    """Initializes a new project using an interactive wizard."""
    project.run_project_wizard()

@genesis.command()
@click.argument('name')
def build(name):
    """Builds a project structure from a live text template."""
    template_string = click.edit(
        "# Enter your project blueprint below.\n"
        "# Use 4-space indentation for nesting.\n"
        "# End directory names with a forward slash '/'.\n"
    )
    if template_string is not None:
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
    if self_update_module.check_for_updates():
        self_update_module.perform_update()
    else:
        # You need a console object here or just use print
        print("✅ Genesis is already up to date.")


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