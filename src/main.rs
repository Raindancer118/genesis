use clap::{Parser, Subcommand};
use anyhow::Result;

mod config;
mod commands;

#[derive(Parser, Debug)]
#[command(name = "genesis")]
#[command(author = "Genesis Team")]
#[command(version = "2.0.0-lightspeed")]
#[command(about = "⚡ Lightning-fast CLI tool with intelligent search, system management, and automation", long_about = "Genesis is a powerful next-generation CLI tool that combines lightning-fast file search (Lightspeed mode), comprehensive system management, package handling across all major platforms, and intelligent automation features. Built with Rust for maximum performance and reliability.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Interactive Setup
    Setup,
    /// System Info & Package Management
    System {
        #[arg(short, long)]
        info: bool,
        
        #[arg(short, long)]
        install: Option<String>,

        #[arg(short, long)]
        remove: Option<String>,
        
        #[arg(short, long)]
        update: bool,
    },
    /// Greeting Service
    Greet,
    /// Kill resource-intensive processes
    Hero {
        #[arg(long)]
        dry_run: bool,

        #[arg(long, default_value = "user")]
        scope: String,

        #[arg(short = 'm', long, default_value_t = 500)]
        mem_threshold: u64,

        #[arg(short = 'c', long, default_value_t = 50.0)]
        cpu_threshold: f32,

        #[arg(long, default_value_t = 15)]
        limit: usize,

        #[arg(long)]
        quiet: bool,

        #[arg(long)]
        fast: bool,
    },
    /// File Sorting
    Sort {
        #[arg(default_value = ".")]
        path: String,
    },
    /// Status Check
    Status,
    /// Disk Usage
    Storage {
         #[arg(default_value = ".")]
         path: String,
    },
    /// New Project
    New {
        name: String,
        #[arg(short, long)]
        template: Option<String>,
        #[arg(short, long)]
        git: bool,
        #[arg(short, long)]
        yes: bool,
        #[arg(short, long)]
        structure: Option<String>,
    },
    /// Build from Template
    /// Build from Template
    Build {
        name: String,
    },
    /// Self Update
    SelfUpdate,
    /// System Health
    Health,
    /// Update System Packages
    Update {
        #[arg(short, long)]
        yes: bool,
    },
    /// Scan (Not Implemented)
    Scan { 
        path: Option<String> 
    },
    /// Search files in the index
    Search { 
        /// Query string to search for in file names and paths
        query: String 
    },
    /// Build file index for search
    Index {
        /// Paths to index (uses default from config if not specified)
        #[arg(short, long)]
        paths: Vec<String>,
        
        /// Display index information
        #[arg(short, long)]
        info: bool,
    },
    /// Install (Top level shortcut for legacy parity)
    Install {
        packages: Vec<String>,
    },
    /// Remove (Top level shortcut)
    Remove {
        packages: Vec<String>,
    },
    /// Info (Top level shortcut)
    Info,
    /// Monitor
    #[command(hide = true)]
    Monitor,
    /// System Benchmark
    Benchmark,
    /// Calculator
    Calc {
        expression: Option<String>,
    },
    /// Environment Variables
    Env {
        action: Option<String>,
    },
    /// System Logs Viewer
    Logs {
        action: Option<String>,
    },
    /// Network Diagnostics
    Network {
        action: Option<String>,
    },
    /// Quick Notes
    Notes {
        action: Option<String>,
    },
    /// Timer & Stopwatch
    Timer {
        mode: Option<String>,
        duration: Option<String>,
    },
    /// Todo Manager
    Todo {
        action: Option<String>,
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
        Commands::Setup => {
            commands::setup::run(config_manager)?;
        }
        Commands::System { info, install, .. } => {
            if info {
                 commands::system::info();
            }
            if let Some(pkg) = install {
                 commands::system::install(vec![pkg.clone()], config_manager)?;
            }
        }
        Commands::Greet => {
            commands::greet::run();
        }
        Commands::Hero { dry_run, scope, mem_threshold, cpu_threshold, limit, quiet, fast } => {
             commands::hero::run(dry_run, scope, mem_threshold, cpu_threshold, limit, quiet, fast);
        }
        Commands::Sort { path } => {
             commands::sort::run(path)?;
        }
        Commands::Status => {
             commands::status::run()?;
        }
        Commands::Storage { path } => {
             commands::storage::run(Some(path))?;
        }
        Commands::New { name, template, git, yes, structure } => {
            commands::project::run_new(Some(name), template, git, yes, structure, config_manager)?;
        }
        Commands::Build { name } => {
             // If not provided, it likely prompts?
             // My run_build prompts if None.
        }
        Commands::Scan { path } => {
             commands::scan::run(path.clone())?;
        }
        Commands::Search { query } => {
             commands::search::search(query, config_manager)?;
        }
        Commands::Index { paths, info } => {
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
                 commands::search::build_index(paths_to_index, config_manager)?;
             }
        }
        Commands::Install { packages } => {
             commands::system::install(packages, config_manager)?;
        }
        Commands::Remove { packages } => {
             commands::system::remove(packages, config_manager)?;
        }
        Commands::Update { yes } => {
             commands::system::update(yes, config_manager)?;
        }
        Commands::SelfUpdate => {
             commands::self_update::run()?;
        }
        Commands::Info => {
             commands::system::info();
        }
        Commands::Monitor => {
             commands::monitor::run()?;
        }
        Commands::Health => {
             commands::health::run()?;
        }
        Commands::Benchmark => {
             commands::benchmark::run()?;
        }
        Commands::Calc { expression } => {
             commands::calc::run(expression)?;
        }
        Commands::Env { action } => {
             commands::env::run(action)?;
        }
        Commands::Logs { action } => {
             commands::logs::run(action)?;
        }
        Commands::Network { action } => {
             commands::network::run(action)?;
        }
        Commands::Notes { action } => {
             commands::notes::run(action)?;
        }
        Commands::Timer { mode, duration } => {
             commands::timer::run(mode, duration)?;
        }
        Commands::Todo { action } => {
             commands::todo::run(action)?;
        }
    }

    Ok(())
}
