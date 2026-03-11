use crate::ui;
use crate::config::ConfigManager;
use anyhow::{Result, Context};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use chrono::{DateTime, Utc};
use directories::ProjectDirs;

mod lightspeed;
use lightspeed::{LightspeedIndex, LightspeedEntry};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileIndex {
    pub entries: Vec<FileEntry>,
    pub last_updated: DateTime<Utc>,
    pub indexed_paths: Vec<PathBuf>,
}

impl FileIndex {
    pub fn new() -> Self {
        Self { entries: Vec::new(), last_updated: Utc::now(), indexed_paths: Vec::new() }
    }

    pub fn load(index_path: &Path) -> Result<Self> {
        if !index_path.exists() { return Ok(Self::new()); }
        let content = fs::read_to_string(index_path).context("Failed to read index file")?;
        serde_json::from_str(&content).context("Failed to parse index file")
    }

    pub fn save(&self, index_path: &Path) -> Result<()> {
        if let Some(parent) = index_path.parent() {
            fs::create_dir_all(parent).context("Failed to create index directory")?;
        }
        let content = serde_json::to_string_pretty(&self).context("Failed to serialize index")?;
        fs::write(index_path, content).context("Failed to write index file")?;
        Ok(())
    }
}

fn get_data_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("", "volantic", "genesis") {
        proj_dirs.data_dir().to_path_buf()
    } else {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
            .join(".local").join("share").join("volantic-genesis")
    }
}

pub fn get_index_path() -> PathBuf { get_data_dir().join("file_index.json") }
pub fn get_lightspeed_index_path() -> PathBuf { get_data_dir().join("lightspeed_index.json") }

pub fn build_index(paths: Vec<PathBuf>, config: &ConfigManager) -> Result<()> {
    ui::print_header("INDEX BUILD");
    ui::section("Scanning");

    let mut index = FileIndex::new();
    let ignore_patterns = &config.config.search.ignore_patterns;
    let max_depth = config.config.search.max_depth;
    let exclude_hidden = config.config.search.exclude_hidden;

    for base_path in &paths {
        if !base_path.exists() {
            ui::skip(&format!("Path not found: {}", base_path.display()));
            continue;
        }
        ui::info_line("Indexing", &base_path.display().to_string());
        index.indexed_paths.push(base_path.clone());

        let walker = WalkDir::new(base_path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| should_include(e, ignore_patterns, exclude_hidden));

        for entry in walker {
            match entry {
                Ok(entry) if entry.file_type().is_file() => {
                    if let Some(fe) = create_file_entry(&entry) {
                        index.entries.push(fe);
                    }
                }
                _ => {}
            }
        }
    }

    index.last_updated = Utc::now();
    index.save(&get_index_path())?;
    ui::success(&format!("Indexed {} files", index.entries.len()));

    // Build lightspeed index
    build_lightspeed_from_basic(&index, config)?;
    Ok(())
}

fn build_lightspeed_from_basic(basic_index: &FileIndex, config: &ConfigManager) -> Result<()> {
    ui::section("Building Lightspeed Index");
    let lightspeed_path = get_lightspeed_index_path();
    let mut ls_index = LightspeedIndex::new();

    ls_index.entries = basic_index.entries.iter().enumerate().map(|(idx, entry)| {
        LightspeedEntry {
            id: idx,
            path: entry.path.clone(),
            name: entry.name.clone(),
            name_lower: entry.name.to_lowercase(),
            size: entry.size,
            modified: entry.modified,
        }
    }).collect();

    ls_index.indexed_paths = basic_index.indexed_paths.clone();
    ls_index.last_updated = basic_index.last_updated;
    ls_index.build_ngram_index(3);

    let fuzzy_distance = config.config.search.fuzzy_threshold;
    if fuzzy_distance > 0 {
        ls_index.build_deletion_index(fuzzy_distance);
    }

    if let Some(parent) = lightspeed_path.parent() {
        fs::create_dir_all(parent).context("Failed to create index directory")?;
    }
    let content = serde_json::to_string(&ls_index).context("Failed to serialize lightspeed index")?;
    fs::write(&lightspeed_path, content).context("Failed to write lightspeed index")?;

    ui::success(&format!("Lightspeed index: {} n-gram entries", ls_index.ngram_index.len()));
    Ok(())
}

fn should_include(entry: &walkdir::DirEntry, ignore_patterns: &[String], exclude_hidden: bool) -> bool {
    let path = entry.path();
    let path_str = path.to_string_lossy();
    if exclude_hidden {
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') && name_str != "." && name_str != ".." {
                return false;
            }
        }
    }
    for pattern in ignore_patterns {
        if path_str.contains(pattern.as_str()) { return false; }
    }
    true
}

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

pub fn search(query: String, config: &ConfigManager) -> Result<()> {
    ui::print_header("SEARCH");

    let lightspeed_path = get_lightspeed_index_path();
    if lightspeed_path.exists() {
        return search_lightspeed(query, config);
    }

    ui::skip("Lightspeed index not found. Run 'vg index' first.");
    Ok(())
}

fn search_lightspeed(query: String, config: &ConfigManager) -> Result<()> {
    let content = fs::read_to_string(&get_lightspeed_index_path()).context("Failed to read lightspeed index")?;
    let ls_index: LightspeedIndex = serde_json::from_str(&content).context("Failed to parse lightspeed index")?;

    ui::section(&format!("Results for '{}'", query));

    let start = std::time::Instant::now();
    let results = ls_index.search_hybrid(&query, true, 30);
    let elapsed = start.elapsed();

    if results.is_empty() {
        ui::skip("No results found.");
        return Ok(());
    }

    let max = config.config.search.max_results;
    for (i, (idx, score)) in results.iter().take(max).enumerate() {
        let entry = &ls_index.entries[*idx];
        let quality = if *score > 80 { "▓▓▓" } else if *score > 50 { "▓▓░" } else { "▓░░" };
        println!("  {:>3}.  {}  {}",
            i + 1,
            entry.path.display().to_string().truecolor(96, 165, 250),
            quality.truecolor(71, 85, 105)
        );
    }

    if results.len() > max {
        ui::skip(&format!("... and {} more results", results.len() - max));
    }

    println!();
    ui::info_line("Search time", &format!("{:.2}ms", elapsed.as_secs_f64() * 1000.0));
    ui::info_line("Index age", &ls_index.last_updated.format("%Y-%m-%d %H:%M:%S").to_string());
    Ok(())
}

pub fn info() -> Result<()> {
    ui::print_header("INDEX INFO");

    let index_path = get_index_path();
    if !index_path.exists() {
        ui::skip("No index found. Run 'vg index' to create one.");
        return Ok(());
    }

    let index = FileIndex::load(&index_path)?;
    ui::section("File Index");
    ui::info_line("Location", &index_path.display().to_string());
    ui::info_line("Files", &index.entries.len().to_string());
    ui::info_line("Last updated", &index.last_updated.format("%Y-%m-%d %H:%M:%S").to_string());

    ui::section("Indexed Paths");
    for path in &index.indexed_paths {
        ui::info_line("·", &path.display().to_string());
    }

    let ls_path = get_lightspeed_index_path();
    if ls_path.exists() {
        if let Ok(content) = fs::read_to_string(&ls_path) {
            if let Ok(ls) = serde_json::from_str::<LightspeedIndex>(&content) {
                ui::section("Lightspeed Index");
                ui::info_line("N-gram entries", &ls.ngram_index.len().to_string());
                ui::info_line("Deletion entries", &ls.deletion_index.len().to_string());
            }
        }
    }

    Ok(())
}
