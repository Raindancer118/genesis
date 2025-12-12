use anyhow::Result;
use colored::Colorize;
use git2::{Repository, StatusOptions};
use sysinfo::{System, SystemExt};
use std::env;

pub fn run() -> Result<()> {
    println!("{}", "📊 Status Check".bold().blue());

    // 1. Git Status
    match Repository::open(".") {
        Ok(repo) => {
            if let Ok(head) = repo.head() {
                let branch = head.shorthand().unwrap_or("DETACHED");
                println!("Git Branch: {}", branch.yellow().bold());
            }

            let mut opts = StatusOptions::new();
            opts.include_untracked(true);
            
            match repo.statuses(Some(&mut opts)) {
                Ok(statuses) => {
                    if statuses.is_empty() {
                         println!("Git Status: {}", "Clean".green());
                    } else {
                         println!("Git Status: {} changes", statuses.len().to_string().red().bold());
                         for entry in statuses.iter().take(5) {
                             let path = entry.path().unwrap_or("?");
                             let status = entry.status();
                             println!("  - {} ({:?})", path, status);
                         }
                         if statuses.len() > 5 {
                             println!("  ... and {} more", statuses.len() - 5);
                         }
                    }
                },
                Err(e) => println!("Git Error: {}", e),
            }
        },
        Err(_) => {
            println!("Current directory is not a Git repository.");
        }
    }

    // 2. System Load
    let mut sys = System::new_all();
    sys.refresh_cpu();
    let load = System::load_average();
    println!("\nSystem Load: {:.2}, {:.2}, {:.2}", load.one, load.five, load.fifteen);
    
    // Uptime
    let uptime = System::uptime();
    let days = uptime / 86400;
    let hours = (uptime % 86400) / 3600;
    let mins = (uptime % 3600) / 60;
    println!("Uptime: {}d {}h {}m", days, hours, mins);

    Ok(())
}
