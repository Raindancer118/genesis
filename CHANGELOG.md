# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]
### Added
- **Setup**: Enhanced menu-driven configuration wizard for granular control.
- **System Update**: Universal update across Arch, Debian, Fedora, OpenSUSE, Alpine, Void, Gentoo, Nix, Homebrew, Flatpak, Snap, Cargo, NPM, Gem, Pipx, and Windows (Choco/Winget/Scoop).
- **Scan**: New virus scanning module wrapping ClamAV with interactive folder selection.
- **Search**: Universal package search across Pacman, Apt, Chocolatey, and Winget.
- **Remove**: Universal package removal capable of detecting the correct manager.
- **Monitor**: initial background monitoring stub.
- **Self-Update**: Now displays this changelog during the update process.

## [0.1.0] - 2025-12-12
### Added
- Initial Rust port of Genesis.
- Commands: `system`, `health`, `project`, `greet`, `hero`, `sort`, `status`, `storage`.
- Configuration management via `config.toml`.
- Setup wizard.
