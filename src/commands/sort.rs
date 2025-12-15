use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use colored::Colorize;
use std::collections::HashMap;
use inquire::{Select, Confirm};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use directories::ProjectDirs;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SortOperation {
    timestamp: DateTime<Utc>,
    base_dir: PathBuf,
    moves: Vec<FileMove>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileMove {
    from: PathBuf,
    to: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SortHistory {
    operations: Vec<SortOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LearningData {
    extension_categories: HashMap<String, String>,
}

impl SortHistory {
    fn load() -> Result<Self> {
        let history_path = Self::get_history_path()?;
        if history_path.exists() {
            let content = fs::read_to_string(&history_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(SortHistory { operations: Vec::new() })
        }
    }

    fn save(&self) -> Result<()> {
        let history_path = Self::get_history_path()?;
        if let Some(parent) = history_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&history_path, content)?;
        Ok(())
    }

    fn add_operation(&mut self, operation: SortOperation) {
        self.operations.push(operation);
        // Keep only last 10 operations
        if self.operations.len() > 10 {
            self.operations.remove(0);
        }
    }

    fn get_history_path() -> Result<PathBuf> {
        let config_dir = if let Some(proj_dirs) = ProjectDirs::from("", "", "genesis") {
            proj_dirs.data_dir().to_path_buf()
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".local/share/genesis")
        };
        Ok(config_dir.join("sort_history.json"))
    }
}

impl LearningData {
    fn load() -> Result<Self> {
        let learning_path = Self::get_learning_path()?;
        if learning_path.exists() {
            let content = fs::read_to_string(&learning_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(LearningData { extension_categories: HashMap::new() })
        }
    }

    fn save(&self) -> Result<()> {
        let learning_path = Self::get_learning_path()?;
        if let Some(parent) = learning_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&learning_path, content)?;
        Ok(())
    }

    fn get_learning_path() -> Result<PathBuf> {
        let config_dir = if let Some(proj_dirs) = ProjectDirs::from("", "", "genesis") {
            proj_dirs.data_dir().to_path_buf()
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".local/share/genesis")
        };
        Ok(config_dir.join("sort_learning.json"))
    }
}

#[derive(Debug, Clone, Copy)]
enum SortStrategy {
    ByExtension,
    ByCategory,
    ByDate,
    BySize,
    Manual,
    Smart,
}

pub fn run(path: String) -> Result<()> {
    let target_dir = Path::new(&path);
    if !target_dir.exists() {
        return Err(anyhow::anyhow!("Directory '{}' does not exist.", path));
    }

    println!("{} '{}'...", "📂 File Sorter".cyan().bold(), path);
    println!();

    // Check if user wants to undo last operation
    let mut history = SortHistory::load().unwrap_or_else(|_| SortHistory { operations: Vec::new() });
    
    if !history.operations.is_empty() {
        let last_op = &history.operations[history.operations.len() - 1];
        let elapsed = Utc::now().signed_duration_since(last_op.timestamp);
        
        if elapsed.num_seconds() < 300 { // 5 minutes
            let undo = Confirm::new(&format!(
                "A sort operation was performed {} seconds ago. Undo it?",
                elapsed.num_seconds()
            ))
            .with_default(false)
            .prompt();

            if let Ok(true) = undo {
                undo_last_operation(&mut history)?;
                return Ok(());
            }
        }
    }

    // Select sorting strategy
    let strategy_options = vec![
        "By Extension (group by file type)",
        "By Category (documents, images, videos, etc.)",
        "By Date Modified",
        "By Size (small, medium, large)",
        "Manual (you choose category for each file)",
        "Smart (learn from your previous choices)",
    ];

    let strategy_choice = Select::new("Choose sorting strategy:", strategy_options.clone())
        .prompt()
        .context("Failed to get user input")?;

    let strategy = match strategy_choice {
        s if s.starts_with("By Extension") => SortStrategy::ByExtension,
        s if s.starts_with("By Category") => SortStrategy::ByCategory,
        s if s.starts_with("By Date") => SortStrategy::ByDate,
        s if s.starts_with("By Size") => SortStrategy::BySize,
        s if s.starts_with("Manual") => SortStrategy::Manual,
        s if s.starts_with("Smart") => SortStrategy::Smart,
        _ => SortStrategy::ByExtension,
    };

    // Perform sorting based on strategy
    match strategy {
        SortStrategy::ByExtension => sort_by_extension(target_dir, &mut history)?,
        SortStrategy::ByCategory => sort_by_category(target_dir, &mut history)?,
        SortStrategy::ByDate => sort_by_date(target_dir, &mut history)?,
        SortStrategy::BySize => sort_by_size(target_dir, &mut history)?,
        SortStrategy::Manual => sort_manual(target_dir, &mut history)?,
        SortStrategy::Smart => sort_smart(target_dir, &mut history)?,
    }

    Ok(())
}

fn sort_by_extension(target_dir: &Path, history: &mut SortHistory) -> Result<()> {
    println!("\n{}", "Sorting by extension...".yellow());
    
    let files = collect_files(target_dir)?;
    if files.is_empty() {
        println!("No files to sort.");
        return Ok(());
    }

    // Preview
    preview_sort(&files, |f| {
        f.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("no_extension")
            .to_lowercase()
    })?;

    if !confirm_operation()? {
        println!("Operation cancelled.");
        return Ok(());
    }

    let mut operation = SortOperation {
        timestamp: Utc::now(),
        base_dir: target_dir.to_path_buf(),
        moves: Vec::new(),
    };

    for file_path in files {
        if let Some(ext) = file_path.extension() {
            let ext_str = ext.to_string_lossy().to_string().to_lowercase();
            let dest_dir = target_dir.join(&ext_str);
            
            fs::create_dir_all(&dest_dir)?;
            
            let file_name = file_path.file_name().unwrap();
            let dest_path = dest_dir.join(file_name);
            
            operation.moves.push(FileMove {
                from: file_path.clone(),
                to: dest_path.clone(),
            });
            
            fs::rename(&file_path, &dest_path)?;
            println!("  {} -> {}/", file_name.to_string_lossy().green(), ext_str);
        }
    }

    history.add_operation(operation);
    history.save()?;
    
    println!("\n{}", format!("✅ Successfully sorted {} files.", history.operations.last().unwrap().moves.len()).green().bold());
    Ok(())
}

fn sort_by_category(target_dir: &Path, history: &mut SortHistory) -> Result<()> {
    println!("\n{}", "Sorting by category...".yellow());
    
    let files = collect_files(target_dir)?;
    if files.is_empty() {
        println!("No files to sort.");
        return Ok(());
    }

    // Preview
    preview_sort(&files, |f| get_category(f).to_string())?;

    if !confirm_operation()? {
        println!("Operation cancelled.");
        return Ok(());
    }

    let mut operation = SortOperation {
        timestamp: Utc::now(),
        base_dir: target_dir.to_path_buf(),
        moves: Vec::new(),
    };

    for file_path in files {
        let category = get_category(&file_path);
        let dest_dir = target_dir.join(category);
        
        fs::create_dir_all(&dest_dir)?;
        
        let file_name = file_path.file_name().unwrap();
        let dest_path = dest_dir.join(file_name);
        
        operation.moves.push(FileMove {
            from: file_path.clone(),
            to: dest_path.clone(),
        });
        
        fs::rename(&file_path, &dest_path)?;
        println!("  {} -> {}/", file_name.to_string_lossy().green(), category);
    }

    history.add_operation(operation);
    history.save()?;
    
    println!("\n{}", format!("✅ Successfully sorted {} files.", history.operations.last().unwrap().moves.len()).green().bold());
    Ok(())
}

fn sort_by_date(target_dir: &Path, history: &mut SortHistory) -> Result<()> {
    println!("\n{}", "Sorting by date modified...".yellow());
    
    let files = collect_files(target_dir)?;
    if files.is_empty() {
        println!("No files to sort.");
        return Ok(());
    }

    // Preview
    preview_sort(&files, |f| {
        fs::metadata(f)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| {
                let datetime: DateTime<Utc> = t.into();
                Some(datetime.format("%Y-%m").to_string())
            })
            .unwrap_or_else(|| "unknown".to_string())
    })?;

    if !confirm_operation()? {
        println!("Operation cancelled.");
        return Ok(());
    }

    let mut operation = SortOperation {
        timestamp: Utc::now(),
        base_dir: target_dir.to_path_buf(),
        moves: Vec::new(),
    };

    for file_path in files {
        let date_folder = fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| {
                let datetime: DateTime<Utc> = t.into();
                Some(datetime.format("%Y-%m").to_string())
            })
            .unwrap_or_else(|| "unknown".to_string());
        
        let dest_dir = target_dir.join(&date_folder);
        fs::create_dir_all(&dest_dir)?;
        
        let file_name = file_path.file_name().unwrap();
        let dest_path = dest_dir.join(file_name);
        
        operation.moves.push(FileMove {
            from: file_path.clone(),
            to: dest_path.clone(),
        });
        
        fs::rename(&file_path, &dest_path)?;
        println!("  {} -> {}/", file_name.to_string_lossy().green(), date_folder);
    }

    history.add_operation(operation);
    history.save()?;
    
    println!("\n{}", format!("✅ Successfully sorted {} files.", history.operations.last().unwrap().moves.len()).green().bold());
    Ok(())
}

fn sort_by_size(target_dir: &Path, history: &mut SortHistory) -> Result<()> {
    println!("\n{}", "Sorting by size...".yellow());
    
    let files = collect_files(target_dir)?;
    if files.is_empty() {
        println!("No files to sort.");
        return Ok(());
    }

    // Preview
    preview_sort(&files, |f| {
        fs::metadata(f)
            .ok()
            .map(|m| {
                let size = m.len();
                if size < 1_000_000 {
                    "small"
                } else if size < 100_000_000 {
                    "medium"
                } else {
                    "large"
                }
            })
            .unwrap_or("unknown")
            .to_string()
    })?;

    if !confirm_operation()? {
        println!("Operation cancelled.");
        return Ok(());
    }

    let mut operation = SortOperation {
        timestamp: Utc::now(),
        base_dir: target_dir.to_path_buf(),
        moves: Vec::new(),
    };

    for file_path in files {
        let size_category = fs::metadata(&file_path)
            .ok()
            .map(|m| {
                let size = m.len();
                if size < 1_000_000 {
                    "small"
                } else if size < 100_000_000 {
                    "medium"
                } else {
                    "large"
                }
            })
            .unwrap_or("unknown");
        
        let dest_dir = target_dir.join(size_category);
        fs::create_dir_all(&dest_dir)?;
        
        let file_name = file_path.file_name().unwrap();
        let dest_path = dest_dir.join(file_name);
        
        operation.moves.push(FileMove {
            from: file_path.clone(),
            to: dest_path.clone(),
        });
        
        fs::rename(&file_path, &dest_path)?;
        println!("  {} -> {}/", file_name.to_string_lossy().green(), size_category);
    }

    history.add_operation(operation);
    history.save()?;
    
    println!("\n{}", format!("✅ Successfully sorted {} files.", history.operations.last().unwrap().moves.len()).green().bold());
    Ok(())
}

fn sort_manual(target_dir: &Path, history: &mut SortHistory) -> Result<()> {
    println!("\n{}", "Manual sorting mode - You choose the category for each file".yellow());
    println!("{}", "The system will learn from your choices!".cyan());
    
    let files = collect_files(target_dir)?;
    if files.is_empty() {
        println!("No files to sort.");
        return Ok(());
    }

    let mut learning_data = LearningData::load().unwrap_or_else(|_| LearningData {
        extension_categories: HashMap::new(),
    });

    let categories = vec![
        "Documents", "Images", "Videos", "Audio", "Archives",
        "Code", "Data", "Executables", "Other", "Skip",
    ];

    let mut operation = SortOperation {
        timestamp: Utc::now(),
        base_dir: target_dir.to_path_buf(),
        moves: Vec::new(),
    };

    for file_path in files {
        let file_name = file_path.file_name().unwrap().to_string_lossy();
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Check if we have a learned category
        let suggested_idx = learning_data.extension_categories.get(&ext)
            .and_then(|cat| categories.iter().position(|c| c.to_lowercase() == cat.to_lowercase()));

        let prompt = if let Some(idx) = suggested_idx {
            format!("📄 {} - Suggested: {}", file_name, categories[idx])
        } else {
            format!("📄 {}", file_name)
        };

        let choice = Select::new(&prompt, categories.clone())
            .with_starting_cursor(suggested_idx.unwrap_or(0))
            .prompt()?;

        if choice == "Skip" {
            continue;
        }

        // Learn from user choice
        if !ext.is_empty() {
            learning_data.extension_categories.insert(ext.clone(), choice.to_string());
        }

        let dest_dir = target_dir.join(choice);
        fs::create_dir_all(&dest_dir)?;
        
        let dest_path = dest_dir.join(file_path.file_name().unwrap());
        
        operation.moves.push(FileMove {
            from: file_path.clone(),
            to: dest_path.clone(),
        });
        
        fs::rename(&file_path, &dest_path)?;
        println!("  {} -> {}/", file_name.green(), choice);
    }

    learning_data.save()?;
    history.add_operation(operation);
    history.save()?;
    
    println!("\n{}", format!("✅ Successfully sorted {} files.", history.operations.last().unwrap().moves.len()).green().bold());
    println!("{}", "Your choices have been saved for future smart sorting!".cyan());
    Ok(())
}

fn sort_smart(target_dir: &Path, history: &mut SortHistory) -> Result<()> {
    println!("\n{}", "Smart sorting using learned patterns...".yellow());
    
    let files = collect_files(target_dir)?;
    if files.is_empty() {
        println!("No files to sort.");
        return Ok(());
    }

    let learning_data = LearningData::load().unwrap_or_else(|_| LearningData {
        extension_categories: HashMap::new(),
    });

    if learning_data.extension_categories.is_empty() {
        println!("{}", "No learned patterns found. Use Manual sorting first to teach the system!".red());
        return Ok(());
    }

    println!("{}", format!("Using {} learned patterns", learning_data.extension_categories.len()).cyan());

    let mut operation = SortOperation {
        timestamp: Utc::now(),
        base_dir: target_dir.to_path_buf(),
        moves: Vec::new(),
    };

    let mut unknown_files = Vec::new();

    for file_path in files {
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        if let Some(category) = learning_data.extension_categories.get(&ext) {
            let dest_dir = target_dir.join(category);
            fs::create_dir_all(&dest_dir)?;
            
            let file_name = file_path.file_name().unwrap();
            let dest_path = dest_dir.join(file_name);
            
            operation.moves.push(FileMove {
                from: file_path.clone(),
                to: dest_path.clone(),
            });
            
            fs::rename(&file_path, &dest_path)?;
            println!("  {} -> {}/", file_name.to_string_lossy().green(), category);
        } else {
            unknown_files.push(file_path);
        }
    }

    // Handle unknown files interactively
    if !unknown_files.is_empty() {
        println!("\n{}", format!("Found {} files with unknown extensions", unknown_files.len()).yellow());
        
        let handle_unknown = Confirm::new("Would you like to categorize them now?")
            .with_default(true)
            .prompt()?;

        if handle_unknown {
            let categories = vec![
                "Documents", "Images", "Videos", "Audio", "Archives",
                "Code", "Data", "Executables", "Other", "Skip",
            ];

            for file_path in unknown_files {
                let file_name = file_path.file_name().unwrap().to_string_lossy();
                
                let choice = Select::new(&format!("📄 {}", file_name), categories.clone())
                    .prompt()?;

                if choice == "Skip" {
                    continue;
                }

                let dest_dir = target_dir.join(choice);
                fs::create_dir_all(&dest_dir)?;
                
                let dest_path = dest_dir.join(file_path.file_name().unwrap());
                
                operation.moves.push(FileMove {
                    from: file_path.clone(),
                    to: dest_path.clone(),
                });
                
                fs::rename(&file_path, &dest_path)?;
                println!("  {} -> {}/", file_name.green(), choice);
            }
        }
    }

    history.add_operation(operation);
    history.save()?;
    
    println!("\n{}", format!("✅ Successfully sorted {} files.", history.operations.last().unwrap().moves.len()).green().bold());
    Ok(())
}

fn undo_last_operation(history: &mut SortHistory) -> Result<()> {
    if let Some(operation) = history.operations.pop() {
        println!("\n{}", "Reverting last sort operation...".yellow());
        
        let mut reverted = 0;
        for file_move in operation.moves.iter().rev() {
            if file_move.to.exists() {
                // Ensure source directory exists
                if let Some(parent) = file_move.from.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                fs::rename(&file_move.to, &file_move.from)?;
                println!("  {} <- {}", 
                    file_move.from.file_name().unwrap().to_string_lossy().green(),
                    file_move.to.parent().unwrap().file_name().unwrap().to_string_lossy()
                );
                reverted += 1;
            }
        }
        
        // Clean up empty directories
        let mut dirs_to_check: Vec<PathBuf> = operation.moves.iter()
            .filter_map(|m| m.to.parent().map(|p| p.to_path_buf()))
            .collect();
        dirs_to_check.sort();
        dirs_to_check.dedup();
        
        for dir in dirs_to_check {
            if dir.exists() && dir != operation.base_dir {
                if let Ok(entries) = fs::read_dir(&dir) {
                    if entries.count() == 0 {
                        let _ = fs::remove_dir(&dir);
                    }
                }
            }
        }
        
        history.save()?;
        println!("\n{}", format!("✅ Reverted {} file moves.", reverted).green().bold());
    } else {
        println!("No operations to undo.");
    }
    
    Ok(())
}

fn collect_files(target_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    for entry in fs::read_dir(target_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            // Skip hidden files
            if let Some(name) = path.file_name() {
                if !name.to_string_lossy().starts_with('.') {
                    files.push(path);
                }
            }
        }
    }
    
    Ok(files)
}

fn get_category(file_path: &Path) -> &str {
    let ext = file_path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    
    match ext.as_str() {
        // Documents
        "pdf" | "doc" | "docx" | "txt" | "rtf" | "odt" | "tex" | "md" => "Documents",
        // Images
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" | "webp" | "ico" => "Images",
        // Videos
        "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => "Videos",
        // Audio
        "mp3" | "wav" | "flac" | "aac" | "ogg" | "wma" | "m4a" => "Audio",
        // Archives
        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => "Archives",
        // Code
        "rs" | "py" | "js" | "ts" | "java" | "c" | "cpp" | "h" | "hpp" | "go" | 
        "rb" | "php" | "cs" | "swift" | "kt" | "scala" | "html" | "css" | "json" | 
        "xml" | "yaml" | "yml" | "toml" => "Code",
        // Data
        "csv" | "sql" | "db" | "sqlite" | "mdb" => "Data",
        // Executables
        "exe" | "msi" | "app" | "deb" | "rpm" | "dmg" | "pkg" => "Executables",
        _ => "Other",
    }
}

fn preview_sort<F>(files: &[PathBuf], categorizer: F) -> Result<()>
where
    F: Fn(&PathBuf) -> String,
{
    let mut category_counts: HashMap<String, usize> = HashMap::new();
    
    for file in files {
        let category = categorizer(file);
        *category_counts.entry(category).or_insert(0) += 1;
    }
    
    println!("\n{}", "Preview of sorting:".cyan().bold());
    for (category, count) in category_counts {
        println!("  {} -> {} file(s)", category.yellow(), count);
    }
    println!();
    
    Ok(())
}

fn confirm_operation() -> Result<bool> {
    Confirm::new("Proceed with sorting?")
        .with_default(true)
        .prompt()
        .context("Failed to get confirmation")
}
