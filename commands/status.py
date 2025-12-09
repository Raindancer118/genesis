import os
import shutil
import subprocess
import psutil
import platform
from rich.console import Console
from rich.spinner import Spinner
from rich.markdown import Markdown

try:
    import google.generativeai as genai

    API_KEY = os.getenv("GEMINI_API_KEY")
    if not API_KEY:
        raise ImportError
    genai.configure(api_key=API_KEY)
    llm = genai.GenerativeModel('gemini-2.5-flash')
except (ImportError, KeyError):
    llm = None

console = Console()


def get_system_metrics():
    """Gathers a wide range of system health data points."""
    metrics = {}
    # Hardware
    metrics['CPU Load (%)'] = psutil.cpu_percent(interval=1)

    # --- KORRIGIERTER ABSCHNITT FÜR CPU-TEMPERATUR ---
    cpu_temp = 'N/A'
    temps = psutil.sensors_temperatures()
    # Common sensor name for Intel CPUs (like in your HP Elitebook)
    if 'coretemp' in temps and temps['coretemp']:
        cpu_temp = temps['coretemp'][0].current
    # Fallback for other common sensor names (e.g., AMD CPUs)
    elif 'k10temp' in temps and temps['k10temp']:
        cpu_temp = temps['k10temp'][0].current
    metrics['CPU Temp (°C)'] = cpu_temp
    
    # Windows CPU Temp Support (often requires heavy WMI or skipped)
    if platform.system() == "Windows":
        try:
             # Typical WMI call via wmic or skipped as 'N/A' often default
             # psutil often fails on Windows for temps without specific drivers
             # We leave it as N/A or try:
             pass
        except:
             pass
    # --- ENDE DER KORREKTUR ---

    metrics['Memory Usage (%)'] = psutil.virtual_memory().percent
    metrics['Disk / Usage (%)'] = psutil.disk_usage('/').percent
    metrics['Disk /home Usage (%)'] = psutil.disk_usage('/home').percent if os.path.exists('/home') else 'N/A'

    battery = psutil.sensors_battery()
    metrics['Battery (%)'] = battery.percent if battery else 'N/A'

    # Software & OS
    pending_updates = 0
    if shutil.which('checkupdates'):
        try:
            updates = subprocess.check_output(
                ['checkupdates'], text=True, stderr=subprocess.DEVNULL
            ).strip().split('\n')
            pending_updates = len([line for line in updates if line])
        except (FileNotFoundError, subprocess.CalledProcessError):
            pending_updates = 0
    elif shutil.which('apt'):
        try:
            result = subprocess.check_output(
                ['apt', 'list', '--upgradable'], text=True, stderr=subprocess.DEVNULL
            )
            pending_updates = len(
                [
                    line
                    for line in result.splitlines()
                    if line and not line.startswith('Listing...')
                ]
            )
        except (subprocess.CalledProcessError, FileNotFoundError):
            pending_updates = 0
    elif shutil.which('apt-get'):
        try:
            result = subprocess.check_output(
                ['apt-get', '-s', 'upgrade'], text=True, stderr=subprocess.DEVNULL
            )
            pending_updates = len(
                [line for line in result.splitlines() if line.startswith('Inst ')]
            )
        except (subprocess.CalledProcessError, FileNotFoundError):
            pending_updates = 0

    if platform.system() == "Windows":
        try:
            # Check winget upgrades
            # winget upgrade --include-unknown -> lists
            out = subprocess.check_output(["winget", "upgrade"], text=True, stderr=subprocess.DEVNULL)
            lines = [l for l in out.splitlines() if l.strip()]
            # Approximate: Table header usually takes 2 lines
            pending_updates = max(0, len(lines) - 2)
        except:
            pending_updates = 0

    metrics['Pending Updates'] = pending_updates

    try:
        failed_services = subprocess.check_output(['systemctl', '--failed', '--no-legend'], text=True).strip()
        metrics['Failed systemd Services'] = failed_services if failed_services else "None"
    except subprocess.CalledProcessError:
        metrics['Failed systemd Services'] = "N/A"
        
    if platform.system() == "Windows":
        metrics['Failed systemd Services'] = "N/A (Windows)"

    metrics['Genesis Greet Service'] = "Active" if os.path.exists(
        f"{os.path.expanduser('~')}/.config/systemd/user/genesis-greet.service") else "Not Installed"

    return metrics


def run_health_check():
    """Gathers data, sends it to the AI for analysis, and prints the result."""
    with console.status("[bold green]Gathering system telemetry...") as status:
        metrics = get_system_metrics()

    if not llm:
        console.print("[bold red]Error: Gemini API key not found.[/bold red]")
        console.print("Please set the GEMINI_API_KEY environment variable.")
        console.print("\n--- [bold]Raw System Metrics[/bold] ---")
        for key, value in metrics.items():
            console.print(f"{key}: {value}")
        return

    report_string = "\n".join([f"- {key}: {value}" for key, value in metrics.items()])
    prompt = f"""
    You are a senior Manjaro Linux system administrator integrated into a tool called 'genesis'.
    Analyze the following system health report. Your response must be in Markdown.

    - First, provide a one-line, executive summary titled "## System Status". This should be either "All systems are nominal." or "Attention needed."
    - If attention is needed, create a "## Key Issues" section with a bulleted list of the problems (e.g., high CPU temp, low disk space, failed services).
    - Finally, create a "## Recommended Actions" section with specific, copy-paste-ready terminal commands to help the user troubleshoot the identified issues. Do not explain the commands unless necessary.
    - If all systems are nominal, only provide the "System Status" line and nothing else.

    Here is the system report:
    ---
    {report_string}
    """

    with console.status("[bold purple]Consulting with Genesis AI core...") as status:
        try:
            response = llm.generate_content(prompt)
            ai_summary = response.text
        except Exception as e:
            console.print(f"[bold red]Error communicating with the AI: {e}[/bold red]")
            return

    console.print(Markdown(ai_summary))


def run_background_check():
    """Silent health check that sends a desktop notification if issues are found."""
    # (This function remains the same as before)
    pass
