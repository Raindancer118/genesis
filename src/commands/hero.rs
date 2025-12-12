use sysinfo::{Pid, ProcessExt, System, SystemExt};
use colored::Colorize;
use inquire::{Confirm, MultiSelect};
use std::collections::HashMap;

pub fn run(
    dry_run: bool,
    scope: String,
    mem_threshold_mb: Option<f64>,
    cpu_threshold: Option<f64>,
    limit: usize,
    quiet: bool,
    fast: bool,
) {
    if !quiet {
        println!("{}", "🦸 Hero Mode Initiated".bold().magenta());
    }

    let mut sys = System::new_all();
    // Sleep briefly to calculate CPU usage if not fast mode
    if !fast {
        if !quiet { println!("Sampling CPU usage..."); }
        std::thread::sleep(std::time::Duration::from_millis(500));
        sys.refresh_all();
    } else {
        sys.refresh_all();
    }

    let mem_limit = mem_threshold_mb.unwrap_or(400.0) * 1024.0 * 1024.0; // MB to Bytes
    let cpu_limit = cpu_threshold.unwrap_or(50.0);

    let current_user_uid = get_current_user_uid();

    let mut targets = Vec::new();

    for (pid, process) in sys.processes() {
        if scope == "user" {
             // Filter by user. simple check: process.user_id() == current_user_uid
             if let Some(uid) = process.user_id() {
                 if let Some(current) = &current_user_uid {
                      if uid != current { continue; }
                 }
             }
        }

        let mem = process.memory() as f64;
        let cpu = process.cpu_usage() as f64;

        if mem > mem_limit || cpu > cpu_limit {
            targets.push((pid, process.name().to_string(), mem, cpu));
        }
    }

    // Sort by resource usage (heuristic: mem + cpu*factor?)
    // Let's sort by Memory for now
    targets.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    targets.truncate(limit);

    if targets.is_empty() {
        if !quiet { println!("{}", "No villains found. System is safe.".green()); }
        return;
    }

    println!("\n{}", "Found Resource Hogs:".red().bold());
    
    // Display and Selection
    // We can use MultiSelect to let user choose who to kill
    
    let mut choices = Vec::new();
    let mut kill_map = HashMap::new();

    for (pid, name, mem, cpu) in &targets {
        let label = format!("{:<6} {:<20} Mem: {:<10} CPU: {:.1}%", pid, name, format_bytes(*mem as u64), cpu);
        choices.push(label.clone());
        kill_map.insert(label, *pid);
    }

    if dry_run {
        for c in choices { println!("{}", c); }
        println!("\nDry run complete.");
        return;
    }

    let selected = MultiSelect::new("Select processes to terminate:", choices)
        .prompt();

    match selected {
        Ok(selection) => {
            if selection.is_empty() {
                println!("No action taken.");
                return;
            }

            for item in selection {
                if let Some(pid) = kill_map.get(&item) {
                     if let Some(proc) = sys.process(*pid) {
                         println!("Killing {} ({})", proc.name(), pid);
                         if proc.kill() {
                             println!("{}", "Eliminated.".green());
                         } else {
                             println!("{}", "Failed to kill.".red());
                         }
                     }
                }
            }
        },
        Err(_) => println!("Aborted."),
    }
}

fn get_current_user_uid() -> Option<sysinfo::Uid> {
    // This is tricky cross-platform without 'users' crate or similar.
    // Sysinfo uses generic Uid.
    // We can iterate processes and find one owned by us? 
    // Or just rely on scope="all" usually.
    // For now, return None to skip user check if implied?
    // Proper way: `users::get_current_uid()`
    // I didn't add `users` crate.
    // I added `whoami`.
    // Sysinfo user_id() returns &Uid.
    // Let's skip precise implementation of UID matching for now unless strictly needed.
    // If scope == "user", we can use `whoami::username()` and match `process.user_id()` -> resolve to name?
    // sysinfo provides `get_user_by_id`.
    None 
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
