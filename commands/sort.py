import os
import shutil
import sys
import time
from pathlib import Path

# (Import your content analysis libraries: pypdf, docx, Pillow)

# --- NEW: Self-Contained Styling Class (No Rich/questionary needed) ---
class Style:
    # ANSI escape codes for colors and styles
    PURPLE = '\033[95m'
    CYAN = '\033[96m'
    BLUE = '\033[94m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'
    END = '\033[0m' # Reset style

    # Unicode box-drawing characters
    TOP_LEFT = '┌'
    TOP_RIGHT = '┐'
    BOTTOM_LEFT = '└'
    BOTTOM_RIGHT = '┘'
    HORIZONTAL = '─'
    VERTICAL = '│'

def get_display_length(text):
    """Calculates the visual length of a string, accounting for wide emojis."""
    emoji_count = text.count('🔎') + text.count('🚀') + text.count('🧠')
    return len(text) + emoji_count # Add one extra space for each emoji found

def print_panel(text, color=Style.CYAN):
    """Prints text inside a styled, self-sizing, and correctly aligned panel."""
    clean_text = text.strip()
    # --- The Fix is Here ---
    content_width = get_display_length(clean_text) # Use our new helper instead of len()

    padding = 4
    bar_width = content_width + padding
    terminal_width = shutil.get_terminal_size().columns

    if bar_width >= terminal_width - 2:
        bar_width = terminal_width - 4

    top_border = f"{Style.TOP_LEFT}{Style.HORIZONTAL * bar_width}{Style.TOP_RIGHT}"
    bottom_border = f"{Style.BOTTOM_LEFT}{Style.HORIZONTAL * bar_width}{Style.BOTTOM_RIGHT}"

    # Manually construct the centered text line to ensure perfect alignment
    padding_total = bar_width - content_width
    left_pad = ' ' * (padding_total // 2)
    right_pad = ' ' * (padding_total - (padding_total // 2)) # Handles odd numbers
    text_line = f"{Style.VERTICAL}{left_pad}{clean_text}{right_pad}{Style.VERTICAL}"

    print(f"\n{color}{Style.BOLD}{top_border}{Style.END}")
    print(f"{color}{Style.BOLD}{text_line}{Style.END}")
    print(f"{color}{Style.BOLD}{bottom_border}{Style.END}")

def print_progress_bar(iteration, total, prefix='', suffix='', length=40, fill='█'):
    """Creates and prints a terminal progress bar."""
    percent = ("{0:.1f}").format(100 * (iteration / float(total)))
    filled_length = int(length * iteration // total)
    bar = fill * filled_length + '-' * (length - filled_length)

    # \r is a carriage return, it moves the cursor to the beginning of the line
    sys.stdout.write(f'\r{Style.GREEN}{prefix} |{bar}| {percent}% {suffix}{Style.END}')
    sys.stdout.flush() # Flush the buffer to make it visible immediately
    if iteration == total:
        sys.stdout.write('\n') # Move to next line on completion

# --- MAIN SCRIPT ---

def sort_downloads():
    # --- PHASE 1: DISCOVERY ---
    print_panel("🔎 Phase 1: Discovery")
    # (Discovery logic remains the same)
    items_to_process = [item for item in Path.home().joinpath("Downloads").iterdir() if item.is_file()] # Simplified for example
    print(f"Found {len(items_to_process)} items to sort.")

    # --- PHASE 2: PREPARE DESTINATION ---
    print_panel("🔎 Phase 2: Preparing Destination")
    target_dir = Path.home().joinpath("Downloads", "Sorted_Output")
    target_dir.mkdir(exist_ok=True)
    print(f"Destination is {Style.PURPLE}'{target_dir.name}'{Style.END}")

    # --- PHASE 3: SORTING ---
    print_panel("🚀 Phase 3: Sorting")

    total_items = len(items_to_process)
    print_progress_bar(0, total_items, prefix='Progress:', suffix='Complete', length=50)

    for i, item in enumerate(items_to_process):
        # Simulate work and update progress bar
        time.sleep(0.05)
        print_progress_bar(i + 1, total_items, prefix='Progress:', suffix='Complete', length=50)

        # --- Example of a styled interactive prompt ---
        if item.suffix == '.jpg': # Simulate an unsure case
            prompt = (
                f"\n{Style.YELLOW}{Style.BOLD}🧠 My analysis suggests '{item.name}' is a 'Screenshot'.{Style.END}\n"
                f"{Style.YELLOW}   Use this category? [Y/n]: {Style.END}"
            )
            # We add a newline before the input to not mess with the progress bar
            # choice = input(prompt).lower().strip()
            # if choice == 'y' or choice == '':
            #     # Logic to handle user's 'yes'
            #     pass

        # Move file logic would go here
        # shutil.move(item, target_dir / item.name)

    # Final status message
    print(f"\n{Style.BOLD}{Style.GREEN}🎉 All done!{Style.END}")


if __name__ == "__main__":
    sort_downloads()
