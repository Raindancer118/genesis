use crate::config::ConfigManager;
use anyhow::{Result, anyhow};
use colored::Colorize;
use inquire::{Text, Select};
use std::process::Command;
use std::path::Path;
use which::which;

pub fn run(path: Option<String>) -> Result<()> {
    println!("{}", "🛡️  Virus Scan".bold().cyan());

    // 1. Check for ClamAV
    if which("clamscan").is_err() {
        return Err(anyhow!("ClamAV is not installed. Please install 'clamav' package first."));
    }
    
    // 2. Determine Target
    let target = match path {
        Some(p) => p,
        None => {
            // TUI Selection
            let options = vec![
                "Current Directory (.)",
                "Home Directory (~)",
                "Full System (/)",
                "Custom Path..."
            ];
            
            let selection = Select::new("Select scan target:", options).prompt()?;
            
            match selection {
                "Current Directory (.)" => ".".to_string(),
                "Home Directory (~)" => dirs::home_dir().unwrap_or_else(|| Path::new(".").into()).to_string_lossy().to_string(),
                "Full System (/)" => "/".to_string(),
                "Custom Path..." => Text::new("Enter path to scan:").prompt()?,
                _ => ".".to_string(),
            }
        }
    };

    let target_path = Path::new(&target);
    if !target_path.exists() {
         return Err(anyhow!("Path '{}' does not exist.", target));
    }

    // 3. Update Signatures (freshclam)
    println!("Checking virus definitions...");
    if which("freshclam").is_ok() {
        // Run with sudo if possible, or just try running it
        // freshclam usually requires root
        println!("Updating signatures (might require sudo)...");
        let _ = Command::new("sudo").arg("freshclam").status(); 
    }

    // 4. Run Scan
    println!("Scanning '{}'...", target);
    
    let mut args = vec!["-r", "--bell", "-i", &target]; // -r recursive, -i infected only, --bell sound
    
    let status = Command::new("clamscan").args(&args).status()?;
    
    if status.success() {
        println!("{}", "Scan complete. No threats found.".green());
    } else {
        // Exit code 1 means virus found (usually).
        match status.code() {
            Some(1) => println!("{}", "⚠️  Threats found! Check output above.".red().bold()),
            Some(0) => println!("{}", "Scan complete. Clean.".green()),
            _ => println!("{}", "Scan error or cancelled.".yellow()),
        }
    }

    Ok(())
}
