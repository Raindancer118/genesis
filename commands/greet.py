import os
import subprocess
from datetime import datetime
from rich.console import Console
from rich.panel import Panel
from . import self_update


def say_good_morning():
    console = Console()
    user = os.getlogin()
    now = datetime.now()

    greeting = "Good morning"
    if 12 <= now.hour < 18:
        greeting = "Good afternoon"
    elif now.hour >= 18:
        greeting = "Good evening"

    # Smart feature: Check for pending updates
    try:
        updates = subprocess.check_output(['checkupdates'], text=True).strip().split('\n')
        update_count = len(updates) if updates[0] else 0
    except (FileNotFoundError, subprocess.CalledProcessError):
        update_count = 0

    update_message = f"You have [bold green]{update_count} updates[/bold green] pending." if update_count > 0 else "System is up to date. ✅"

    panel_text = f"{greeting}, [bold magenta]{user.capitalize()}[/bold magenta]! ☀️\n{update_message}"
    console.print(Panel(panel_text, title="Welcome Back", border_style="cyan", expand=False))

    # --- NEW: Check for self-update ---
    genesis_update_available = self_update.check_for_updates()
    genesis_status = "💡 [bold yellow]Update available! Run 'genesis self-update'[/bold yellow]" if genesis_update_available else "Genesis is up to date. ✅"

    panel_text = f"{greeting}, {user}!\n{system_update_message}\n{genesis_status}"
    console.print(Panel(panel_text, title="Welcome Back", border_style="cyan", expand=False))

