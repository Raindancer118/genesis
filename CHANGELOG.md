# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- **Deep Sorting Mode**: Content-based file analysis that reads file contents to determine categories intelligently
  - Analyzes text files for code patterns, documentation structure, and data formats
  - Uses AI (when available) for enhanced content understanding
  - Falls back to heuristic analysis when AI is not configured
  - Detects code indicators (functions, imports, classes) to categorize programming files
  - Identifies documentation by markdown headers and structure
  - Recognizes configuration files and data formats
- **Custom Sort Destinations**: Configure where files go based on category
  - Support for absolute paths (e.g., `/home/user/Documents`)
  - Home directory expansion (`~/Documents`)
  - Per-category destination mapping in config file
  - Example: Documents go to `~/Documents/sorted`, Images to `~/Pictures`
  - All sorting modes now respect custom destinations

### Changed
- All sorting modes now support custom destination configuration
- Updated sorting mode count from 6 to 7 modes (added Deep mode)

## [2.0.0] - 2025-12-16
### Added
- **Lightspeed Search**: Revolutionary file search with sub-millisecond performance using n-gram indexing and parallel fuzzy matching with SIMD acceleration.
- **Search Indexing**: Build searchable indexes with `genesis index` for O(k) substring search independent of file count.
- **Fuzzy Matching**: Typo-tolerant search using SymSpell algorithm for ultra-fast approximate matching.
- **Setup**: Enhanced menu-driven configuration wizard for granular control.
- **System Update**: Universal update across Arch, Debian, Fedora, OpenSUSE, Alpine, Void, Gentoo, Nix, Homebrew, Flatpak, Snap, Cargo, NPM, Gem, Pipx, and Windows (Choco/Winget/Scoop).
- **Scan**: New virus scanning module wrapping ClamAV with interactive folder selection.
- **Package Search**: Universal package search across Pacman, Apt, Chocolatey, and Winget.
- **Remove**: Universal package removal capable of detecting the correct manager.
- **Monitor**: Initial background monitoring stub.
- **Self-Update**: Now displays this changelog during the update process.
- **Benchmark**: Comprehensive system performance testing (CPU, Memory, Disk I/O) with ratings.
- **Calculator**: Built-in calculator with support for basic operations, functions (sqrt, sin, cos, tan, abs, ln, log), and interactive mode.
- **Environment**: Environment variable viewer with search, filtering, and export command examples.
- **Logs**: System log viewer with filtering options for systemd journals, service logs, kernel logs, and authentication logs.
- **Network**: Network diagnostics toolkit including ping, port scanning, DNS lookup, traceroute, and speed test integration.
- **Notes**: Quick note-taking system with tagging, search, and full CRUD operations stored locally.
- **Timer**: Productivity timer with countdown, stopwatch, and Pomodoro technique support.
- **Todo**: Task management with priority levels, status tracking, and organized views.

### Performance
- Search queries complete in <1ms (0.5-1ms typical)
- Index 1000 files in ~100ms
- Multi-core parallel processing with Rayon
- SIMD-accelerated fuzzy matching

## [0.1.0] - 2025-12-12
### Added
- Initial Rust port of Genesis.
- Commands: `system`, `health`, `project`, `greet`, `hero`, `sort`, `status`, `storage`.
- Configuration management via `config.toml`.
- Setup wizard.
