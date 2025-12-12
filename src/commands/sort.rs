use std::fs;
use std::path::Path;
use anyhow::{Result, Context};
use colored::Colorize;
use std::collections::HashMap;

pub fn run(path: String) -> Result<()> {
    let target_dir = Path::new(&path);
    if !target_dir.exists() {
        return Err(anyhow::anyhow!("Directory '{}' does not exist.", path));
    }

    println!("{} '{}'...", "📂 Sorting files in".cyan(), path);

    let entries = fs::read_dir(target_dir).context("Failed to read directory")?;

    let mut moved_count = 0;
    
    // Define categories or just extension-based?
    // Python version usually did extension based or smart categorization.
    // Let's implement extension-based sorting first.
    
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() { continue; }
        
        // Skip hidden files
        if path.file_name().unwrap_or_default().to_string_lossy().starts_with('.') { continue; }

        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_string().to_lowercase();
            
            // Skip sorting the binary itself or sensitive files if needed?
            // Usually safe to move.
            
            let subdir_name = ext_str;
            let subdir_path = target_dir.join(&subdir_name);
            
            if !subdir_path.exists() {
                fs::create_dir(&subdir_path)?;
            }
            
            let file_name = path.file_name().unwrap();
            let dest_path = subdir_path.join(file_name);
            
            fs::rename(&path, &dest_path)?;
            println!("Moved {} -> {}/", file_name.to_string_lossy(), subdir_name);
            moved_count += 1;
        }
    }

    if moved_count > 0 {
        println!("{}", format!("✅ Successfully sorted {} files.", moved_count).green());
    } else {
        println!("No files to sort.");
    }

    Ok(())
}
