use anyhow::Result;
use colored::Colorize;
use sysinfo::System;
use std::time::{Duration, Instant};
use std::thread;
use comfy_table::{Table, presets::UTF8_FULL};

pub fn run() -> Result<()> {
    println!("{}", "⚡ System Benchmark".bold().magenta());
    println!("{}", "Running comprehensive system performance tests...\n".cyan());
    
    // CPU Benchmark
    println!("{}", "1. CPU Performance Test".yellow().bold());
    let cpu_score = benchmark_cpu()?;
    println!("{}: {} ops/sec\n", "CPU Score".bold(), cpu_score.to_string().green().bold());
    
    // Memory Benchmark
    println!("{}", "2. Memory Performance Test".yellow().bold());
    let mem_score = benchmark_memory()?;
    println!("{}: {} MB/s\n", "Memory Score".bold(), mem_score.to_string().green().bold());
    
    // Disk I/O Benchmark
    println!("{}", "3. Disk I/O Performance Test".yellow().bold());
    let disk_score = benchmark_disk()?;
    println!("{}: {} MB/s\n", "Disk Score".bold(), disk_score.to_string().green().bold());
    
    // System Info
    println!("{}", "4. System Information".yellow().bold());
    display_system_info()?;
    
    // Summary
    println!("\n{}", "═".repeat(60).cyan());
    println!("{}", "BENCHMARK SUMMARY".bold().cyan());
    println!("{}", "═".repeat(60).cyan());
    
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Component", "Score", "Rating"]);
    
    table.add_row(vec![
        "CPU".to_string(),
        format!("{} ops/sec", cpu_score),
        rate_performance(cpu_score as f64, 100000.0),
    ]);
    
    table.add_row(vec![
        "Memory".to_string(),
        format!("{} MB/s", mem_score),
        rate_performance(mem_score as f64, 1000.0),
    ]);
    
    table.add_row(vec![
        "Disk I/O".to_string(),
        format!("{} MB/s", disk_score),
        rate_performance(disk_score as f64, 100.0),
    ]);
    
    println!("{}", table);
    println!("{}", "═".repeat(60).cyan());
    
    Ok(())
}

fn benchmark_cpu() -> Result<u64> {
    println!("Testing CPU with prime number calculation...");
    
    let start = Instant::now();
    let duration = Duration::from_secs(2);
    let mut operations = 0u64;
    
    while start.elapsed() < duration {
        // Simple prime check for numbers up to 10000
        for n in 2..10000 {
            if is_prime(n) {
                operations += 1;
            }
        }
    }
    
    let ops_per_sec = (operations as f64 / start.elapsed().as_secs_f64()) as u64;
    
    Ok(ops_per_sec)
}

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }
    
    let limit = (n as f64).sqrt() as u64;
    for i in (3..=limit).step_by(2) {
        if n % i == 0 {
            return false;
        }
    }
    
    true
}

fn benchmark_memory() -> Result<u64> {
    println!("Testing memory with array operations...");
    
    let size = 10_000_000; // 10 million elements
    let start = Instant::now();
    
    // Allocate and fill array
    let mut data: Vec<u64> = Vec::with_capacity(size);
    for i in 0..size {
        data.push(i as u64);
    }
    
    // Perform operations
    let sum: u64 = data.iter().sum();
    let _ = sum; // Use the result
    
    // Calculate throughput
    let elapsed = start.elapsed().as_secs_f64();
    let bytes_processed = (size * std::mem::size_of::<u64>()) as f64;
    let mb_per_sec = (bytes_processed / (1024.0 * 1024.0)) / elapsed;
    
    Ok(mb_per_sec as u64)
}

fn benchmark_disk() -> Result<u64> {
    println!("Testing disk I/O with file operations...");
    
    use std::fs::File;
    use std::io::Write;
    
    let test_file = "/tmp/genesis_benchmark_test.dat";
    let data_size = 10 * 1024 * 1024; // 10 MB
    let data = vec![0u8; data_size];
    
    // Write test
    let start = Instant::now();
    {
        let mut file = File::create(test_file)?;
        file.write_all(&data)?;
        file.sync_all()?;
    }
    let write_time = start.elapsed().as_secs_f64();
    
    // Read test
    let start = Instant::now();
    {
        let _ = std::fs::read(test_file)?;
    }
    let read_time = start.elapsed().as_secs_f64();
    
    // Cleanup
    let _ = std::fs::remove_file(test_file);
    
    // Calculate average throughput
    let mb_size = data_size as f64 / (1024.0 * 1024.0);
    let avg_time = (write_time + read_time) / 2.0;
    let mb_per_sec = mb_size / avg_time;
    
    Ok(mb_per_sec as u64)
}

fn display_system_info() -> Result<()> {
    let mut sys = System::new_all();
    sys.refresh_all();
    
    println!("{}: {}", "OS".bold(), System::name().unwrap_or("Unknown".to_string()));
    println!("{}: {}", "Kernel".bold(), System::kernel_version().unwrap_or("Unknown".to_string()));
    
    let total_mem = sys.total_memory() / 1024 / 1024;
    println!("{}: {} MB", "Total Memory".bold(), total_mem);
    
    println!("{}: {}", "CPU Cores".bold(), sys.cpus().len());
    
    if let Some(cpu) = sys.cpus().first() {
        println!("{}: {}", "CPU Brand".bold(), cpu.brand());
    }
    
    Ok(())
}

fn rate_performance(score: f64, baseline: f64) -> String {
    let ratio = score / baseline;
    
    if ratio >= 2.0 {
        "Excellent ⭐⭐⭐⭐⭐".green().to_string()
    } else if ratio >= 1.5 {
        "Very Good ⭐⭐⭐⭐".green().to_string()
    } else if ratio >= 1.0 {
        "Good ⭐⭐⭐".cyan().to_string()
    } else if ratio >= 0.5 {
        "Average ⭐⭐".yellow().to_string()
    } else {
        "Below Average ⭐".red().to_string()
    }
}
