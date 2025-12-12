use anyhow::{Result, Context, anyhow};
use colored::Colorize;
use std::process::Command;
use std::env;
use std::path::Path;

pub fn run() -> Result<()> {
    println!("{}", "🚀 Self-Update Initiated...".bold().cyan());

    // 1. Determine Installation Directory
    // We assume /opt/genesis or find relative to executable
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().context("Failed to get executable directory")?;
    
    // Heuristic: If we are in target/release, project root is ../../
    // If installed via script, symlinked from /usr/local/bin, current_exe returns the resolved path.
    // e.g. /opt/genesis/target/release/genesis
    
    let project_root = if exe_dir.ends_with("release") && exe_dir.parent().unwrap().ends_with("target") {
        exe_dir.parent().unwrap().parent().unwrap()
    } else {
        // Fallback or assume /opt/genesis
        Path::new("/opt/genesis")
    };

    if !project_root.exists() || !project_root.join(".git").exists() {
        return Err(anyhow!("Could not locate Genesis Git repository at {:?}. Cannot self-update.", project_root));
    }

    println!("Project Root: {:?}", project_root);

    // 2. Git Pull
    println!("📦 Pulling latest changes...");
    let status = Command::new("git")
        .arg("pull")
        .current_dir(project_root)
        .status()
        .context("Failed to run git pull")?;

    if !status.success() {
        return Err(anyhow!("Git pull failed."));
    }

    // 2.5 Show Changelog
    let changelog_path = project_root.join("CHANGELOG.md");
    if changelog_path.exists() {
        println!("\n{}", "While you wait: This is new to genesis:".bold().magenta());
        // Simple parser: Extract the first section under valid headers
        if let Ok(content) = std::fs::read_to_string(changelog_path) {
            let mut lines = content.lines();
            let mut printing = false;
            let mut count = 0;
            
            for line in lines {
                if line.starts_with("## [") {
                    if printing { break; } // Stop at next header
                    printing = true;
                    println!("{}", line.bold());
                    continue;
                }
                if printing {
                    println!("{}", line);
                    count += 1;
                    if count > 20 { 
                        println!("... (see CHANGELOG.md for more)"); 
                        break; 
                    }
                }
            }
            println!(); 
        }
    }

    // 3. Rebuild
    println!("🔨 Rebuilding...");
    let cargo = if env::var("CARGO").is_ok() {
        "cargo".to_string()
    } else {
        // Try to find cargo in path or ~/.cargo/bin
        if which::which("cargo").is_ok() {
            "cargo".to_string()
        } else {
            let home = env::var("HOME").unwrap_or_default();
            format!("{}/.cargo/bin/cargo", home)
        }
    };

    let status = Command::new(cargo)
        .arg("build")
        .arg("--release")
        .current_dir(project_root)
        .status()
        .context("Failed to run cargo build")?;

    if !status.success() {
        return Err(anyhow!("Build failed."));
    }

    println!("{}", "✅ Self-update complete! Please restart Genesis.".green().bold());

    Ok(())
}
