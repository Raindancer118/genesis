# Volantic Genesis `vg`

> Fast, focused system CLI — package management, file search, and system health in one tool.

```
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ◂
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━
    ·  ·  ·

  V O L A N T I C   G E N E S I S
  ─────────────────────────────────
  v3.0.0
```

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/Raindancer118/genesis/main/install.sh | bash
```

Installs the pre-built binary to `/usr/local/bin/vg`. Requires sudo for the final copy step (the script will ask).

---

## Commands

| Command | Description |
|---|---|
| `vg update` | Update all available package managers |
| `vg install <pkg>` | Search across all PMs in parallel → pick interactively → install |
| `vg uninstall <pkg>` | Uninstall a package |
| `vg search <query>` | Lightning-fast file search (SQLite FTS5) |
| `vg index [--info]` | Build or inspect the file search index |
| `vg health` | System health report |
| `vg info` | System information |
| `vg greet` | Daily greeting (used by systemd service) |
| `vg config` | View or change settings |
| `vg self-update` | Pull latest changes and rebuild |

---

## Package Manager Support

`vg install` and `vg update` detect and use whatever is available on your system:

**Arch / Manjaro** — pamac · yay · paru · pacman
**Debian / Ubuntu** — apt
**Universal** — flatpak · snap
**Language** — cargo · npm · pipx
**macOS** — brew

Priority on Arch/Manjaro: `pamac → yay → paru → pacman → flatpak → snap → language tools`

---

## File Search

`vg search` uses a **SQLite FTS5** index — sub-millisecond full-text search over filenames and paths.

```bash
vg index                    # index your home directory (from config)
vg index --paths /srv /etc  # index specific paths
vg index --info             # show index stats
vg search nginx.conf
vg search .config
```

The index lives at `~/.local/share/volantic/genesis/search.db`.

---

## Configuration

```bash
vg config list              # show all settings
vg config get search.max_results
vg config set search.max_results 100
vg config edit              # interactive editor
```

**Available keys:**

| Key | Default | Description |
|---|---|---|
| `search.max_results` | `50` | Max results shown |
| `search.max_depth` | `10` | Directory depth for indexing |
| `search.exclude_hidden` | `true` | Skip hidden files/dirs |
| `search.fuzzy_threshold` | `2` | Edit distance for fuzzy search |
| `system.auto_confirm_update` | `false` | Skip prompts during `vg update` |
| `analytics.enabled` | `true` | Send anonymous daily ping |
| `analytics.track_commands` | `false` | Include command name in ping |

Config file: `~/.config/volantic/genesis/config.toml`

---

## Analytics

`vg` sends an anonymous daily ping to `analytics.volantic.de` — **enabled by default**.

To opt out:
```bash
vg config set analytics.enabled false
```

What is sent:
```json
{
  "tool": "volantic-genesis",
  "client_id": "a1b2c3d4",
  "version": "3.0.0",
  "os": "linux",
  "arch": "x86_64",
  "timestamp": "2026-03-11T10:00:00Z"
}
```

`client_id` is a one-way SHA256 hash of your hostname and username — it cannot be reversed. No file paths, usernames, or IP addresses are stored.

---

## systemd Services

Two optional user services ship with `vg`:

| File | Purpose |
|---|---|
| `vg-greet.service` | Runs `vg greet` at login |
| `vg-sentry.service` | Runs `vg health` every 15 min |
| `vg-sentry.timer` | Timer for sentry service |

Install manually:
```bash
cp vg-*.service vg-*.timer ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now vg-greet.service
systemctl --user enable --now vg-sentry.timer
```

---

## Build from Source

```bash
git clone https://github.com/Raindancer118/genesis
cd genesis
cargo build --release
sudo cp target/release/vg /usr/local/bin/vg
```

Requires Rust 1.75+ and a C compiler (for bundled SQLite).

---

## License

MIT — © Volantic
