use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use colored::Colorize;
use std::collections::HashMap;
use inquire::{Select, Confirm};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use crate::ai::GeminiClient;

// Size thresholds for file categorization
const SIZE_SMALL_THRESHOLD: u64 = 1_000_000; // 1 MB
const SIZE_MEDIUM_THRESHOLD: u64 = 100_000_000; // 100 MB

// Screenshot detection constants
const MIN_SCREENSHOT_WIDTH: u32 = 1200;
const MIN_SCREENSHOT_HEIGHT: u32 = 600;
const ASPECT_RATIO_TOLERANCE: f64 = 0.15;

// AI sorting constants
const HIGH_CONFIDENCE_THRESHOLD: f32 = 70.0;
const AI_SORTING_MIN_CONFIDENCE: f32 = 50.0;
const MIN_FILES_BEFORE_SMART_SWITCH: usize = 5;

#[derive(Debug)]
enum UserChoice {
    Retry,
    Continue,
    Abort,
}

fn handle_ai_error(error: &anyhow::Error, file_name: &str) -> Result<UserChoice> {
    println!("\n{}", format!("❌ AI Error while processing '{}'", file_name).red().bold());
    println!("{}", format!("Error: {}", error).red());
    
    let options = vec![
        "Retry this file",
        "Skip this file and continue with remaining files",
        "Abort entire operation",
    ];
    
    let choice = Select::new("What would you like to do?", options)
        .prompt()
        .context("Failed to get user input")?;
    
    match choice {
        s if s.starts_with("Retry") => Ok(UserChoice::Retry),
        s if s.starts_with("Skip") => Ok(UserChoice::Continue),
        _ => Ok(UserChoice::Abort),
    }
}

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
    ManualLearning,      // User manually categorizes each file, system learns
    AssistedLearning,    // System suggests based on heuristics, user corrects
    Smart,               // Uses learned patterns automatically
    AIAssistedLearning,  // System suggests, AI corrects/validates
    AILearning,          // AI suggests, user corrects
    AISorting,           // Fully automatic AI-based sorting
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
    let mut strategy_options = vec![
        "By Extension (group by file type)",
        "By Category (documents, images, videos, etc.)",
        "By Date Modified",
        "By Size (small, medium, large)",
        "Manual Learning (you categorize each file, system learns)",
        "Assisted Learning (system suggests based on rules, you correct)",
        "Smart (uses your learned patterns automatically)",
    ];
    
    // Only show AI options if API key is available
    if GeminiClient::is_available() {
        strategy_options.push("AI-Assisted Learning (system suggests, AI corrects) 🤖");
        strategy_options.push("AI Learning (AI suggests, you teach) ⚡");
        strategy_options.push("AI Sorting (fully automatic AI categorization) 🚀");
    }

    let strategy_choice = Select::new("Choose sorting strategy:", strategy_options.clone())
        .prompt()
        .context("Failed to get user input")?;

    let strategy = match strategy_choice {
        s if s.starts_with("By Extension") => SortStrategy::ByExtension,
        s if s.starts_with("By Category") => SortStrategy::ByCategory,
        s if s.starts_with("By Date") => SortStrategy::ByDate,
        s if s.starts_with("By Size") => SortStrategy::BySize,
        s if s.starts_with("Manual Learning") => SortStrategy::ManualLearning,
        s if s.starts_with("Assisted Learning") => SortStrategy::AssistedLearning,
        s if s.starts_with("Smart") => SortStrategy::Smart,
        s if s.starts_with("AI-Assisted Learning") => SortStrategy::AIAssistedLearning,
        s if s.starts_with("AI Learning") => SortStrategy::AILearning,
        s if s.starts_with("AI Sorting") => SortStrategy::AISorting,
        _ => SortStrategy::ByExtension,
    };

    // Perform sorting based on strategy
    match strategy {
        SortStrategy::ByExtension => sort_by_extension(target_dir, &mut history)?,
        SortStrategy::ByCategory => sort_by_category(target_dir, &mut history)?,
        SortStrategy::ByDate => sort_by_date(target_dir, &mut history)?,
        SortStrategy::BySize => sort_by_size(target_dir, &mut history)?,
        SortStrategy::ManualLearning => sort_manual_learning(target_dir, &mut history)?,
        SortStrategy::AssistedLearning => sort_assisted_learning(target_dir, &mut history)?,
        SortStrategy::Smart => sort_smart(target_dir, &mut history)?,
        SortStrategy::AIAssistedLearning => sort_ai_assisted_learning(target_dir, &mut history)?,
        SortStrategy::AILearning => sort_ai_learning(target_dir, &mut history)?,
        SortStrategy::AISorting => sort_ai_sorting(target_dir, &mut history)?,
    }

    Ok(())
}

fn print_success_message(count: usize) {
    println!("\n{}", format!("✅ Successfully sorted {} files.", count).green().bold());
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
        let ext_str = file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("no_extension")
            .to_lowercase();
        
        let dest_dir = target_dir.join(&ext_str);
        fs::create_dir_all(&dest_dir)?;
        
        if let Some(file_name) = file_path.file_name() {
            let dest_path = dest_dir.join(file_name);
            
            operation.moves.push(FileMove {
                from: file_path.clone(),
                to: dest_path.clone(),
            });
            
            fs::rename(&file_path, &dest_path)?;
            println!("  {} -> {}/", file_name.to_string_lossy().green(), ext_str);
        }
    }

    let count = operation.moves.len();
    history.add_operation(operation);
    history.save()?;
    
    print_success_message(count);
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
        
        if let Some(file_name) = file_path.file_name() {
            let dest_path = dest_dir.join(file_name);
            
            operation.moves.push(FileMove {
                from: file_path.clone(),
                to: dest_path.clone(),
            });
            
            fs::rename(&file_path, &dest_path)?;
            println!("  {} -> {}/", file_name.to_string_lossy().green(), category);
        }
    }

    let count = operation.moves.len();
    history.add_operation(operation);
    history.save()?;
    
    print_success_message(count);
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
        
        if let Some(file_name) = file_path.file_name() {
            let dest_path = dest_dir.join(file_name);
            
            operation.moves.push(FileMove {
                from: file_path.clone(),
                to: dest_path.clone(),
            });
            
            fs::rename(&file_path, &dest_path)?;
            println!("  {} -> {}/", file_name.to_string_lossy().green(), date_folder);
        }
    }

    let count = operation.moves.len();
    history.add_operation(operation);
    history.save()?;
    
    print_success_message(count);
    Ok(())
}

fn get_size_category(size: u64) -> &'static str {
    if size < SIZE_SMALL_THRESHOLD {
        "small"
    } else if size < SIZE_MEDIUM_THRESHOLD {
        "medium"
    } else {
        "large"
    }
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
            .map(|m| get_size_category(m.len()))
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
            .map(|m| get_size_category(m.len()))
            .unwrap_or("unknown");
        
        let dest_dir = target_dir.join(size_category);
        fs::create_dir_all(&dest_dir)?;
        
        if let Some(file_name) = file_path.file_name() {
            let dest_path = dest_dir.join(file_name);
            
            operation.moves.push(FileMove {
                from: file_path.clone(),
                to: dest_path.clone(),
            });
            
            fs::rename(&file_path, &dest_path)?;
            println!("  {} -> {}/", file_name.to_string_lossy().green(), size_category);
        }
    }

    let count = operation.moves.len();
    history.add_operation(operation);
    history.save()?;
    
    print_success_message(count);
    Ok(())
}

fn sort_manual_learning(target_dir: &Path, history: &mut SortHistory) -> Result<()> {
    println!("\n{}", "Manual Learning mode - Pure manual categorization".yellow());
    println!("{}", "You choose every file's category. The system learns silently.".cyan());
    
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
        let file_name_display = file_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Check if we have a learned category
        let suggested_idx = learning_data.extension_categories.get(&ext)
            .and_then(|cat| categories.iter().position(|c| c.to_lowercase() == cat.to_lowercase()));

        let prompt = if let Some(idx) = suggested_idx {
            format!("📄 {} - Suggested: {}", file_name_display, categories[idx])
        } else {
            format!("📄 {}", file_name_display)
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
        
        if let Some(file_name) = file_path.file_name() {
            let dest_path = dest_dir.join(file_name);
            
            operation.moves.push(FileMove {
                from: file_path.clone(),
                to: dest_path.clone(),
            });
            
            fs::rename(&file_path, &dest_path)?;
            println!("  {} -> {}/", file_name_display.green(), choice);
        }
    }

    learning_data.save()?;
    let count = operation.moves.len();
    history.add_operation(operation);
    history.save()?;
    
    print_success_message(count);
    println!("{}", "Your choices have been saved for future smart sorting!".cyan());
    Ok(())
}

fn sort_assisted_learning(target_dir: &Path, history: &mut SortHistory) -> Result<()> {
    println!("\n{}", "Assisted Learning mode - System suggests, you decide".yellow());
    println!("{}", "The system uses rules to suggest categories and learns from you.".cyan());
    
    let files = collect_files(target_dir)?;
    if files.is_empty() {
        println!("No files to sort.");
        return Ok(());
    }

    let mut learning_data = LearningData::load().unwrap_or_else(|_| LearningData {
        extension_categories: HashMap::new(),
    });

    let categories = vec![
        "Documents", "Images", "Images/Screenshots", "Videos", "Audio", "Archives",
        "Code", "Data", "Executables", "Other", "Skip",
    ];

    let mut operation = SortOperation {
        timestamp: Utc::now(),
        base_dir: target_dir.to_path_buf(),
        moves: Vec::new(),
    };

    for file_path in files {
        let file_name_display = file_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Get rule-based suggestion
        let mut suggested_category = get_category(&file_path).to_string();
        
        // Check if it might be a screenshot
        if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
            if let Ok(true) = detect_screenshot(&file_path) {
                suggested_category = "Images/Screenshots".to_string();
            }
        }
        
        // Check if we have a learned category that's different
        if let Some(learned_cat) = learning_data.extension_categories.get(&ext) {
            if learned_cat != &suggested_category {
                suggested_category = learned_cat.clone();
            }
        }

        let suggested_idx = categories.iter()
            .position(|c| c.to_lowercase() == suggested_category.to_lowercase())
            .unwrap_or(0);

        let prompt = format!("📄 {} - Suggested: {}", file_name_display, suggested_category);

        let choice = Select::new(&prompt, categories.clone())
            .with_starting_cursor(suggested_idx)
            .prompt()?;

        if choice == "Skip" {
            continue;
        }

        // Learn from user choice
        if !ext.is_empty() && choice != suggested_category {
            learning_data.extension_categories.insert(ext.clone(), choice.to_string());
        }

        let dest_dir = target_dir.join(choice);
        fs::create_dir_all(&dest_dir)?;
        
        if let Some(file_name) = file_path.file_name() {
            let dest_path = dest_dir.join(file_name);
            
            operation.moves.push(FileMove {
                from: file_path.clone(),
                to: dest_path.clone(),
            });
            
            fs::rename(&file_path, &dest_path)?;
            println!("  {} -> {}/", file_name_display.green(), choice);
        }
    }

    learning_data.save()?;
    let count = operation.moves.len();
    history.add_operation(operation);
    history.save()?;
    
    print_success_message(count);
    println!("{}", "Your corrections have been learned!".cyan());
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
            
            if let Some(file_name) = file_path.file_name() {
                let dest_path = dest_dir.join(file_name);
                
                operation.moves.push(FileMove {
                    from: file_path.clone(),
                    to: dest_path.clone(),
                });
                
                fs::rename(&file_path, &dest_path)?;
                println!("  {} -> {}/", file_name.to_string_lossy().green(), category);
            }
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
                let file_name_display = file_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                
                let choice = Select::new(&format!("📄 {}", file_name_display), categories.clone())
                    .prompt()?;

                if choice == "Skip" {
                    continue;
                }

                let dest_dir = target_dir.join(choice);
                fs::create_dir_all(&dest_dir)?;
                
                if let Some(file_name) = file_path.file_name() {
                    let dest_path = dest_dir.join(file_name);
                    
                    operation.moves.push(FileMove {
                        from: file_path.clone(),
                        to: dest_path.clone(),
                    });
                    
                    fs::rename(&file_path, &dest_path)?;
                    println!("  {} -> {}/", file_name_display.green(), choice);
                }
            }
        }
    }

    let count = operation.moves.len();
    history.add_operation(operation);
    history.save()?;
    
    print_success_message(count);
    Ok(())
}

fn sort_ai_assisted_learning(target_dir: &Path, history: &mut SortHistory) -> Result<()> {
    println!("\n{}", "AI-Assisted Learning mode - System suggests, AI validates".yellow());
    println!("{}", "The system suggests categories based on rules, then AI validates/corrects them.".cyan());
    
    let ai_client = match GeminiClient::new() {
        Ok(client) => client,
        Err(e) => {
            println!("{}", format!("Error: Failed to initialize AI client: {}", e).red());
            println!("{}", "Make sure GEMINI_API_KEY environment variable is set.".yellow());
            return Ok(());
        }
    };
    
    let files = collect_files(target_dir)?;
    if files.is_empty() {
        println!("No files to sort.");
        return Ok(());
    }

    let mut learning_data = LearningData::load().unwrap_or_else(|_| LearningData {
        extension_categories: HashMap::new(),
    });

    let mut operation = SortOperation {
        timestamp: Utc::now(),
        base_dir: target_dir.to_path_buf(),
        moves: Vec::new(),
    };

    for (idx, file_path) in files.iter().enumerate() {
        let file_name_display = file_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        println!("\n{} [{}/{}]", format!("Processing: {}", file_name_display).bold(), idx + 1, files.len());

        // Get system's rule-based suggestion
        let mut system_suggestion = get_category(&file_path).to_string();
        
        // Check if it might be a screenshot
        if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
            if let Ok(true) = detect_screenshot(&file_path) {
                system_suggestion = "Images/Screenshots".to_string();
            }
        }

        println!("  {} suggests: {}", "System".cyan(), system_suggestion);

        // Get file metadata for AI analysis
        let metadata = get_file_metadata(file_path)?;
        
        // Ask AI to validate/correct the system's suggestion with retry support
        let (ai_suggestion, confidence) = loop {
            match ai_client.suggest_category(
                &file_name_display,
                &ext,
                &metadata,
            ) {
                Ok(result) => break result,
                Err(e) => {
                    match handle_ai_error(&e, &file_name_display)? {
                        UserChoice::Retry => continue, // Retry the AI call
                        UserChoice::Continue => {
                            // Skip AI, use system suggestion
                            println!("{}", "  Using system suggestion.".yellow());
                            break (system_suggestion.clone(), 0.0);
                        }
                        UserChoice::Abort => {
                            println!("\n{}", "Operation aborted by user.".yellow());
                            
                            // Save any moves that were completed
                            if !operation.moves.is_empty() {
                                let count = operation.moves.len();
                                history.add_operation(operation);
                                history.save()?;
                                learning_data.save()?;
                                print_success_message(count);
                                println!("{}", "AI corrections have been learned!".cyan());
                            }
                            
                            return Ok(());
                        }
                    }
                }
            }
        };

        let final_category = if confidence >= HIGH_CONFIDENCE_THRESHOLD && ai_suggestion != system_suggestion {
            // AI disagrees with high confidence
            println!("  {} corrects to: {} (confidence: {:.0}%)", "AI".green(), ai_suggestion, confidence);
            
            // Ask AI to explain the correction
            if let Ok(explanation) = ai_client.learn_from_correction(
                &file_name_display,
                &system_suggestion,
                &ai_suggestion,
            ) {
                println!("\n  {}", "Why the correction:".cyan().bold());
                println!("  {}", explanation.trim());
            }
            
            // Learn from AI correction
            if !ext.is_empty() {
                learning_data.extension_categories.insert(ext.clone(), ai_suggestion.clone());
            }
            
            ai_suggestion
        } else if confidence > 0.0 && confidence < HIGH_CONFIDENCE_THRESHOLD && ai_suggestion != system_suggestion {
            // AI is unsure - keep system suggestion
            println!("  {} is unsure (confidence: {:.0}%), keeping system suggestion", "AI".yellow(), confidence);
            system_suggestion
        } else {
            // AI agrees or has no strong opinion
            println!("  {} agrees", "AI".green());
            system_suggestion
        };

        let dest_dir = target_dir.join(&final_category);
        fs::create_dir_all(&dest_dir)?;
        
        if let Some(file_name) = file_path.file_name() {
            let dest_path = dest_dir.join(file_name);
            
            operation.moves.push(FileMove {
                from: file_path.clone(),
                to: dest_path.clone(),
            });
            
            fs::rename(&file_path, &dest_path)?;
            println!("  {} -> {}/", file_name_display.green(), final_category);
        }
    }

    learning_data.save()?;
    let count = operation.moves.len();
    history.add_operation(operation);
    history.save()?;
    
    print_success_message(count);
    println!("{}", "AI corrections have been learned!".cyan());
    Ok(())
}

fn sort_ai_learning(target_dir: &Path, history: &mut SortHistory) -> Result<()> {
    println!("\n{}", "AI Learning mode - AI suggests, you teach".yellow());
    println!("{}", "The AI categorizes and you correct it. Both learn from your feedback.".cyan());
    
    let ai_client = match GeminiClient::new() {
        Ok(client) => client,
        Err(e) => {
            println!("{}", format!("Error: Failed to initialize AI client: {}", e).red());
            println!("{}", "Make sure GEMINI_API_KEY environment variable is set.".yellow());
            return Ok(());
        }
    };
    
    let files = collect_files(target_dir)?;
    if files.is_empty() {
        println!("No files to sort.");
        return Ok(());
    }

    let mut learning_data = LearningData::load().unwrap_or_else(|_| LearningData {
        extension_categories: HashMap::new(),
    });

    let categories = vec![
        "Documents", "Images", "Images/Screenshots", "Videos", "Audio", "Archives",
        "Code", "Data", "Executables", "Other",
    ];

    let mut operation = SortOperation {
        timestamp: Utc::now(),
        base_dir: target_dir.to_path_buf(),
        moves: Vec::new(),
    };

    let mut switch_to_smart = false;

    for (idx, file_path) in files.iter().enumerate() {
        if switch_to_smart {
            println!("\n{}", "Switching to Smart mode for remaining files...".cyan());
            break;
        }

        let file_name_display = file_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        println!("\n{} [{}/{}]", format!("Processing: {}", file_name_display).bold(), idx + 1, files.len());

        // Get file metadata for AI analysis
        let metadata = get_file_metadata(file_path)?;
        
        // Try AI suggestion with retry support
        let (suggested_category, confidence) = loop {
            match ai_client.suggest_category(
                &file_name_display,
                &ext,
                &metadata,
            ) {
                Ok(result) => break result,
                Err(e) => {
                    match handle_ai_error(&e, &file_name_display)? {
                        UserChoice::Retry => continue, // Retry the AI call
                        UserChoice::Continue => {
                            // Skip this file and continue
                            break ("Other".to_string(), 0.0);
                        }
                        UserChoice::Abort => {
                            println!("\n{}", "Operation aborted by user.".yellow());
                            
                            // Save any moves that were completed
                            if !operation.moves.is_empty() {
                                let count = operation.moves.len();
                                history.add_operation(operation);
                                history.save()?;
                                learning_data.save()?;
                                print_success_message(count);
                                println!("{}", format!("Processed {} files before aborting.", count).cyan());
                            }
                            
                            return Ok(());
                        }
                    }
                }
            }
        };

        let category = if confidence >= HIGH_CONFIDENCE_THRESHOLD {
            // High confidence - use AI suggestion
            println!("  {} suggests: {} (confidence: {:.0}%)", "AI".green(), suggested_category, confidence);
            
            let use_suggestion = Confirm::new("Use this category?")
                .with_default(true)
                .prompt()?;
            
            if use_suggestion {
                suggested_category.clone()
            } else {
                // User disagreed, ask for correct category and learn
                let correct_category = Select::new("Choose correct category:", categories.clone())
                    .prompt()?;
                
                // Ask AI to learn from this correction
                if let Ok(explanation) = ai_client.learn_from_correction(
                    &file_name_display,
                    &suggested_category,
                    correct_category,
                ) {
                    println!("\n{}", "AI Learning:".cyan().bold());
                    println!("{}", explanation.trim());
                }
                
                // Save to learning data for future
                if !ext.is_empty() {
                    learning_data.extension_categories.insert(ext.clone(), correct_category.to_string());
                }
                
                correct_category.to_string()
            }
        } else {
            // Low confidence - ask user
            println!("  {} is unsure (confidence: {:.0}%)", "AI".yellow(), confidence);
            if confidence > 0.0 {
                println!("  Suggested: {}", suggested_category);
            }
            
            let choice = Select::new("Please choose category:", categories.clone())
                .prompt()?;
            
            // Learn from user choice
            if !ext.is_empty() {
                learning_data.extension_categories.insert(ext.clone(), choice.to_string());
            }
            
            choice.to_string()
        };

        // Ask if user wants to switch to smart mode
        if idx > MIN_FILES_BEFORE_SMART_SWITCH && !learning_data.extension_categories.is_empty() {
            let switch = Confirm::new("Switch to Smart mode for remaining files?")
                .with_default(false)
                .prompt()
                .unwrap_or(false);
            
            if switch {
                switch_to_smart = true;
            }
        }

        let dest_dir = target_dir.join(&category);
        fs::create_dir_all(&dest_dir)?;
        
        if let Some(file_name) = file_path.file_name() {
            let dest_path = dest_dir.join(file_name);
            
            operation.moves.push(FileMove {
                from: file_path.clone(),
                to: dest_path.clone(),
            });
            
            fs::rename(&file_path, &dest_path)?;
            println!("  {} -> {}/", file_name_display.green(), category);
        }
    }

    // If switched to smart mode, process remaining files
    if switch_to_smart {
        let remaining_files: Vec<PathBuf> = files.into_iter()
            .skip(operation.moves.len())
            .collect();
        
        for file_path in remaining_files {
            let ext = file_path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            
            if let Some(category) = learning_data.extension_categories.get(&ext) {
                let dest_dir = target_dir.join(category);
                fs::create_dir_all(&dest_dir)?;
                
                if let Some(file_name) = file_path.file_name() {
                    let dest_path = dest_dir.join(file_name);
                    
                    operation.moves.push(FileMove {
                        from: file_path.clone(),
                        to: dest_path.clone(),
                    });
                    
                    fs::rename(&file_path, &dest_path)?;
                    println!("  {} -> {}/", file_name.to_string_lossy().green(), category);
                }
            }
        }
    }

    learning_data.save()?;
    let count = operation.moves.len();
    history.add_operation(operation);
    history.save()?;
    
    print_success_message(count);
    println!("{}", "AI learning data has been saved!".cyan());
    Ok(())
}

fn get_file_metadata(file_path: &Path) -> Result<String> {
    let mut metadata_parts = Vec::new();
    
    if let Ok(meta) = fs::metadata(file_path) {
        metadata_parts.push(format!("Size: {} bytes", meta.len()));
        
        if let Ok(modified) = meta.modified() {
            if let Ok(datetime) = modified.elapsed() {
                metadata_parts.push(format!("Modified: {} seconds ago", datetime.as_secs()));
            }
        }
    }
    
    // Try to detect if it's a screenshot using image analysis
    if let Some(ext) = file_path.extension() {
        let ext_str = ext.to_string_lossy().to_lowercase();
        if matches!(ext_str.as_str(), "png" | "jpg" | "jpeg" | "webp") {
            if let Ok(is_screenshot) = detect_screenshot(file_path) {
                if is_screenshot {
                    metadata_parts.push("Likely a screenshot (widescreen aspect ratio)".to_string());
                }
            }
        }
    }
    
    Ok(metadata_parts.join(", "))
}

fn detect_screenshot(file_path: &Path) -> Result<bool> {
    use image::GenericImageView;
    
    let img = image::open(file_path)
        .context("Failed to open image")?;
    
    let (width, height) = img.dimensions();
    
    // Check if it's widescreen (16:9 or similar)
    if width > MIN_SCREENSHOT_WIDTH && height > MIN_SCREENSHOT_HEIGHT {
        let aspect_ratio = width as f64 / height as f64;
        let is_widescreen = (aspect_ratio - 16.0/9.0).abs() < ASPECT_RATIO_TOLERANCE
            || (aspect_ratio - 16.0/10.0).abs() < ASPECT_RATIO_TOLERANCE
            || (aspect_ratio - 21.0/9.0).abs() < ASPECT_RATIO_TOLERANCE;
        
        return Ok(is_widescreen);
    }
    
    Ok(false)
}

fn sort_ai_sorting(target_dir: &Path, history: &mut SortHistory) -> Result<()> {
    println!("\n{}", "AI Sorting mode - Fully automatic AI categorization".yellow());
    println!("{}", "The AI will categorize all files automatically without user input.".cyan());
    
    let ai_client = match GeminiClient::new() {
        Ok(client) => client,
        Err(e) => {
            println!("{}", format!("Error: Failed to initialize AI client: {}", e).red());
            println!("{}", "Make sure GEMINI_API_KEY environment variable is set.".yellow());
            return Ok(());
        }
    };
    
    let files = collect_files(target_dir)?;
    if files.is_empty() {
        println!("No files to sort.");
        return Ok(());
    }

    println!("\n{}", format!("Processing {} files with AI...", files.len()).cyan());
    
    // Ask for confirmation before proceeding
    let proceed = Confirm::new("Proceed with automatic AI sorting?")
        .with_default(true)
        .prompt()?;
    
    if !proceed {
        println!("Operation cancelled.");
        return Ok(());
    }

    let mut operation = SortOperation {
        timestamp: Utc::now(),
        base_dir: target_dir.to_path_buf(),
        moves: Vec::new(),
    };

    let mut successful = 0;
    let mut failed = 0;

    for (idx, file_path) in files.iter().enumerate() {
        let file_name_display = file_path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let ext = file_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        print!("\r{} [{}/{}]", "Processing...".cyan(), idx + 1, files.len());
        std::io::Write::flush(&mut std::io::stdout()).ok();

        // Get file metadata for AI analysis
        let metadata = get_file_metadata(file_path)?;
        
        // Get AI suggestion with retry support
        let category = loop {
            match ai_client.suggest_category(
                &file_name_display,
                &ext,
                &metadata,
            ) {
                Ok((suggested_category, confidence)) => {
                    if confidence >= AI_SORTING_MIN_CONFIDENCE {
                        break suggested_category;
                    } else {
                        // Low confidence, use fallback
                        break get_category(&file_path).to_string();
                    }
                }
                Err(e) => {
                    println!(); // New line after progress indicator
                    match handle_ai_error(&e, &file_name_display)? {
                        UserChoice::Retry => continue, // Retry the AI call
                        UserChoice::Continue => {
                            // Skip this file, use fallback
                            failed += 1;
                            break get_category(&file_path).to_string();
                        }
                        UserChoice::Abort => {
                            println!("\n{}", "Operation aborted by user.".yellow());
                            
                            // Save any moves that were completed
                            if !operation.moves.is_empty() {
                                let count = operation.moves.len();
                                history.add_operation(operation);
                                history.save()?;
                                print_success_message(count);
                                println!("{}", format!("Successfully categorized: {}", successful).green());
                                if failed > 0 {
                                    println!("{}", format!("Failed AI categorization (used fallback): {}", failed).yellow());
                                }
                                println!("{}", format!("Processed {} out of {} files before aborting.", idx + 1, files.len()).cyan());
                            }
                            
                            return Ok(());
                        }
                    }
                }
            }
        };

        let dest_dir = target_dir.join(&category);
        fs::create_dir_all(&dest_dir)?;
        
        if let Some(file_name) = file_path.file_name() {
            let dest_path = dest_dir.join(file_name);
            
            operation.moves.push(FileMove {
                from: file_path.clone(),
                to: dest_path.clone(),
            });
            
            fs::rename(&file_path, &dest_path)?;
            successful += 1;
        }
    }

    println!(); // New line after progress
    
    let count = operation.moves.len();
    history.add_operation(operation);
    history.save()?;
    
    print_success_message(count);
    println!("{}", format!("Successfully categorized: {}", successful).green());
    if failed > 0 {
        println!("{}", format!("Failed AI categorization (used fallback): {}", failed).yellow());
    }
    println!("{}", "Tip: Use AI-Assisted Learning mode to teach the AI about your preferences!".cyan());
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
                
                if let Err(e) = fs::rename(&file_move.to, &file_move.from) {
                    eprintln!("Warning: Failed to revert {}: {}", 
                        file_move.to.display(), e);
                    continue;
                }
                
                if let (Some(from_name), Some(to_parent)) = 
                    (file_move.from.file_name(), file_move.to.parent().and_then(|p| p.file_name())) {
                    println!("  {} <- {}", 
                        from_name.to_string_lossy().green(),
                        to_parent.to_string_lossy()
                    );
                }
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
                if let Ok(mut entries) = fs::read_dir(&dir) {
                    if entries.next().is_none() {
                        if let Err(e) = fs::remove_dir(&dir) {
                            eprintln!("Warning: Failed to remove empty directory {}: {}", dir.display(), e);
                        }
                    }
                }
            }
        }
        
        history.save()?;
        print_success_message(reverted);
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
