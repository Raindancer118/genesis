use crate::config::ConfigManager;
use anyhow::{Result, Context};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use chrono::{DateTime, Utc};
use directories::ProjectDirs;

/// Represents a single indexed file entry
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
}

/// The file index structure
#[derive(Debug, Serialize, Deserialize)]
pub struct FileIndex {
    pub entries: Vec<FileEntry>,
    pub last_updated: DateTime<Utc>,
    pub indexed_paths: Vec<PathBuf>,
}

impl FileIndex {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            last_updated: Utc::now(),
            indexed_paths: Vec::new(),
        }
    }

    pub fn load(index_path: &Path) -> Result<Self> {
        if !index_path.exists() {
            return Ok(Self::new());
        }
        
        let content = fs::read_to_string(index_path)
            .context("Failed to read index file")?;
        let index: FileIndex = serde_json::from_str(&content)
            .context("Failed to parse index file")?;
        Ok(index)
    }

    pub fn save(&self, index_path: &Path) -> Result<()> {
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create index directory")?;
        }
        
        let content = serde_json::to_string_pretty(&self)
            .context("Failed to serialize index")?;
        fs::write(index_path, content)
            .context("Failed to write index file")?;
        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<&FileEntry> {
        let query_lower = query.to_lowercase();
        self.entries.iter()
            .filter(|entry| {
                entry.name.to_lowercase().contains(&query_lower) ||
                entry.path.to_string_lossy().to_lowercase().contains(&query_lower)
            })
            .collect()
    }
}

/// Get the path where the index file is stored
pub fn get_index_path() -> PathBuf {
    let config_dir = if let Some(proj_dirs) = ProjectDirs::from("", "", "genesis") {
        proj_dirs.data_dir().to_path_buf()
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local")
            .join("share")
            .join("genesis")
    };
    config_dir.join("file_index.json")
}

/// Build or rebuild the file index
pub fn build_index(paths: Vec<PathBuf>, config: &ConfigManager) -> Result<()> {
    println!("{}", "🔍 Building file index...".bold().cyan());
    
    let index_path = get_index_path();
    let mut index = FileIndex::new();
    
    let ignore_patterns = &config.config.search.ignore_patterns;
    let max_depth = config.config.search.max_depth;
    let exclude_hidden = config.config.search.exclude_hidden;
    
    for base_path in &paths {
        if !base_path.exists() {
            println!("{}", format!("⚠️  Path does not exist: {}", base_path.display()).yellow());
            continue;
        }
        
        println!("Indexing {}...", base_path.display());
        index.indexed_paths.push(base_path.clone());
        
        let walker = WalkDir::new(base_path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| should_include(e, ignore_patterns, exclude_hidden));
        
        for entry in walker {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() {
                        if let Some(file_entry) = create_file_entry(&entry) {
                            index.entries.push(file_entry);
                        }
                    }
                },
                Err(e) => {
                    // Skip entries we can't access (permission denied, etc.)
                    if config.config.search.verbose {
                        eprintln!("Error accessing entry: {}", e);
                    }
                }
            }
        }
    }
    
    index.last_updated = Utc::now();
    index.save(&index_path)?;
    
    println!("{}", format!("✅ Indexed {} files", index.entries.len()).bold().green());
    println!("Index saved to: {}", index_path.display());
    
    Ok(())
}

/// Check if a directory entry should be included based on ignore patterns
fn should_include(entry: &walkdir::DirEntry, ignore_patterns: &[String], exclude_hidden: bool) -> bool {
    let path = entry.path();
    let path_str = path.to_string_lossy();
    
    // Skip hidden files/directories if configured to do so
    if exclude_hidden {
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') && name_str != "." && name_str != ".." {
                return false;
            }
        }
    }
    
    // Check ignore patterns
    for pattern in ignore_patterns {
        if path_str.contains(pattern) {
            return false;
        }
    }
    
    true
}

/// Create a FileEntry from a walkdir DirEntry
fn create_file_entry(entry: &walkdir::DirEntry) -> Option<FileEntry> {
    let metadata = entry.metadata().ok()?;
    let modified = metadata.modified().ok()?;
    let modified_dt: DateTime<Utc> = modified.into();
    
    Some(FileEntry {
        path: entry.path().to_path_buf(),
        name: entry.file_name().to_string_lossy().to_string(),
        size: metadata.len(),
        modified: modified_dt,
    })
}

/// Search the index for files matching the query
pub fn search(query: String, config: &ConfigManager) -> Result<()> {
    let index_path = get_index_path();
    
    if !index_path.exists() {
        println!("{}", "⚠️  No index found. Please run 'genesis index' first to build the index.".yellow());
        return Ok(());
    }
    
    let index = FileIndex::load(&index_path)?;
    
    println!("{}", format!("🔍 Searching for '{}'...", query).bold().cyan());
    
    let results = index.search(&query);
    
    if results.is_empty() {
        println!("{}", "No results found.".yellow());
        return Ok(());
    }
    
    println!("\n{} results found:\n", results.len());
    
    let max_results = config.config.search.max_results;
    for (i, entry) in results.iter().take(max_results).enumerate() {
        println!("{} {}", 
            format!("{}.", i + 1).bold(),
            entry.path.display().to_string().cyan()
        );
        
        if config.config.search.show_details {
            println!("   Size: {} | Modified: {}", 
                format_bytes(entry.size),
                entry.modified.format("%Y-%m-%d %H:%M:%S")
            );
        }
    }
    
    if results.len() > max_results {
        println!("\n{}", format!("... and {} more results (use config to increase max_results)", 
            results.len() - max_results).dimmed());
    }
    
    println!("\n{}", format!("Index last updated: {}", 
        index.last_updated.format("%Y-%m-%d %H:%M:%S")).dimmed());
    
    Ok(())
}

/// Format bytes into human-readable format
fn format_bytes(bytes: u64) -> String {
    const UNIT: f64 = 1024.0;
    if bytes < 1024 {
        return format!("{} B", bytes);
    }
    
    let size = bytes as f64;
    let exp = ((size.ln() / UNIT.ln()).floor() as i32).max(1).min(5);
    let units = ["B", "KB", "MB", "GB", "TB", "PB"];
    let unit = units.get(exp as usize).unwrap_or(&"PB");
    
    format!("{:.1} {}", size / UNIT.powi(exp), unit)
}

/// Display index information
pub fn info() -> Result<()> {
    let index_path = get_index_path();
    
    if !index_path.exists() {
        println!("{}", "No index found. Run 'genesis index' to create one.".yellow());
        return Ok(());
    }
    
    let index = FileIndex::load(&index_path)?;
    
    println!("{}", "📊 Index Information".bold().cyan());
    println!("Location: {}", index_path.display());
    println!("Total files indexed: {}", index.entries.len());
    println!("Last updated: {}", index.last_updated.format("%Y-%m-%d %H:%M:%S"));
    println!("\nIndexed paths:");
    for path in &index.indexed_paths {
        println!("  - {}", path.display());
    }
    
    Ok(())
}
