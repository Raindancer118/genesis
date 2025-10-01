import os
import subprocess
from datetime import datetime
from rich.console import Console
from rich.panel import Panel
from . import self_update  # Use relative import

console = Console()


def say_good_morning():
    user = os.getlogin().capitalize()
    now = datetime.now()

    greeting = "Good morning"
    if 12 <= now.hour < 18:
        greeting = "Good afternoon"
    elif now.hour >= 18:
        greeting = "Good evening"

    # --- Smart feature: Check for pending system updates ---
    update_message = "System is up to date. ✅"  # Default message
    try:
        # 'checkupdates' is a standard script on Arch/Manjaro
        updates_output = subprocess.check_output(['checkupdates'], text=True, stderr=subprocess.DEVNULL).strip()
        if updates_output:
            update_count = len(updates_output.split('\n'))
            update_message = f"You have [bold green]{update_count} system updates[/bold green] pending."
    except (FileNotFoundError, subprocess.CalledProcessError):
        update_message = "[yellow]Could not check for system updates.[/yellow]"

    # --- Smart feature: Check for Genesis self-updates ---
    genesis_update_available = self_update.check_for_updates()
    genesis_status = "💡 [bold yellow]Update available! Run 'genesis self-update'[/bold yellow]" if genesis_update_available else "Genesis is up to date. ✅"

    # --- CORRECTED LINE ---
    # The panel_text now uses the correct 'update_message' variable
    panel_text = f"{greeting}, [bold magenta]{user}[/bold magenta]! ☀️\n{update_message}\n{genesis_status}"

    console.print(Panel(panel_text, title="Welcome Back", border_style="cyan", expand=False))