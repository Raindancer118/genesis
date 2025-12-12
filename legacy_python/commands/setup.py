
import questionary
from rich.console import Console
from rich.panel import Panel
from rich.text import Text
from .config import config

console = Console()

def run_setup_wizard():
    """Starts the interactive configuration wizard."""
    console.clear()
    console.print(Panel.fit("[bold cyan]Genesis Configuration Wizard[/bold cyan]", border_style="cyan"))
    
    while True:
        choice = questionary.select(
            "Main Menu",
            choices=[
                "Configure System (Update, Install)",
                "Configure Hero (Resource Killer)",
                "Configure Project Defaults",
                "Reset All to Defaults",
                "Save & Exit",
                "Exit without Saving"
            ]
        ).ask()
        
        if choice == "Configure System (Update, Install)":
            _configure_system()
        elif choice == "Configure Hero (Resource Killer)":
            _configure_hero()
        elif choice == "Configure Project Defaults":
            _configure_project()
        elif choice == "Reset All to Defaults":
            if questionary.confirm("Are you sure you want to reset all settings to defaults?").ask():
                config.reset()
                console.print("[green]Configuration reset to defaults.[/green]")
        elif choice == "Save & Exit":
            config.save()
            console.print("[bold green]Configuration saved successfully.[/bold green]")
            break
        elif choice == "Exit without Saving":
            if questionary.confirm("Discard changes?").ask():
                console.print("[yellow]Exiting without saving changes.[/yellow]")
                break

def _configure_system():
    while True:
        choices = [
            f"Auto Confirm Updates: {config.get('system.auto_confirm_update')}",
            f"Update Mirrors: {config.get('system.update_mirrors')}",
            f"Create Timeshift Snapshot: {config.get('system.create_timeshift')}",
            f"Default Install Confirm: {config.get('system.default_install_confirm')}",
            "Back"
        ]
        
        choice = questionary.select("System Configuration", choices=choices).ask()
        
        if choice.startswith("Auto Confirm Updates"):
            val = questionary.confirm("Automatically confirm system updates?", default=config.get('system.auto_confirm_update')).ask()
            config.set('system.auto_confirm_update', val)
        elif choice.startswith("Update Mirrors"):
            val = questionary.confirm("Update mirrors before system upgrade?", default=config.get('system.update_mirrors')).ask()
            config.set('system.update_mirrors', val)
        elif choice.startswith("Create Timeshift"):
            val = questionary.confirm("Create Timeshift snapshot before update?", default=config.get('system.create_timeshift')).ask()
            config.set('system.create_timeshift', val)
        elif choice.startswith("Default Install Confirm"):
            val = questionary.confirm("Ask for confirmation before installing packages?", default=config.get('system.default_install_confirm')).ask()
            config.set('system.default_install_confirm', val)
        elif choice == "Back":
            break

def _configure_hero():
    while True:
        choices = [
            f"CPU Threshold: {config.get('hero.cpu_threshold')}%",
            f"Memory Threshold: {config.get('hero.mem_threshold_mb')} MB",
            f"Default Scope: {config.get('hero.default_scope')}",
            "Back"
        ]
        
        choice = questionary.select("Hero Configuration", choices=choices).ask()
        
        if choice.startswith("CPU Threshold"):
            val = questionary.text("CPU Threshold (%)", default=str(config.get('hero.cpu_threshold'))).ask()
            try:
                config.set('hero.cpu_threshold', float(val))
            except ValueError:
                console.print("[red]Invalid number.[/red]")
        elif choice.startswith("Memory Threshold"):
            val = questionary.text("Memory Threshold (MB)", default=str(config.get('hero.mem_threshold_mb'))).ask()
            try:
                config.set('hero.mem_threshold_mb', float(val))
            except ValueError:
                console.print("[red]Invalid number.[/red]")
        elif choice.startswith("Default Scope"):
            val = questionary.select("Default Scope", choices=["user", "all"], default=config.get('hero.default_scope')).ask()
            config.set('hero.default_scope', val)
        elif choice == "Back":
            break

def _configure_project():
    while True:
        choices = [
            f"Default Author: {config.get('project.default_author') or '(Not Set)'}",
            f"Default Email: {config.get('project.default_email') or '(Not Set)'}",
            f"Default License: {config.get('project.default_license')}",
            f"Init Git by Default: {config.get('project.use_git_init')}",
            "Back"
        ]
        
        choice = questionary.select("Project Defaults", choices=choices).ask()
        
        if choice.startswith("Default Author"):
            val = questionary.text("Default Author Name", default=config.get('project.default_author')).ask()
            config.set('project.default_author', val)
        elif choice.startswith("Default Email"):
            val = questionary.text("Default Email", default=config.get('project.default_email')).ask()
            config.set('project.default_email', val)
        elif choice.startswith("Default License"):
            val = questionary.select("Default License", choices=["MIT", "Apache-2.0", "GPL-3.0", "Unlicense", "None"], default=config.get('project.default_license')).ask()
            config.set('project.default_license', val)
        elif choice.startswith("Init Git"):
            val = questionary.confirm("Initialize Git repository by default?", default=config.get('project.use_git_init')).ask()
            config.set('project.use_git_init', val)
        elif choice == "Back":
            break
