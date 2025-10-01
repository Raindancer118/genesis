import click
from commands import greet, project, sort, system

@click.group()
def genesis():
    """Genesis is your personal command-line assistant."""
    pass

# --- Register commands from other files ---

@genesis.command()
def greet():
    """Displays a morning greeting."""
    greet.say_good_morning()

@genesis.command()
@click.argument('project_type')
@click.argument('name')
def new(project_type, name):
    """Initializes a new project scaffold."""
    project.create_project(project_type, name)

@genesis.command()
@click.option('--path', default='.', help='The directory to sort. Defaults to current directory.')
def sort(path):
    """Sorts files in a directory into categories."""
    sort.sort_directory(path)

@genesis.command()
@click.argument('path', type=click.Path(exists=True))
def scan(path):
    """Scans a directory for viruses with clamscan, showing progress."""
    system.scan_directory(path)

@genesis.command()
@click.argument('package')
def install(package):
    """Finds and installs a package using the best available manager."""
    system.install_package(package)

@genesis.command()
@click.argument('name')
def build(name):
    """Builds a project structure from a text template."""
    template_marker = "# Enter your project structure blueprint below.\n# Use indentation for nesting.\n# End directory names with a forward slash '/'.\n"

    template_string = click.edit(template_marker)

    if template_string is not None:
        print(f"Blueprint received. Building project '{name}'...")
        project.build_from_template(name, template_string)
        print("✅ Project structure built successfully.")
    else:
        print("Build cancelled.")

if __name__ == '__main__':
    genesis()
