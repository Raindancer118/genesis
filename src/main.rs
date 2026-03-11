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
#[command(version = "3.0.0")]
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
    /// Lightning-fast file search (SQLite FTS5)
    Search {
        query: String,
    },
    /// Build or show file search index
    Index {
        #[arg(short, long)]
        info: bool,
        #[arg(short, long)]
        paths: Vec<String>,
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
    /// View or change settings
    Config {
        /// Action: list, get, set, edit
        action: Option<String>,
        /// Config key (e.g. search.max_results)
        key: Option<String>,
        /// Value to set
        value: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config_manager = config::ConfigManager::new();

    // Fire analytics ping in background (non-blocking, daily max)
    analytics::maybe_ping(&config_manager);

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
        Commands::Config { .. } => "config",
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
        Commands::Search { query } => {
            commands::search::search(query, &config_manager)?;
        }
        Commands::Index { info, paths } => {
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
                commands::search::build_index(paths_to_index, &config_manager)?;
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
        Commands::Config { action, key, value } => {
            commands::config_cmd::run(action, key, value, &mut config_manager)?;
        }
    }

    Ok(())
}
