use anyhow::Result;
use colored::Colorize;
use walkdir::WalkDir;
use std::path::Path;
use std::cmp::Reverse;

pub fn run(path: Option<String>) -> Result<()> {
    let target = path.unwrap_or_else(|| ".".to_string());
    println!("{} '{}'...", "💾 Analyzing storage usage in".cyan(), target);

    let mut total_size: u64 = 0;
    let mut file_count: u64 = 0;
    let mut files = Vec::new();

    for entry in WalkDir::new(&target).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let size = entry.metadata()?.len();
            total_size += size;
            file_count += 1;
            files.push((entry.path().to_string_lossy().to_string(), size));
        }
    }

    println!("Total Size: {}", format_bytes(total_size).bold());
    println!("File Count: {}", file_count);

    // Top 10 largest files
    files.sort_by_key(|k| Reverse(k.1));
    println!("\n{}", "Top 10 Largest Files:".yellow());
    for (path, size) in files.iter().take(10) {
        println!("{:<10} {}", format_bytes(*size), path);
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
    format!("{:.2} {}B", (bytes as f64) / div.powi(exp), pre)
}
