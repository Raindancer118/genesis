#!/usr/bin/env python3
import click
from commands import greet, project, sort, system, status, self_update
from rich.console import Console


@click.group()
def genesis():
    """Genesis is your personal command-line assistant for Manjaro.

    It helps with sorting files, creating projects, system maintenance,
    and more, all with a beautiful, modern interface.
    """
    pass

@genesis.command()
@click.argument('packages', nargs=-1, required=True) # Allow multiple packages
def install(packages):
    """Finds and installs package(s) using the best available manager."""
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
def greet():
    """Displays a custom morning greeting."""
    greet.say_good_morning()

@genesis.command()
def status():
    """Performs a comprehensive, AI-driven system health check."""
    status.run_health_check()

@genesis.command()
@click.argument('project_type')
@click.argument('name')
def new(project_type, name):
    """Initializes a new project from a predefined template."""
    project.create_project(project_type, name)


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
    sort.sort_directory(path)


@genesis.command()
@click.argument('path', type=click.Path(exists=True, resolve_path=True))
def scan(path):
    """Scans a directory for viruses with clamscan, showing progress."""
    system.scan_directory(path)


@genesis.command()
@click.argument('package', nargs=-1)
def install(package):
    """Finds and installs package(s) using pacman or pamac (AUR)."""
    if not package:
        click.echo("Please specify at least one package to install.")
        return
    system.install_packages(package)

@genesis.command(hidden=True) # hidden=True keeps it out of the --help menu
def monitor():
    """Background monitor task for the systemd service."""
    status.run_background_check()

@genesis.command()
def self_update():
    """Checks for and applies updates to Genesis itself."""
    console.print("🔎 Checking for updates to Genesis...")
    if self_update.check_for_updates():
        self_update.perform_update()
    else:
        console.print("✅ Genesis is already up to date.")


if __name__ == '__main__':
    genesis()