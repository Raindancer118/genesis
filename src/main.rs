use clap::{Parser, Subcommand};
use colored::Colorize;
use anyhow::{Result, Context};
use std::process::Command;
use std::path::Path;
use std::env;

mod config;
mod commands;

#[derive(Parser, Debug)]
#[command(name = "genesis")]
#[command(about = "Genesis is your personal command-line assistant", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Displays a custom morning greeting
    Greet,

    /// Opens the interactive configuration wizard
    Setup,

    /// Initializes a new project
    New {
        /// Project name
        #[arg(long)]
        name: Option<String>,

        /// Project template to use
        #[arg(long)]
        template: Option<String>,

        /// Initialize a Git repository
        #[arg(long)]
        git: bool,

        /// Skip confirmation
        #[arg(long, short = 'y')]
        yes: bool,

        /// JSON structure definition
        #[arg(long)]
        structure: Option<String>,
    },

    /// Builds a project structure from a live text template
    Build {
        name: String,
    },

    /// Sorts files in a directory
    Sort {
        #[arg(default_value = ".")]
        path: String,
    },

    /// Scans for viruses
    Scan {
        path: Option<String>,
    },

    /// Searches for packages
    Search {
        query: String,
    },

    /// Finds and installs packages
    Install {
        #[arg(required = true)]
        packages: Vec<String>,
    },

    /// Finds and removes packages
    Remove {
        #[arg(required = true)]
        packages: Vec<String>,
    },

    /// Performs a full system update
    Update {
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Updates Genesis itself
    SelfUpdate,

    /// System status check (and Info)
    Info,

    /// Background monitor (Hidden)
    #[command(hide = true)]
    Monitor,

    /// Deep system health check
    Health,

    /// Kill resource-intensive processes
    Hero {
        #[arg(long)]
        dry_run: bool,

        #[arg(long, default_value = "user")]
        scope: String,

        #[arg(long)]
        mem_threshold: Option<f64>,

        #[arg(long)]
        cpu_threshold: Option<f64>,

        #[arg(long, default_value_t = 15)]
        limit: usize,

        #[arg(long)]
        quiet: bool,

        #[arg(long)]
        fast: bool,
    },

    /// Analyze disk usage
    Storage {
        path: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut config_manager = config::ConfigManager::new();
    run_rust(&mut config_manager).await
}

async fn run_rust(config_manager: &mut config::ConfigManager) -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Greet => {
            commands::greet::run();
        }
        Commands::Setup => {
            commands::setup::run(config_manager)?;
        }
        Commands::New { name, template, git, yes, structure } => {
            commands::project::run_new(name, template, git, yes, structure, config_manager)?;
        }
        Commands::Build { name } => {
             commands::project::run_build(name, None)?; // we need to parse template arg if it existed in CLI
             // Wait, CLI definition of Build:
             // Build { name: String }
             // It doesn't take template/content arg?
             // Python `build` command took `template_string`?
             // Checks genesis.py for `build` args.
             // @click.argument('template_string', required=False)?
             // If not provided, it likely prompts?
             // My run_build prompts if None.
        }
        Commands::Sort { path } => {
             commands::sort::run(path)?;
        }
        Commands::Scan { path } => {
             println!("Scan command not implemented yet.");
        }
        Commands::Search { query } => {
             println!("Search command not implemented yet.");
        }
        Commands::Install { packages } => {
             commands::system::install(packages, config_manager)?;
        }
        Commands::Remove { packages } => {
             // commands::system::remove(packages, config_manager)?;
             println!("Remove command not implemented yet.");
        }
        Commands::Update { yes } => {
             commands::system::update(yes, config_manager)?;
        }
        Commands::SelfUpdate => {
             commands::self_update::run()?;
        }
        Commands::Status => {
             commands::status::run()?;
        }
        Commands::Info => {
             commands::system::info();
        }
        Commands::Monitor => {
             println!("Monitor command not implemented yet.");
        }
        Commands::Health => {
             commands::health::run()?;
        }
        Commands::Hero { dry_run, scope, mem_threshold, cpu_threshold, limit, quiet, fast } => {
             commands::hero::run(dry_run, scope, mem_threshold, cpu_threshold, limit, quiet, fast);
        }
        Commands::Storage { path } => {
             commands::storage::run(path)?;
        }
    }

    Ok(())
}
