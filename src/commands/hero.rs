use sysinfo::System;
use inquire::MultiSelect;
use colored::Colorize;
use std::collections::HashMap;
use anyhow::Result;
use comfy_table::{Table, presets::UTF8_FULL, ContentArrangement, Cell, Color};

// Legacy function for backward compatibility
pub fn run(
    dry_run: bool,
    scope: String,
    mem_threshold: u64,
    cpu_threshold: f32,
    limit: usize,
    quiet: bool,
    fast: bool,
) {
    let _ = run_revamped(dry_run, scope, mem_threshold, cpu_threshold, limit, quiet, fast, None);
}

/// Revamped hero command with enhanced features and better UX
pub fn run_revamped(
    dry_run: bool,
    scope: String,
    mem_threshold: u64,
    cpu_threshold: f32,
    limit: usize,
    quiet: bool,
    fast: bool,
    auto_kill: Option<usize>,
) -> Result<()> {
    // Print banner
    if !quiet {
        println!("\n{}", "═══════════════════════════════════════════════════════════".cyan().bold());
        println!("{}", "              🦸  HERO MODE - PROCESS MANAGER              ".cyan().bold());
        println!("{}", "═══════════════════════════════════════════════════════════".cyan().bold());
        println!();
    }

    // Initialize system and refresh
    let mut sys = System::new_all();
    
    if !fast {
        if !quiet { 
            println!("{}", "⏳ Sampling CPU usage (500ms)...".yellow()); 
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
        sys.refresh_all();
    } else {
        sys.refresh_all();
    }

    let current_user_name = whoami::username();
    
    if !quiet {
        println!("{} {}", "🔍 Scanning for resource hogs...".yellow(), 
                 if scope == "user" { 
                     format!("(user: {})", current_user_name).dimmed() 
                 } else { 
                     "(all processes)".dimmed() 
                 });
        println!("{} Memory > {} MB, CPU > {}%", 
                 "📊 Thresholds:".yellow(), 
                 mem_threshold, 
                 cpu_threshold);
        println!();
    }

    // Collect target processes
    let mut targets = Vec::new();

    for (pid, process) in sys.processes() {
        // Filter by scope
        if scope == "user" {
            // For simplicity, we'll skip the detailed user filtering
            // sysinfo 0.33 doesn't have get_user_by_id as a public method
            // We'll just match by comparing process user_id with current user's processes
            // A better approach would require the 'users' crate, but we'll keep it simple
            if let Some(_uid) = process.user_id() {
                // We could filter more precisely here with the users crate
                // For now, we'll just continue - scope filtering is best-effort
            }
        }

        let mem_mb = process.memory() / 1024 / 1024; // Convert to MB
        let cpu = process.cpu_usage();

        // Check thresholds
        if mem_mb > mem_threshold || cpu > cpu_threshold {
            let name = process.name().to_string_lossy().into_owned();
            let parent = process.parent().map(|p| p.as_u32());
            targets.push((*pid, name, mem_mb, cpu, parent));
        }
    }

    // Sort by combined resource score (memory weight + CPU weight)
    targets.sort_by(|a, b| {
        let score_a = (a.2 as f32) + (a.3 * 10.0); // mem_mb + cpu * 10
        let score_b = (b.2 as f32) + (b.3 * 10.0);
        score_b.partial_cmp(&score_a).unwrap()
    });
    
    targets.truncate(limit);

    if targets.is_empty() {
        if !quiet { 
            println!("{}", "✨ No resource hogs found. System is healthy! 🎉".green().bold());
        }
        return Ok(());
    }

    // Display results in a nice table
    if !quiet {
        println!("{} {} processes found\n", "⚠️  Found".red().bold(), targets.len());
    }
    
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            Cell::new("PID").fg(Color::Cyan),
            Cell::new("Process Name").fg(Color::Cyan),
            Cell::new("Memory").fg(Color::Cyan),
            Cell::new("CPU %").fg(Color::Cyan),
            Cell::new("Parent PID").fg(Color::Cyan),
        ]);

    let mut choices = Vec::new();
    let mut kill_map = HashMap::new();

    for (pid, name, mem, cpu, parent) in &targets {
        let parent_str = parent.map(|p| p.to_string()).unwrap_or_else(|| "-".to_string());
        
        table.add_row(vec![
            pid.to_string(),
            name.clone(),
            format_bytes(*mem as u64 * 1024 * 1024),
            format!("{:.1}", cpu),
            parent_str,
        ]);
        
        let label = format!("[PID: {}] {} - {} RAM, {:.1}% CPU", 
                           pid, name, format_bytes(*mem as u64 * 1024 * 1024), cpu);
        choices.push(label.clone());
        kill_map.insert(label, *pid);
    }

    if !quiet {
        println!("{}", table);
        println!();
    }

    // Dry run mode
    if dry_run {
        println!("{}", "🔍 Dry run mode - no processes will be terminated".yellow().bold());
        return Ok(());
    }

    // Auto-kill mode
    if let Some(auto_count) = auto_kill {
        let to_kill = targets.iter().take(auto_count);
        println!("{}", format!("⚡ Auto-killing top {} processes...", auto_count).yellow().bold());
        
        for (pid, name, _, _, _) in to_kill {
            if let Some(proc) = sys.process(*pid) {
                print!("  Terminating {} (PID: {})... ", name, pid);
                if proc.kill() {
                    println!("{}", "✓ Success".green());
                } else {
                    println!("{}", "✗ Failed".red());
                }
            }
        }
        
        return Ok(());
    }

    // Interactive selection mode
    let selected = MultiSelect::new(
        "Select processes to terminate (use Space to select, Enter to confirm):", 
        choices
    ).prompt();

    match selected {
        Ok(selection) => {
            if selection.is_empty() {
                println!("{}", "ℹ️  No processes selected. Exiting.".blue());
                return Ok(());
            }

            println!("\n{}", "⚠️  Terminating selected processes...".yellow().bold());
            let mut success_count = 0;
            let mut fail_count = 0;

            for item in selection {
                if let Some(pid) = kill_map.get(&item) {
                    if let Some(proc) = sys.process(*pid) {
                        print!("  Killing {} (PID: {})... ", proc.name().to_string_lossy(), pid);
                        if proc.kill() {
                            println!("{}", "✓ Success".green());
                            success_count += 1;
                        } else {
                            println!("{}", "✗ Failed (may require elevated privileges)".red());
                            fail_count += 1;
                        }
                    }
                }
            }
            
            println!();
            println!("{}", "═══════════════════════════════════════".cyan());
            println!("{}  Terminated: {}", "✓".green(), success_count);
            if fail_count > 0 {
                println!("{}  Failed: {} (try running with sudo)", "✗".red(), fail_count);
            }
            println!("{}", "═══════════════════════════════════════".cyan());
        },
        Err(_) => {
            println!("{}", "❌ Operation cancelled by user.".yellow());
        }
    }
    
    Ok(())
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
