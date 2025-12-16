use anyhow::Result;
use colored::Colorize;
use std::process::Command;
use std::net::{TcpStream, IpAddr};
use std::time::Duration;
use inquire::{Text, Select};
use which::which;

pub fn run(action: Option<String>) -> Result<()> {
    println!("{}", "🌐 Network Diagnostics".bold().cyan());
    
    let action = match action {
        Some(a) => a,
        None => {
            let options = vec![
                "Network Info",
                "Ping Host",
                "Port Scan",
                "DNS Lookup",
                "Trace Route",
                "Speed Test",
            ];
            Select::new("Select action:", options).prompt()?.to_string()
        }
    };
    
    match action.as_str() {
        "Network Info" | "info" => show_network_info()?,
        "Ping Host" | "ping" => ping_host()?,
        "Port Scan" | "scan" | "ports" => scan_ports()?,
        "DNS Lookup" | "dns" | "lookup" => dns_lookup()?,
        "Trace Route" | "trace" | "traceroute" => trace_route()?,
        "Speed Test" | "speed" | "speedtest" => speed_test()?,
        _ => println!("{}", "Unknown action".red()),
    }
    
    Ok(())
}

fn show_network_info() -> Result<()> {
    println!("\n{}", "Network Interface Information".bold().yellow());
    
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("ip")
            .arg("addr")
            .output()?;
        
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
        
        println!("\n{}", "Routing Table".bold().yellow());
        let output = Command::new("ip")
            .arg("route")
            .output()?;
        
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("ipconfig")
            .arg("/all")
            .output()?;
        
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("ifconfig")
            .output()?;
        
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }
    
    Ok(())
}

fn ping_host() -> Result<()> {
    let host = Text::new("Enter host to ping (e.g., google.com):").prompt()?;
    let count = Text::new("Number of pings:")
        .with_default("4")
        .prompt()?;
    
    println!("\n{} {}...", "Pinging".cyan(), host.yellow().bold());
    
    #[cfg(target_os = "windows")]
    let output = Command::new("ping")
        .arg("-n")
        .arg(&count)
        .arg(&host)
        .output()?;
    
    #[cfg(not(target_os = "windows"))]
    let output = Command::new("ping")
        .arg("-c")
        .arg(&count)
        .arg(&host)
        .output()?;
    
    if output.status.success() {
        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        println!("{}", String::from_utf8_lossy(&output.stderr).red());
    }
    
    Ok(())
}

fn scan_ports() -> Result<()> {
    let host = Text::new("Enter host to scan:").prompt()?;
    let start_port: u16 = Text::new("Start port:")
        .with_default("1")
        .prompt()?
        .parse()?;
    let end_port: u16 = Text::new("End port:")
        .with_default("1024")
        .prompt()?
        .parse()?;
    
    println!("\n{} {} (ports {}-{})...", "Scanning".cyan(), host.yellow().bold(), start_port, end_port);
    
    let mut open_ports = Vec::new();
    
    for port in start_port..=end_port {
        let addr = format!("{}:{}", host, port);
        
        if let Ok(_) = TcpStream::connect_timeout(
            &addr.parse().unwrap_or_else(|_| format!("{}:{}", host, port).parse().unwrap()),
            Duration::from_millis(200)
        ) {
            open_ports.push(port);
            println!("{} {}: {}", "✓".green(), "Open".green().bold(), port);
        }
        
        // Progress indicator every 100 ports
        if port % 100 == 0 {
            print!(".");
            use std::io::{self, Write};
            io::stdout().flush()?;
        }
    }
    
    println!("\n");
    
    if open_ports.is_empty() {
        println!("{}", "No open ports found in the specified range.".yellow());
    } else {
        println!("{} {}", "Open ports:".green().bold(), open_ports.len());
        for port in &open_ports {
            println!("  - {}", port);
        }
    }
    
    Ok(())
}

fn dns_lookup() -> Result<()> {
    let host = Text::new("Enter hostname to lookup:").prompt()?;
    
    println!("\n{} {}...", "Looking up".cyan(), host.yellow().bold());
    
    if which("nslookup").is_ok() {
        let output = Command::new("nslookup")
            .arg(&host)
            .output()?;
        
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        } else {
            println!("{}", String::from_utf8_lossy(&output.stderr).red());
        }
    } else if which("dig").is_ok() {
        let output = Command::new("dig")
            .arg(&host)
            .output()?;
        
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        } else {
            println!("{}", String::from_utf8_lossy(&output.stderr).red());
        }
    } else if which("host").is_ok() {
        let output = Command::new("host")
            .arg(&host)
            .output()?;
        
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        } else {
            println!("{}", String::from_utf8_lossy(&output.stderr).red());
        }
    } else {
        println!("{}", "No DNS lookup tool found (nslookup, dig, or host)".red());
    }
    
    Ok(())
}

fn trace_route() -> Result<()> {
    let host = Text::new("Enter host to trace:").prompt()?;
    
    println!("\n{} {}...", "Tracing route to".cyan(), host.yellow().bold());
    
    #[cfg(target_os = "windows")]
    let cmd = "tracert";
    
    #[cfg(not(target_os = "windows"))]
    let cmd = "traceroute";
    
    if which(cmd).is_ok() {
        let output = Command::new(cmd)
            .arg(&host)
            .output()?;
        
        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
        } else {
            println!("{}", String::from_utf8_lossy(&output.stderr).red());
        }
    } else {
        println!("{} {} is not available", "Error:".red(), cmd);
    }
    
    Ok(())
}

fn speed_test() -> Result<()> {
    println!("\n{}", "Running speed test...".cyan());
    
    // Check for speedtest-cli
    if which("speedtest-cli").is_ok() {
        println!("Using speedtest-cli...\n");
        let status = Command::new("speedtest-cli")
            .arg("--simple")
            .status()?;
        
        if !status.success() {
            println!("{}", "Speed test failed".red());
        }
    } else if which("speedtest").is_ok() {
        println!("Using speedtest...\n");
        let status = Command::new("speedtest")
            .status()?;
        
        if !status.success() {
            println!("{}", "Speed test failed".red());
        }
    } else {
        println!("{}", "Speed test tool not found.".yellow());
        println!("Install one of the following:");
        println!("  - speedtest-cli: pip install speedtest-cli");
        println!("  - speedtest: https://www.speedtest.net/apps/cli");
    }
    
    Ok(())
}
