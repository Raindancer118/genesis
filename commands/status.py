import os
import subprocess
import psutil
from rich.console import Console
from rich.spinner import Spinner
from rich.markdown import Markdown
import subprocess

# --- AI Integration ---
try:
    import google.generativeai as genai

    API_KEY = os.getenv("GEMINI_API_KEY")
    if not API_KEY:
        raise ImportError
    genai.configure(api_key=API_KEY)
    llm = genai.GenerativeModel('gemini-pro')
except (ImportError, KeyError):
    llm = None  # Set to None if API is not configured

console = Console()


def get_system_metrics():
    """Gathers a wide range of system health data points."""
    metrics = {}
    # Hardware
    metrics['CPU Load (%)'] = psutil.cpu_percent(interval=1)
    metrics['CPU Temp (°C)'] = psutil.sensors_temperatures().get('coretemp', [{}])[0].get('current', 'N/A')
    metrics['Memory Usage (%)'] = psutil.virtual_memory().percent
    metrics['Disk / Usage (%)'] = psutil.disk_usage('/').percent
    metrics['Disk /home Usage (%)'] = psutil.disk_usage('/home').percent if os.path.exists('/home') else 'N/A'
    metrics['Battery (%)'] = psutil.sensors_battery().percent if hasattr(psutil, "sensors_battery") else 'N/A'

    # Software & OS
    try:
        updates = subprocess.check_output(['checkupdates'], text=True, stderr=subprocess.DEVNULL).strip().split('\n')
        metrics['Pending Updates'] = len(updates) if updates[0] else 0
    except (FileNotFoundError, subprocess.CalledProcessError):
        metrics['Pending Updates'] = 0

    try:
        failed_services = subprocess.check_output(['systemctl', '--failed', '--no-legend'], text=True).strip()
        metrics['Failed systemd Services'] = failed_services if failed_services else "None"
    except subprocess.CalledProcessError:
        metrics['Failed systemd Services'] = "N/A"

    # Genesis Status
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
        # Print raw data as a fallback
        console.print("\n--- [bold]Raw System Metrics[/bold] ---")
        for key, value in metrics.items():
            console.print(f"{key}: {value}")
        return

    # Prepare data and prompt for the AI
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
        if not llm:
            return  # Cannot run without the AI

        metrics = get_system_metrics()
        report_string = "\n".join([f"- {key}: {value}" for key, value in metrics.items()])

        # A more direct prompt for the background task
        prompt = f"""
        Analyze the following system report.
        If there are any critical or warning-level issues (e.g., disk space > 90%, high CPU temp, failed services, pending security updates),
        respond with a single, concise sentence summarizing the most critical issue.
        If all systems are nominal, respond with the exact string "OK".

        Report:
        ---
        {report_string}
        """

        try:
            response = llm.generate_content(prompt)
            ai_summary = response.text.strip()
        except Exception:
            ai_summary = "OK"  # Fail silently if AI is unavailable

        if ai_summary != "OK":
            # Send a desktop notification!
            try:
                subprocess.run(
                    ['notify-send', '-u', 'critical', 'Genesis System Alert', ai_summary],
                    check=True
                )
            except FileNotFoundError:
                # notify-send not found, but we shouldn't crash the service
                pass