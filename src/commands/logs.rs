use anyhow::Result;
use colored::Colorize;
use std::process::Command;
use inquire::{Text, Select, Confirm};
use which::which;

pub fn run(action: Option<String>) -> Result<()> {
    println!("{}", "📋 System Logs Viewer".bold().blue());
    
    let action = match action {
        Some(a) => a,
        None => {
            let options = vec![
                "Recent Logs",
                "Follow Logs",
                "System Logs",
                "Service Logs",
                "Kernel Logs",
                "Authentication Logs",
            ];
            Select::new("Select action:", options).prompt()?.to_string()
        }
    };
    
    match action.as_str() {
        "Recent Logs" | "recent" => show_recent_logs()?,
        "Follow Logs" | "follow" | "tail" => follow_logs()?,
        "System Logs" | "system" | "syslog" => show_system_logs()?,
        "Service Logs" | "service" => show_service_logs()?,
        "Kernel Logs" | "kernel" | "dmesg" => show_kernel_logs()?,
        "Authentication Logs" | "auth" => show_auth_logs()?,
        _ => println!("{}", "Unknown action".red()),
    }
    
    Ok(())
}

fn show_recent_logs() -> Result<()> {
    println!("\n{}", "Recent System Logs".yellow().bold());
    
    #[cfg(target_os = "linux")]
    {
        if which("journalctl").is_ok() {
            println!("Using journalctl...\n");
            let lines = Text::new("Number of lines:")
                .with_default("50")
                .prompt()?;
            
            let status = Command::new("journalctl")
                .arg("-n")
                .arg(&lines)
                .arg("--no-pager")
                .status()?;
            
            if !status.success() {
                println!("{}", "Failed to retrieve logs".red());
            }
        } else {
            // Fallback to traditional logs
            println!("Showing /var/log/syslog (last 50 lines)...\n");
            let _ = Command::new("tail")
                .arg("-n")
                .arg("50")
                .arg("/var/log/syslog")
                .status();
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        println!("Using log show...\n");
        let status = Command::new("log")
            .arg("show")
            .arg("--predicate")
            .arg("eventMessage contains \"error\" or eventMessage contains \"warning\"")
            .arg("--last")
            .arg("1h")
            .status()?;
        
        if !status.success() {
            println!("{}", "Failed to retrieve logs".red());
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        println!("Using Get-EventLog...\n");
        let status = Command::new("powershell")
            .arg("-Command")
            .arg("Get-EventLog -LogName System -Newest 50 | Format-Table -AutoSize")
            .status()?;
        
        if !status.success() {
            println!("{}", "Failed to retrieve logs".red());
        }
    }
    
    Ok(())
}

fn follow_logs() -> Result<()> {
    println!("\n{}", "Following logs (Ctrl+C to stop)...".cyan());
    
    #[cfg(target_os = "linux")]
    {
        if which("journalctl").is_ok() {
            let _ = Command::new("journalctl")
                .arg("-f")
                .status();
        } else {
            let _ = Command::new("tail")
                .arg("-f")
                .arg("/var/log/syslog")
                .status();
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("log")
            .arg("stream")
            .status();
    }
    
    #[cfg(target_os = "windows")]
    {
        println!("{}", "Live log following not supported on Windows via this tool.".yellow());
        println!("Use Event Viewer instead.");
    }
    
    Ok(())
}

fn show_system_logs() -> Result<()> {
    println!("\n{}", "System Logs".yellow().bold());
    
    #[cfg(target_os = "linux")]
    {
        if which("journalctl").is_ok() {
            let priority = Select::new("Priority level:", vec!["All", "Error", "Warning", "Info"])
                .prompt()?;
            
            let mut cmd = Command::new("journalctl");
            cmd.arg("-n").arg("100").arg("--no-pager");
            
            match priority {
                "Error" => { cmd.arg("-p").arg("err"); },
                "Warning" => { cmd.arg("-p").arg("warning"); },
                "Info" => { cmd.arg("-p").arg("info"); },
                _ => {},
            }
            
            let _ = cmd.status();
        }
    }
    
    Ok(())
}

fn show_service_logs() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        if which("journalctl").is_ok() {
            let service = Text::new("Service name (e.g., sshd, nginx):").prompt()?;
            
            println!("\n{} {}...", "Showing logs for".cyan(), service.yellow().bold());
            
            let lines = Text::new("Number of lines:")
                .with_default("50")
                .prompt()?;
            
            let status = Command::new("journalctl")
                .arg("-u")
                .arg(&service)
                .arg("-n")
                .arg(&lines)
                .arg("--no-pager")
                .status()?;
            
            if !status.success() {
                println!("{}", "Failed to retrieve service logs".red());
            }
        } else {
            println!("{}", "journalctl not available".red());
        }
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        println!("{}", "Service logs viewing is only supported on Linux with systemd".yellow());
    }
    
    Ok(())
}

fn show_kernel_logs() -> Result<()> {
    println!("\n{}", "Kernel Logs".yellow().bold());
    
    #[cfg(target_os = "linux")]
    {
        if which("dmesg").is_ok() {
            let follow = Confirm::new("Follow kernel logs?")
                .with_default(false)
                .prompt()?;
            
            let mut cmd = Command::new("dmesg");
            cmd.arg("--human").arg("--color=always");
            
            if follow {
                cmd.arg("--follow");
            }
            
            let _ = cmd.status();
        } else {
            println!("{}", "dmesg not available".red());
        }
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        println!("{}", "Kernel logs viewing is only supported on Linux".yellow());
    }
    
    Ok(())
}

fn show_auth_logs() -> Result<()> {
    println!("\n{}", "Authentication Logs".yellow().bold());
    
    #[cfg(target_os = "linux")]
    {
        if which("journalctl").is_ok() {
            let status = Command::new("journalctl")
                .arg("-u")
                .arg("ssh")
                .arg("-n")
                .arg("50")
                .arg("--no-pager")
                .status()?;
            
            if !status.success() {
                // Try auth.log
                let _ = Command::new("tail")
                    .arg("-n")
                    .arg("50")
                    .arg("/var/log/auth.log")
                    .status();
            }
        } else if std::path::Path::new("/var/log/auth.log").exists() {
            let _ = Command::new("tail")
                .arg("-n")
                .arg("50")
                .arg("/var/log/auth.log")
                .status();
        } else {
            println!("{}", "Authentication logs not found".red());
        }
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        println!("{}", "Authentication logs viewing is only supported on Linux".yellow());
    }
    
    Ok(())
}
