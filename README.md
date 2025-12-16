# ⚡ Genesis - Lightning-Fast CLI Tool

<div align="center">

**The next-generation command-line tool that supercharges your workflow**

[![Version](https://img.shields.io/badge/version-2.0.0--lightspeed-blue.svg)](https://github.com/Raindancer118/genesis)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

[Features](#-features) • [Installation](#-installation) • [Quick Start](#-quick-start) • [Documentation](#-documentation)

</div>

---

## 🎯 What is Genesis?

Genesis is a **blazingly fast**, **intelligent**, and **versatile** CLI tool built with Rust that brings together powerful features for modern developers and power users:

- ⚡ **Lightspeed Search**: Find files in milliseconds with typo-tolerant fuzzy matching
- 🔧 **Universal Package Management**: One interface for all package managers (apt, pacman, brew, chocolatey, and more)
- 📊 **System Intelligence**: Monitor health, manage resources, and optimize performance
- 🚀 **Developer Tools**: Project scaffolding, git integration, and automation
- 🎨 **Beautiful UI**: Colorful, intuitive terminal interface with interactive menus

## ✨ Features

### ⚡ Lightspeed Search (NEW!)

Revolutionary file search that's **faster than anything you've used before**:

```bash
# Index your filesystem once
genesis index

# Search with lightning speed (<1ms!)
genesis search myfile

# Typo-tolerant! Finds "monitor.rs" even when you type:
genesis search monitr

# Substring search - finds "Bauhaus" when searching for "Haus"
genesis search config
```

**How it works:**
- 🧠 **N-gram indexing** for O(k) substring search (independent of file count!)
- 🔍 **Parallel fuzzy matching** with SIMD acceleration across CPU cores
- 🎯 **SymSpell algorithm** for ultra-fast approximate matching
- 📈 **Sub-millisecond search times** - typically 0.5-1ms

### 🔧 Universal Package Management

Stop memorizing different package manager commands! Genesis supports them all:

```bash
# Works on ANY platform - detects your package manager automatically
genesis install python3 nodejs rust

# Update everything at once
genesis update

# Search across all available package managers
genesis search docker

# Remove packages
genesis remove package-name
```

**Supported package managers:**
- Linux: `apt`, `pacman`, `yay`, `paru`, `dnf`, `zypper`, `apk`, `xbps`, `emerge`
- macOS: `brew`, `nix`
- Windows: `chocolatey`, `winget`, `scoop`
- Universal: `flatpak`, `snap`, `cargo`, `npm`, `pip`, `pipx`, `gem`

### 🛡️ System Management

Keep your system healthy and optimized:

```bash
# Kill resource-hungry processes
genesis hero

# Check system health
genesis health

# View system information
genesis info

# Monitor disk usage
genesis storage

# Real-time system monitoring
genesis monitor
```

### 🚀 Developer Productivity

Accelerate your development workflow:

```bash
# Create new projects with templates
genesis new myproject --template rust

# Organize files intelligently with AI
genesis sort ./downloads

# Quick calculations
genesis calc "2^16 * 3"

# Manage environment variables
genesis env

# View system logs
genesis logs

# Network diagnostics
genesis network
```

#### 🤖 AI-Powered File Sorting

Genesis now features **7 intelligent sorting modes** with optional Gemini AI integration:

```bash
# Sort files in current directory
genesis sort .

# Available modes:
# 1. Manual Learning - You categorize, system learns silently
# 2. Assisted Learning - System suggests, you correct
# 3. Smart - Uses your learned patterns automatically
# 4. Deep - Content-based analysis with AI/heuristics 🔍
# 5. AI-Assisted Learning - System suggests, AI corrects/validates 🤖
# 6. AI Learning - AI suggests, you teach ⚡
# 7. AI Sorting - Fully automatic AI categorization 🚀
```

**AI Features:**
- 🎯 Intelligent categorization using Gemini 2.0 Flash
- 📸 Automatic screenshot detection
- 🧠 Learns from your corrections and AI validations
- 💬 AI explains corrections when disagreeing with system
- 🔄 Switch from AI Learning to smart mode mid-session
- 📚 AI validates system suggestions in AI-Assisted Learning

**Deep Sorting Mode:**
- 🔍 Analyzes file contents to understand what they are
- 📝 Detects code patterns, documentation structure, and data formats
- 🤖 Uses AI (when available) for enhanced content analysis
- 🎯 Falls back to heuristic analysis when AI is not configured

**Custom Destinations:**
- 📂 Configure where files go based on category
- 🏠 Support for absolute paths (e.g., `/home/user/Documents`) and home directory expansion (`~/Documents`)
- ⚙️ Configure in `~/.config/genesis/config.toml` under `[sort.custom_destinations]`

Example configuration:
```toml
[sort]
enable_deep_sorting = false

[sort.custom_destinations]
Documents = "~/Documents/sorted"
Images = "~/Pictures"
Code = "~/Projects"
```

### 📝 Productivity Tools

Built-in tools for daily tasks:

```bash
# Quick notes
genesis notes

# Todo management
genesis todo

# Timer and stopwatch
genesis timer

# System benchmark
genesis benchmark
```

## 🚀 Installation

### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/Raindancer118/genesis.git
cd genesis

# Build and install
cargo build --release
sudo cp target/release/genesis /usr/local/bin/

# Or use the install script
./install.sh
```

### Quick Install Script

```bash
curl -sSf https://raw.githubusercontent.com/Raindancer118/genesis/main/install.sh | sh
```

## 🎓 Quick Start

### 1. Setup

Configure Genesis interactively:

```bash
genesis setup
```

This opens an interactive menu where you can configure:
- Package manager preferences
- Search settings (Lightspeed mode, fuzzy threshold)
- Project defaults
- System behavior
- **Gemini API key for AI-assisted sorting** ⚡

#### Setting up Gemini API (Optional - for AI features)

To enable AI-assisted file sorting:

1. **Get your API key:**
   - Visit: https://makersuite.google.com/app/apikey
   - Sign in with your Google account
   - Click 'Create API Key'
   - Copy the generated key

2. **Configure the key:**
   ```bash
   # Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)
   export GEMINI_API_KEY='your-api-key-here'
   
   # Then reload your profile
   source ~/.bashrc
   ```

3. **Verify it works:**
   ```bash
   genesis sort .
   # You should now see AI-powered sorting options
   ```

### 2. Build Your Search Index

Enable lightning-fast file search:

```bash
# Index current directory
genesis index

# Index specific paths
genesis index --paths ~/Documents ~/Projects

# View index information
genesis index --info
```

### 3. Search Like Never Before

```bash
# Basic search
genesis search config

# Works with typos!
genesis search cnfig

# Substring matching
genesis search .rs
```

### 4. Manage Packages

```bash
# Install packages (auto-detects your package manager)
genesis install vim neovim

# Update all packages
genesis update --yes

# Search for packages
genesis search package firefox
```

### 5. Monitor Your System

```bash
# Check system health
genesis health

# Kill resource hogs
genesis hero

# View disk usage
genesis storage
```

## 📖 Documentation

### Search Configuration

Fine-tune Lightspeed search in `~/.config/genesis/config.toml`:

```toml
[search]
lightspeed_mode = true      # Enable Lightspeed (default: true)
fuzzy_threshold = 2          # Edit distance for fuzzy matching (0-3)
max_depth = 10              # Directory traversal depth
max_results = 50            # Maximum results to display
show_details = false        # Show file size and modification time
exclude_hidden = true       # Skip hidden files/directories

# Paths to index by default
default_paths = ["/home/user/Documents"]

# Patterns to ignore during indexing
ignore_patterns = [
    "node_modules",
    ".git",
    "target",
    ".cache",
    "__pycache__"
]
```

### Command Reference

| Command | Description |
|---------|-------------|
| `genesis index` | Build search index |
| `genesis search <query>` | Search files with Lightspeed |
| `genesis install <pkg>` | Install package(s) |
| `genesis update` | Update all packages |
| `genesis hero` | Kill resource-intensive processes |
| `genesis health` | System health check |
| `genesis new <name>` | Create new project |
| `genesis sort <path>` | Organize files with 7 intelligent modes |
| `genesis setup` | Interactive configuration (includes Gemini API) |

### File Sorting Modes

Genesis provides **7 intelligent sorting modes**:

| Mode | Description | Learning | AI Required |
|------|-------------|----------|-------------|
| **Manual Learning** | You categorize each file manually | ✅ System learns | ❌ No |
| **Assisted Learning** | System suggests based on rules, you correct | ✅ System learns | ❌ No |
| **Smart** | Automatically uses your learned patterns | Uses learned data | ❌ No |
| **Deep** | Content-based analysis (AI + heuristics) | Analyzes content | 🔶 Optional |
| **AI-Assisted Learning** | System suggests, AI validates/corrects | ✅ AI corrects | ✅ Yes |
| **AI Learning** | AI suggests, you teach and correct | ✅ Both learn | ✅ Yes |
| **AI Sorting** | Fully automatic AI categorization | AI categorizes | ✅ Yes |

**Additional features:**
- 📸 Automatic screenshot detection (detects 16:9, 16:10, 21:9 aspect ratios)
- 🔄 Switch from AI Learning to smart mode mid-session
- 💬 AI explains corrections when disagreeing with system
- 🤖 AI validates system suggestions in AI-Assisted Learning mode
- 🧠 Persistent learning across sessions
- ↩️ Undo last operation within 5 minutes
- 🔍 Deep mode analyzes file contents for better categorization
- 📂 Custom destinations for each category (configure in config.toml)

See `genesis --help` for the complete command list.

## 🎨 Screenshots

### Lightspeed Search in Action
```
⚡ Lightspeed search for 'config'...

3 results found in 0.54ms:

1. ./src/config.rs [✓✓✓]
   Size: 5.1 KB | Modified: 2025-12-16 01:17:07 | Score: 95

2. ./legacy_python/commands/config.py [✓✓]
   Size: 3.9 KB | Modified: 2025-12-16 01:13:09 | Score: 72

Index last updated: 2025-12-16 01:36:34 | Search time: 0.54ms
```

### Interactive Setup Menu
```
🛠️  Genesis Configuration
? Main Menu:
> General Settings
  System Settings
  Project Settings
  Search Settings ⚡
  Save & Exit
  Discard & Exit
```

## 🔥 Performance

Genesis is built with performance in mind:

| Operation | Time |
|-----------|------|
| Search query | **<1ms** |
| Index 1000 files | ~100ms |
| Package install | System-dependent |
| System health check | ~50ms |

**Search Performance Comparison:**
- Traditional `find`: 100-1000ms
- `locate`: 10-50ms
- **Genesis Lightspeed**: **0.5-1ms** ⚡

## 🛠️ Technology

Genesis leverages cutting-edge technologies:

- **Rust**: Memory-safe, blazingly fast systems language
- **Rayon**: Data parallelism for multi-core performance
- **SIMD**: CPU vector instructions for accelerated fuzzy matching
- **N-gram indexing**: Advanced data structures for O(k) search
- **SymSpell**: Dictionary-based fuzzy search algorithm

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

MIT License - see [LICENSE](LICENSE) for details

## 🙏 Acknowledgments

- Built with ❤️ using Rust
- Inspired by modern CLI tools like `ripgrep`, `fd`, and `exa`
- Search algorithms based on academic research in information retrieval

---

<div align="center">

**Made with ⚡ by the Genesis Team**

[⭐ Star us on GitHub](https://github.com/Raindancer118/genesis) | [🐛 Report Bug](https://github.com/Raindancer118/genesis/issues) | [💡 Request Feature](https://github.com/Raindancer118/genesis/issues)

</div>
