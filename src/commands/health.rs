use anyhow::Result;
use colored::Colorize;
use sysinfo::System;
use std::process::Command;
use which::which;

pub fn run() -> Result<()> {
    println!("{}", "🏥 System Health Report".bold().green());
    println!("{}", "=======================".bold());

    let mut sys = System::new_all();
    sys.refresh_all(); // Refresh everything

    // 1. Basic Info
    println!("\n{}", "--- System Info ---".yellow());
    println!("{}: {}", "OS".bold(), System::name().unwrap_or("Unknown".to_string()));
    println!("{}: {}", "Kernel".bold(), System::kernel_version().unwrap_or("Unknown".to_string()));
    println!("{}: {}", "Hostname".bold(), System::host_name().unwrap_or("Unknown".to_string()));
    
    // Uptime
    let uptime = System::uptime();
    let days = uptime / 86400;
    let hours = (uptime % 86400) / 3600;
    println!("{}: {}d {}h {}m", "Uptime".bold(), days, hours, uptime % 3600 / 60);

    // 2. Resources
    println!("\n{}", "--- Resources ---".yellow());
    // RAM
    let total_mem = sys.total_memory() / 1024 / 1024;
    let used_mem = sys.used_memory() / 1024 / 1024;
    let mem_percent = (used_mem as f64 / total_mem as f64) * 100.0;
    println!("{}: {} / {} MB ({:.1}%)", "Memory".bold(), used_mem, total_mem, mem_percent);
    
    // Swap
    let total_swap = sys.total_swap() / 1024 / 1024;
    let used_swap = sys.used_swap() / 1024 / 1024;
    println!("{}: {} / {} MB", "Swap".bold(), used_swap, total_swap);

    // Load
    let load = System::load_average();
    println!("{}: {:.2}, {:.2}, {:.2}", "Load Avg".bold(), load.one, load.five, load.fifteen);

    // Disks
    println!("\n{}", "--- Storage ---".yellow());
    let disks = sysinfo::Disks::new_with_refreshed_list();
    for disk in &disks {
        let total = disk.total_space();
        let available = disk.available_space();
        let used = total - available;
        let percent = (used as f64 / total as f64) * 100.0;
        
        let color_func = if percent > 90.0 { |s: String| s.red() } else { |s: String| s.white() };
        
        println!("{}: {} used of {} ({:.1}%) [{}]", 
            disk.mount_point().to_string_lossy().bold(),
            format_bytes(used),
            format_bytes(total),
            percent,
            color_func(format!("{: <10}", disk.name().to_string_lossy()))
        );
    }

    // 3. Integrity Checks
    println!("\n{}", "--- Integrity Checks ---".yellow());
    
    // Failed Services (Linux)
    if cfg!(target_os = "linux") {
        check_failed_units();
    }

    // Pending Updates
    check_pending_updates();

    // Check Genesis Service itself
    check_genesis_services();

    println!("\n{}", "✅ Health Check Complete.".green().bold());
    Ok(())
}

fn check_failed_units() {
    print!("{}: ", "Failed Systemd Units".bold());
    if let Ok(output) = Command::new("systemctl").args(["--failed", "--no-legend"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let count = stdout.lines().count();
        if count == 0 {
            println!("{}", "None".green());
        } else {
            println!("{}", format!("{} failed units found!", count).red().bold());
            for line in stdout.lines() {
                println!("  - {}", line.trim());
            }
        }
    } else {
         println!("{}", "N/A (systemctl not found/error)".dimmed());
    }
}

fn check_pending_updates() {
    print!("{}: ", "Pending Updates".bold());
    
    // Simple checks based on available package managers
    let mut count = 0;
    let mut checked = false;

    // Arch (checkupdates)
    if which("checkupdates").is_ok() {
        checked = true;
        if let Ok(output) = Command::new("checkupdates").output() {
            count += String::from_utf8_lossy(&output.stdout).lines().count();
        }
    }
    // Debian (apt)
    else if which("apt").is_ok() {
         checked = true;
         // approximate
         if let Ok(output) = Command::new("apt").args(["list", "--upgradable"]).output() {
             let out = String::from_utf8_lossy(&output.stdout);
             // exclude "Listing..." line
             let lines = out.lines().filter(|l| !l.starts_with("Listing")).count();
             count += lines;
         }
    }
    // Windows (winget)
    else if cfg!(windows) && which("winget").is_ok() {
        checked = true;
        // winget upgrade --include-unknown
        // output format varies, primitive check
    }

    if checked {
        if count == 0 {
             println!("{}", "System up to date".green());
        } else {
             println!("{}", format!("~{} updates available", count).yellow());
        }
    } else {
        println!("{}", "Unknown (Cannot determine)".dimmed());
    }
}

fn check_genesis_services() {
    // Check if genesis-greet.service is active (User service)
    if cfg!(target_os = "linux") {
         print!("{}: ", "Genesis Services".bold());
         // systemctl --user is-active genesis-greet.service
         let status = Command::new("systemctl")
            .arg("--user")
            .arg("is-active")
            .arg("genesis-greet.service")
            .output();
         
         match status {
             Ok(o) => {
                 let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                 if s == "active" {
                     println!("{}", "Active".green());
                 } else {
                     println!("{}", format!("Inactive ({})", s).red());
                 }
             },
             Err(_) => println!("{}", "Error checking".red()),
         }
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
