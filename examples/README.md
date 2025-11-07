# Genesis Project Structure Examples

This directory contains example JSON structures that can be used with the `genesis new` command to create custom project scaffolds.

## Usage

### Using a JSON file

```bash
genesis new --name my-project --structure examples/project-structure-example.json
```

### Using a JSON string directly

```bash
genesis new --name my-project --structure '{"my-project": {"src": {"main.py": "print(\"Hello\")"}, "README.md": "# Project"}}'
```

## JSON Structure Format

The JSON structure uses a simple format:
- **Directories**: Represented as keys with dictionary values (can be empty `{}`)
- **Files with content**: Represented as keys with string values containing the file content
- **Empty files**: Represented as keys with `null` values

### Example Structure

```json
{
  "project-name": {
    "directory": {
      "subdirectory": {},
      "file-with-content.txt": "This is the file content",
      "empty-file.txt": null
    },
    "README.md": "# Project Title\n\nProject description"
  }
}
```

### Features

1. **Nested directories**: Create deep directory structures
2. **File contents**: Include file content directly in the JSON
3. **Empty files**: Use `null` to create empty files
4. **Empty directories**: Use `{}` to create empty directories

## Available Templates

The `genesis new` command also supports predefined templates:

- `Python (Simple)` - Basic Python project
- `Python (Package)` - Distributable Python package
- `Node.js (Basic)` - Node.js application
- `TypeScript (Node)` - TypeScript Node.js project
- `Go (CLI)` - Go command-line application
- `Rust (CLI)` - Rust binary crate
- `Java (Basic)` - Simple Java project
- `Java (Maven)` - Maven-based Java project
- `Java (Spring Boot)` - Spring Boot application
- `C++ (Basic)` - C++ project with CMake
- `C# (.NET)` - C# .NET console application
- `Ruby` - Ruby project structure
- `PHP` - PHP web application
- `Empty Project` - Just creates an empty directory

### Using Templates

```bash
genesis new --name my-project --template "Python (Simple)"
```

## Examples in this directory

- `project-structure-example.json` - A comprehensive Java project structure with multiple packages, controllers, models, and services
