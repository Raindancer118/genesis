use crate::config::ConfigManager;
use anyhow::{Result, Context, anyhow};
use colored::Colorize;
use inquire::Confirm;
use std::process::Command;
use sysinfo::{System, SystemExt, ProcessorExt, DiskExt}; 
use which::which;

pub fn install(packages: Vec<String>, config: &ConfigManager) -> Result<()> {
    if packages.is_empty() {
        println!("No packages specified.");
        return Ok(());
    }

    println!("{}", "📦 Package Installation".bold().cyan());

    if which("pacman").is_ok() {
        handle_arch_install(packages, config)?;
    } else if which("apt-get").is_ok() || which("apt").is_ok() {
        handle_debian_install(packages, config)?;
    } else if which("choco").is_ok() || which("winget").is_ok() || cfg!(windows) {
        handle_windows_install(packages, config)?;
    } else {
        return Err(anyhow!("No supported package manager found."));
    }

    Ok(())
}

fn handle_arch_install(packages: Vec<String>, config: &ConfigManager) -> Result<()> {
    let pamac = which("pamac").is_ok();
    
    let mut to_pacman = Vec::new();
    let mut to_pamac = Vec::new();
    let mut not_found = Vec::new(); 
    
    for pkg in &packages {
        println!("🔎 Searching for '{}'...", pkg);
        let status = Command::new("pacman").args(["-Si", pkg]).output()?;
        if status.status.success() {
            println!("  -> Found in official repositories.");
            to_pacman.push(pkg);
        } else if pamac {
            let status_aur = Command::new("pamac").args(["info", pkg]).output()?;
            if status_aur.status.success() {
                println!("  -> Found in AUR.");
                to_pamac.push(pkg);
            } else {
                not_found.push(pkg);
            }
        } else {
            not_found.push(pkg);
        }
    }

    if !not_found.is_empty() {
        println!("{}", format!("Warning: Could not find package(s): {}", not_found.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")).yellow().bold());
    }

    if to_pacman.is_empty() && to_pamac.is_empty() {
        println!("No packages to install.");
        return Ok(());
    }
    
    if config.config.system.default_install_confirm {
        if !Confirm::new("Proceed with installation?").with_default(true).prompt()? {
            println!("Cancelled.");
            return Ok(());
        }
    }

    if !to_pacman.is_empty() {
        println!("Installing via pacman: {:?}", to_pacman);
        let status = Command::new("sudo").arg("pacman").arg("-S").arg("--needed").arg("--noconfirm").args(to_pacman).status()?;
        if !status.success() {
            println!("{}", "Pacman install failed.".red());
        }
    }

    if !to_pamac.is_empty() {
        println!("Installing via pamac: {:?}", to_pamac);
        let status = Command::new("pamac").arg("build").arg("--no-confirm").args(to_pamac).status()?;
        if !status.success() {
            println!("{}", "Pamac install failed.".red());
        }
    }

    Ok(())
}

fn handle_debian_install(packages: Vec<String>, config: &ConfigManager) -> Result<()> {
    let apt = if which("apt").is_ok() { "apt" } else { "apt-get" };
    
    let mut to_install = Vec::new();
    let mut not_found = Vec::new();

    for pkg in &packages {
        println!("🔎 Checking availability for '{}'...", pkg);
        let status = Command::new("apt-get").args(["install", "--dry-run", pkg]).output()?;
        if status.status.success() {
             to_install.push(pkg);
        } else {
             not_found.push(pkg);
        }
    }
    
    if !not_found.is_empty() {
         println!("{}", format!("Warning: Could not find package(s): {}", not_found.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")).yellow().bold());
    }

    if to_install.is_empty() { return Ok(()); }
    
    println!("APT packages: {:?}", to_install);
    if config.config.system.default_install_confirm {
         if !Confirm::new("Proceed?").with_default(true).prompt()? { return Ok(()); }
    }

    let status = Command::new("sudo").arg(apt).arg("install").arg("-y").args(to_install).status()?;
    if !status.success() { println!("{}", "Install failed.".red()); }

    Ok(())
}

fn handle_windows_install(packages: Vec<String>, config: &ConfigManager) -> Result<()> {
    if which("choco").is_ok() {
        println!("Using Chocolatey.");
        if config.config.system.default_install_confirm {
             if !Confirm::new("Proceed with Chocolatey?").with_default(true).prompt()? { return Ok(()); }
        }
        for pkg in packages {
            println!("Installing {}...", pkg);
            Command::new("choco").args(["install", &pkg, "-y"]).status()?;
        }
    } else if which("winget").is_ok() {
        println!("Using Winget.");
        if config.config.system.default_install_confirm {
             if !Confirm::new("Proceed with Winget?").with_default(true).prompt()? { return Ok(()); }
        }
        for pkg in packages {
            println!("Installing {}...", pkg);
            Command::new("winget").args(["install", "-e", "--id", &pkg]).status()?;
        }
    }
    Ok(())
}

pub fn update(yes: bool, _config: &ConfigManager) -> Result<()> {
    println!("{}", "🔄 System Update Initiated".bold().blue());

    if which("pacman").is_ok() {
        println!("Updating Arch Linux system...");
        let mut cmd = Command::new("sudo");
        cmd.arg("pacman").arg("-Syu");
        if yes { cmd.arg("--noconfirm"); }
        cmd.status()?;
        
        if which("pamac").is_ok() {
            println!("Updating AUR (pamac)...");
            let mut cmd = Command::new("pamac");
            cmd.arg("update");
            if yes { cmd.arg("--no-confirm"); }
            cmd.status()?;
        } else if which("yay").is_ok() {
            println!("Updating AUR (yay)...");
             let mut cmd = Command::new("yay");
            cmd.arg("-Syu");
            if yes { cmd.arg("--noconfirm"); }
            cmd.status()?;
        }
    } else if which("apt-get").is_ok() {
        println!("Updating Debian/Ubuntu system...");
        Command::new("sudo").args(["apt-get", "update"]).status()?;
        let mut cmd = Command::new("sudo");
        cmd.args(["apt-get", "upgrade"]);
        if yes { cmd.arg("-y"); }
        cmd.status()?;
    } else if cfg!(windows) {
        if which("winget").is_ok() {
             println!("Updating Winget packages...");
             let mut cmd = Command::new("winget");
             cmd.args(["upgrade", "--all"]);
             // winget doesn't have simple 'yes' flag for all, usually prompts or --accept-package-agreements
             // --accept-source-agreements
             cmd.status()?;
        } else if which("choco").is_ok() {
             println!("Updating Chocolatey packages...");
             let mut cmd = Command::new("choco");
             cmd.args(["upgrade", "all"]);
             if yes { cmd.arg("-y"); }
             cmd.status()?;
        }
    }

    Ok(())
}

pub fn info() {
    let mut sys = System::new_all();
    sys.refresh_all();

    println!("{}", "System Information".bold().green());
    println!("{}: {}", "OS".bold(), sys.name().unwrap_or("Unknown".into()));
    println!("{}: {}", "Kernel".bold(), sys.kernel_version().unwrap_or("Unknown".into()));
    println!("{}: {}", "Host Name".bold(), sys.host_name().unwrap_or("Unknown".into()));
    println!("{}: {} cores", "CPU".bold(), sys.cpus().len());
    println!("{}: {} MB / {} MB", 
        "Memory".bold(), 
        sys.used_memory() / 1024 / 1024, 
        sys.total_memory() / 1024 / 1024
    );

    println!("\n{}", "Disks".bold());
    for disk in sys.disks() {
         println!("{}: {} / {} ({} free)", 
            disk.name().to_string_lossy(),
            format_bytes(disk.total_space() - disk.available_space()),
            format_bytes(disk.total_space()),
            format_bytes(disk.available_space())
         );
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNIT: u64 = 1024;
    if bytes < UNIT {
        return format!("{} B", bytes);
    }
    let div = UNIT as f64;
    let exp = (bytes as f64).log(div).floor() as i32;
    let pre = "KMGTPE".chars().nth((exp - 1) as usize).unwrap_or('?');
    format!("{:.1} {}B", (bytes as f64) / div.powi(exp), pre)
}
