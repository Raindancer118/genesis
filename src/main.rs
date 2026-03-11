use clap::{Parser, Subcommand};
use anyhow::Result;

mod ui;
mod config;
mod package_managers;
mod commands;

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
        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },
    /// Search and install a package interactively
    Install {
        /// Package name to search for
        pkg: String,
        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
    },
    /// Uninstall a package
    Uninstall {
        /// Package name
        pkg: String,
    },
    /// Lightning-fast file search
    Search {
        /// Query string
        query: String,
    },
    /// Build or show file search index
    Index {
        /// Display index information
        #[arg(short, long)]
        info: bool,
        /// Paths to index
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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_manager = config::ConfigManager::new();

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
    }

    Ok(())
}
