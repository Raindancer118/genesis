use clap::{Parser, Subcommand};
use anyhow::Result;

mod ui;
mod config;
mod package_managers;
mod commands;
mod analytics;

#[derive(Parser, Debug)]
#[command(name = "vg")]
#[command(author = "Volantic")]
#[command(version = "3.8.3")]
#[command(about = "Volantic Genesis — Fast, focused system CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Update all package managers
    Update {
        #[arg(short, long)]
        yes: bool,
    },
    /// Search and install a package interactively
    Install {
        pkg: String,
        #[arg(short, long)]
        yes: bool,
    },
    /// Uninstall a package
    Uninstall {
        pkg: String,
    },
    /// Lightning-fast file search (SQLite FTS5 + interactive TUI)
    Search {
        /// Search query (omit to launch interactive TUI)
        query: Option<String>,
        /// Filter by extension(s), comma-separated (e.g. rs,md,toml)
        #[arg(short = 'e', long)]
        ext: Option<String>,
        /// Limit results to paths starting with this prefix
        #[arg(short = 'p', long)]
        path: Option<String>,
        /// Maximum number of results
        #[arg(short = 'l', long)]
        limit: Option<usize>,
        /// Launch interactive TUI (default when no query given)
        #[arg(short = 'i', long)]
        interactive: bool,
        /// Show match scores and timing breakdown
        #[arg(short = 'v', long)]
        verbose: bool,
        /// Search all indexed scopes including system files (default: user files only)
        #[arg(short = 'a', long)]
        all: bool,
    },
    /// Build or show file search index
    Index {
        #[arg(short, long)]
        info: bool,
        #[arg(short, long)]
        paths: Vec<String>,
        /// Run silently as a background job (used internally by auto-index)
        #[arg(long, hide = true)]
        background: bool,
    },
    /// Daily greeting
    Greet,
    /// System health report
    Health,
    /// System information
    Info,
    /// Update Volantic Genesis itself
    #[command(name = "self-update")]
    SelfUpdate,
    /// Wait until a new release is available, then install it automatically
    #[command(name = "expect-update")]
    ExpectUpdate {
        /// Polling interval in seconds (overrides config expect_update.interval_secs)
        #[arg(short = 'i', long)]
        interval: Option<u64>,
    },
    /// View or change settings
    Config {
        /// Action: list, get, set, edit
        action: Option<String>,
        /// Config key (e.g. search.max_results)
        key: Option<String>,
        /// Value to set
        value: Option<String>,
    },
    /// Create a bootable Manjaro KDE USB stick with Ventoy
    Manjaro,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config_manager = config::ConfigManager::new();

    // Fire analytics ping in background (non-blocking, daily max)
    analytics::maybe_ping(&config_manager);

    // Auto-index: spawn a background re-index if the interval has elapsed.
    // Skip if the current command IS already an index job (avoid recursion).
    let is_index_cmd = matches!(&cli.command, Commands::Index { .. });
    if !is_index_cmd {
        let ai = &config_manager.config.auto_index;
        let elapsed = config::ConfigManager::seconds_since_last_auto_index();
        if ai.enabled && elapsed >= ai.interval_minutes * 60 {
            // Stamp immediately so concurrent vg invocations don't all spawn at once.
            config::ConfigManager::touch_auto_index_stamp();
            if let Ok(exe) = std::env::current_exe() {
                let paths: Vec<String> = if ai.paths.is_empty() {
                    config_manager.config.search.default_paths.clone()
                } else {
                    ai.paths.clone()
                };
                let mut cmd = std::process::Command::new(exe);
                cmd.arg("index").arg("--background");
                for p in &paths { cmd.arg("--paths").arg(p); }
                cmd.stdout(std::process::Stdio::null())
                   .stderr(std::process::Stdio::null())
                   .stdin(std::process::Stdio::null());
                #[cfg(unix)]
                {
                    use std::os::unix::process::CommandExt;
                    // Detach from process group so it survives terminal close
                    unsafe { cmd.pre_exec(|| { libc::setsid(); Ok(()) }); }
                }
                let _ = cmd.spawn();
            }
        }
    }

    // Track command
    let cmd_name = match &cli.command {
        Commands::Update { .. } => "update",
        Commands::Install { .. } => "install",
        Commands::Uninstall { .. } => "uninstall",
        Commands::Search { .. } => "search",
        Commands::Index { .. } => "index",
        Commands::Greet => "greet",
        Commands::Health => "health",
        Commands::Info => "info",
        Commands::SelfUpdate => "self-update",
        Commands::ExpectUpdate { .. } => "expect-update",
        Commands::Config { .. } => "config",
        Commands::Manjaro => "manjaro",
    };
    analytics::track_command(&config_manager, cmd_name);

    match cli.command {
        Commands::Update { yes } => {
            commands::update::run(yes)?;
        }
        Commands::Install { pkg, yes } => {
            commands::package::install(&pkg, yes)?;
        }
        Commands::Uninstall { pkg } => {
            commands::package::uninstall(&pkg)?;
        }
        Commands::Search { query, ext, path, limit, interactive, verbose, all } => {
            let use_tui = interactive || query.is_none();
            if use_tui {
                let initial = query.as_deref().unwrap_or("");
                commands::search_tui::run_interactive_with_query(&config_manager, initial)?;
            } else {
                commands::search::search(commands::search::SearchParams {
                    query: query.unwrap(),
                    ext,
                    path_filter: path,
                    limit,
                    verbose,
                    all_scopes: all,
                }, &config_manager)?;
            }
        }
        Commands::Index { info, paths, background } => {
            if info {
                commands::search::info()?;
            } else {
                let paths_to_index: Vec<std::path::PathBuf> = if paths.is_empty() {
                    config_manager.config.search.default_paths.iter()
                        .map(|p| std::path::PathBuf::from(p))
                        .collect()
                } else {
                    paths.iter().map(|p| std::path::PathBuf::from(p)).collect()
                };
                // In background mode the parent already redirected stdio to null,
                // so build_index output is invisible. Stamp on success.
                commands::search::build_index(paths_to_index, &config_manager)?;
                if background {
                    config::ConfigManager::touch_auto_index_stamp();
                }
            }
        }
        Commands::Greet => {
            commands::greet::run();
        }
        Commands::Health => {
            commands::health::run()?;
        }
        Commands::Info => {
            commands::info::run();
        }
        Commands::SelfUpdate => {
            commands::self_update::run()?;
        }
        Commands::ExpectUpdate { interval } => {
            let secs = interval.unwrap_or(config_manager.config.expect_update.interval_secs);
            commands::self_update::expect_update(secs)?;
        }
        Commands::Config { action, key, value } => {
            commands::config_cmd::run(action, key, value, &mut config_manager)?;
        }
        Commands::Manjaro => {
            commands::manjaro::run()?;
        }
    }

    Ok(())
}
