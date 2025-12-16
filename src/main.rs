use clap::{Parser, Subcommand};
use anyhow::Result;

mod config;
mod commands;
mod ai;

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
    // ═══════════════════════════════════════════════════════════════════════
    // PACKAGE MANAGEMENT
    // ═══════════════════════════════════════════════════════════════════════
    
    /// [Package] Install packages across any package manager
    #[command(visible_alias = "i")]
    Install {
        /// Packages to install
        packages: Vec<String>,
    },
    
    /// [Package] Remove/uninstall packages
    #[command(visible_alias = "rm")]
    Remove {
        /// Packages to remove
        packages: Vec<String>,
    },
    
    /// [Package] Update all system packages and package managers
    #[command(visible_alias = "up")]
    Update {
        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
        
        /// Only update specific package managers (comma-separated: apt,brew,cargo)
        #[arg(short, long)]
        only: Option<String>,
        
        /// Show detailed output
        #[arg(short, long)]
        verbose: bool,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // FILE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════
    
    /// [Files] Search files using lightning-fast index
    #[command(visible_alias = "s")]
    Search { 
        /// Query string to search for in file names and paths
        query: String 
    },
    
    /// [Files] Build or manage file search index
    Index {
        /// Paths to index (uses default from config if not specified)
        #[arg(short, long)]
        paths: Vec<String>,
        
        /// Display index information
        #[arg(short, long)]
        info: bool,
    },
    
    /// [Files] Organize and sort files intelligently
    Sort {
        /// Path to sort
        #[arg(default_value = ".")]
        path: String,
    },
    
    /// [Files] Analyze disk usage
    Storage {
        /// Path to analyze
        #[arg(default_value = ".")]
        path: String,
    },
    
    /// [Files] Scan directory (experimental)
    Scan { 
        /// Path to scan
        path: Option<String> 
    },

    // ═══════════════════════════════════════════════════════════════════════
    // SYSTEM TOOLS
    // ═══════════════════════════════════════════════════════════════════════
    
    /// [System] Display system information
    Info,
    
    /// [System] Check system health and status
    Health,
    
    /// [System] Kill resource-intensive processes interactively
    Hero {
        /// Show what would be killed without actually killing
        #[arg(long)]
        dry_run: bool,

        /// Scope of processes to analyze: 'user' or 'all'
        #[arg(long, default_value = "user")]
        scope: String,

        /// Memory threshold in MB (processes using more will be shown)
        #[arg(short = 'm', long, default_value_t = 500)]
        mem_threshold: u64,

        /// CPU threshold in % (processes using more will be shown)
        #[arg(short = 'c', long, default_value_t = 50.0)]
        cpu_threshold: f32,

        /// Maximum number of processes to show
        #[arg(short = 'l', long, default_value_t = 20)]
        limit: usize,

        /// Suppress output except for critical information
        #[arg(short = 'q', long)]
        quiet: bool,

        /// Skip CPU sampling for faster execution
        #[arg(short = 'f', long)]
        fast: bool,
        
        /// Automatically kill top N processes without prompting
        #[arg(short = 'a', long)]
        auto: Option<usize>,
    },
    
    /// [System] System performance benchmark
    Benchmark,
    
    /// [System] Real-time system monitoring
    #[command(hide = true)]
    Monitor,
    
    /// [System] View and analyze system logs
    Logs {
        /// Action: view, search, tail
        action: Option<String>,
    },
    
    /// [System] Network diagnostics and information
    Network {
        /// Action: status, ping, trace
        action: Option<String>,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // DEVELOPER TOOLS
    // ═══════════════════════════════════════════════════════════════════════
    
    /// [Dev] Create new project from template
    New {
        /// Project name
        name: String,
        
        /// Project template (rust, python, node, etc)
        #[arg(short, long)]
        template: Option<String>,
        
        /// Initialize git repository
        #[arg(short, long)]
        git: bool,
        
        /// Skip confirmation prompts
        #[arg(short, long)]
        yes: bool,
        
        /// Custom project structure
        #[arg(short, long)]
        structure: Option<String>,
    },
    
    /// [Dev] Build project from template
    Build {
        /// Template name
        name: String,
    },
    
    /// [Dev] Check project status
    Status,
    
    /// [Dev] Manage environment variables
    Env {
        /// Action: list, set, unset
        action: Option<String>,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // PRODUCTIVITY TOOLS
    // ═══════════════════════════════════════════════════════════════════════
    
    /// [Productivity] Quick calculator
    Calc {
        /// Expression to calculate
        expression: Option<String>,
    },
    
    /// [Productivity] Quick notes manager
    Notes {
        /// Action: add, list, view
        action: Option<String>,
    },
    
    /// [Productivity] Todo list manager
    Todo {
        /// Action: add, list, done
        action: Option<String>,
    },
    
    /// [Productivity] Timer and stopwatch
    Timer {
        /// Mode: timer, stopwatch
        mode: Option<String>,
        
        /// Duration for timer (e.g., 5m, 1h)
        duration: Option<String>,
    },

    // ═══════════════════════════════════════════════════════════════════════
    // UTILITIES
    // ═══════════════════════════════════════════════════════════════════════
    
    /// [Utility] Interactive setup wizard
    Setup,
    
    /// [Utility] Update Genesis itself
    #[command(name = "self-update")]
    SelfUpdate,
    
    /// [Utility] Daily greeting service
    Greet,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut config_manager = config::ConfigManager::new();
    run_rust(&mut config_manager).await
}

async fn run_rust(config_manager: &mut config::ConfigManager) -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        // Package Management
        Commands::Install { packages } => {
             commands::system::install(packages, config_manager)?;
        }
        Commands::Remove { packages } => {
             commands::system::remove(packages, config_manager)?;
        }
        Commands::Update { yes, only, verbose } => {
             commands::system::update_revamped(yes, only, verbose, config_manager)?;
        }
        
        // File Operations
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
        Commands::Sort { path } => {
             commands::sort::run(path)?;
        }
        Commands::Storage { path } => {
             commands::storage::run(Some(path))?;
        }
        Commands::Scan { path } => {
             commands::scan::run(path.clone())?;
        }
        
        // System Tools
        Commands::Info => {
             commands::system::info();
        }
        Commands::Health => {
             commands::health::run()?;
        }
        Commands::Hero { dry_run, scope, mem_threshold, cpu_threshold, limit, quiet, fast, auto } => {
             commands::hero::run_revamped(dry_run, scope, mem_threshold, cpu_threshold, limit, quiet, fast, auto)?;
        }
        Commands::Benchmark => {
             commands::benchmark::run()?;
        }
        Commands::Monitor => {
             commands::monitor::run()?;
        }
        Commands::Logs { action } => {
             commands::logs::run(action)?;
        }
        Commands::Network { action } => {
             commands::network::run(action)?;
        }
        
        // Developer Tools
        Commands::New { name, template, git, yes, structure } => {
            commands::project::run_new(Some(name), template, git, yes, structure, config_manager)?;
        }
        Commands::Build { name: _ } => {
             // Build command is not fully implemented yet
             println!("Build command is not yet implemented.");
        }
        Commands::Status => {
             commands::status::run()?;
        }
        Commands::Env { action } => {
             commands::env::run(action)?;
        }
        
        // Productivity Tools
        Commands::Calc { expression } => {
             commands::calc::run(expression)?;
        }
        Commands::Notes { action } => {
             commands::notes::run(action)?;
        }
        Commands::Todo { action } => {
             commands::todo::run(action)?;
        }
        Commands::Timer { mode, duration } => {
             commands::timer::run(mode, duration)?;
        }
        
        // Utilities
        Commands::Setup => {
            commands::setup::run(config_manager)?;
        }
        Commands::SelfUpdate => {
             commands::self_update::run()?;
        }
        Commands::Greet => {
            commands::greet::run();
        }
    }

    Ok(())
}
