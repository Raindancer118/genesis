use anyhow::Result;
use colored::Colorize;
use std::time::{Duration, Instant};
use std::thread;
use inquire::{Select, Text};

pub fn run(mode: Option<String>, duration: Option<String>) -> Result<()> {
    println!("{}", "⏱️  Timer & Stopwatch".bold().magenta());
    
    let mode = match mode {
        Some(m) => m,
        None => {
            let options = vec!["Timer (Countdown)", "Stopwatch", "Pomodoro"];
            Select::new("Select mode:", options).prompt()?.to_string()
        }
    };
    
    match mode.as_str() {
        "Timer (Countdown)" | "timer" => {
            let duration_str = match duration {
                Some(d) => d,
                None => Text::new("Enter duration (e.g., 5m, 30s, 1h30m):").prompt()?,
            };
            run_timer(&duration_str)?;
        },
        "Stopwatch" | "stopwatch" => {
            run_stopwatch()?;
        },
        "Pomodoro" | "pomodoro" => {
            run_pomodoro()?;
        },
        _ => {
            println!("{}", "Unknown mode. Use 'timer', 'stopwatch', or 'pomodoro'".red());
        }
    }
    
    Ok(())
}

fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim().to_lowercase();
    let mut total_seconds = 0u64;
    let mut current_num = String::new();
    
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            current_num.push(ch);
        } else {
            if !current_num.is_empty() {
                let num: u64 = current_num.parse()?;
                match ch {
                    'h' => total_seconds += num * 3600,
                    'm' => total_seconds += num * 60,
                    's' => total_seconds += num,
                    _ => {},
                }
                current_num.clear();
            }
        }
    }
    
    if !current_num.is_empty() {
        // Default to seconds if no unit specified
        total_seconds += current_num.parse::<u64>()?;
    }
    
    if total_seconds == 0 {
        total_seconds = 60; // Default 1 minute
    }
    
    Ok(Duration::from_secs(total_seconds))
}

fn run_timer(duration_str: &str) -> Result<()> {
    let duration = parse_duration(duration_str)?;
    let total_secs = duration.as_secs();
    
    println!("\n{}", format!("Timer set for {} seconds", total_secs).cyan());
    println!("{}", "Press Ctrl+C to cancel".dimmed());
    
    let start = Instant::now();
    
    loop {
        let elapsed = start.elapsed();
        if elapsed >= duration {
            break;
        }
        
        let remaining = duration - elapsed;
        let secs = remaining.as_secs();
        let hours = secs / 3600;
        let minutes = (secs % 3600) / 60;
        let seconds = secs % 60;
        
        print!("\r{}", format!("⏱️  {:02}:{:02}:{:02}", hours, minutes, seconds).yellow().bold());
        use std::io::{self, Write};
        io::stdout().flush()?;
        
        thread::sleep(Duration::from_millis(100));
    }
    
    println!("\n\n{}", "⏰ TIME'S UP! ⏰".green().bold().on_black());
    
    // Try to play a sound (platform dependent)
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("paplay")
            .arg("/usr/share/sounds/freedesktop/stereo/complete.oga")
            .spawn();
    }
    
    Ok(())
}

fn run_stopwatch() -> Result<()> {
    println!("\n{}", "Stopwatch started. Press Enter to stop...".cyan());
    
    let start = Instant::now();
    
    // Spawn a thread to display time
    let handle = thread::spawn(move || {
        loop {
            let elapsed = start.elapsed();
            let total_secs = elapsed.as_secs();
            let hours = total_secs / 3600;
            let minutes = (total_secs % 3600) / 60;
            let seconds = total_secs % 60;
            let millis = elapsed.subsec_millis();
            
            print!("\r{}", format!("⏱️  {:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis).cyan().bold());
            use std::io::{self, Write};
            io::stdout().flush().unwrap();
            
            thread::sleep(Duration::from_millis(10));
        }
    });
    
    // Wait for Enter key
    use std::io::{self, BufRead};
    let stdin = io::stdin();
    let _ = stdin.lock().lines().next();
    
    // Stop the thread (in a real app, we'd use proper signaling)
    println!("\n\n{}", "Stopwatch stopped.".green());
    
    Ok(())
}

fn run_pomodoro() -> Result<()> {
    println!("\n{}", "🍅 Pomodoro Timer".bold().red());
    println!("Work session: 25 minutes");
    println!("Break: 5 minutes\n");
    
    let mut session = 1;
    
    loop {
        println!("{}", format!("Session #{}", session).cyan().bold());
        println!("Starting work session (25 minutes)...");
        
        if let Err(_) = run_timer("25m") {
            break;
        }
        
        println!("\n{}", "Work session complete! Time for a break.".green().bold());
        
        let continue_choice = inquire::Confirm::new("Take a 5-minute break?")
            .with_default(true)
            .prompt()?;
        
        if !continue_choice {
            break;
        }
        
        println!("Starting break (5 minutes)...");
        if let Err(_) = run_timer("5m") {
            break;
        }
        
        println!("\n{}", "Break complete!".green().bold());
        
        let continue_choice = inquire::Confirm::new("Start another session?")
            .with_default(true)
            .prompt()?;
        
        if !continue_choice {
            break;
        }
        
        session += 1;
    }
    
    println!("\n{}", format!("Completed {} Pomodoro session(s)!", session).green().bold());
    
    Ok(())
}
