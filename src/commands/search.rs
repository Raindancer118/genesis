// src/commands/search.rs
use crate::ui;
use crate::config::ConfigManager;
use anyhow::{Result, Context};
use colored::Colorize;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use walkdir::WalkDir;
use directories::ProjectDirs;
use chrono::Utc;

fn get_db_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("", "volantic", "genesis") {
        proj_dirs.data_dir().join("search.db")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local").join("share").join("volantic-genesis").join("search.db")
    }
}

fn open_db() -> Result<Connection> {
    let db_path = get_db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).context("Failed to create data directory")?;
    }
    let conn = Connection::open(&db_path).context("Failed to open SQLite database")?;
    // Enable WAL mode for better performance
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    Ok(conn)
}

fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS index_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS files USING fts5(
            name,
            path,
            tokenize='unicode61'
        );
        CREATE TABLE IF NOT EXISTS files_meta (
            rowid INTEGER PRIMARY KEY,
            size INTEGER NOT NULL,
            modified TEXT NOT NULL
        );
    ")?;
    Ok(())
}

pub fn build_index(paths: Vec<PathBuf>, config: &ConfigManager) -> Result<()> {
    ui::print_header("INDEX BUILD");

    let conn = open_db()?;
    init_db(&conn)?;

    // Clear existing index
    conn.execute_batch("DELETE FROM files; DELETE FROM files_meta;")?;

    let ignore_patterns = &config.config.search.ignore_patterns;
    let max_depth = config.config.search.max_depth;
    let exclude_hidden = config.config.search.exclude_hidden;

    let mut count: u64 = 0;

    for base_path in &paths {
        if !base_path.exists() {
            ui::skip(&format!("Path not found: {}", base_path.display()));
            continue;
        }
        ui::info_line("Indexing", &base_path.display().to_string());

        let walker = WalkDir::new(base_path)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| should_include(e, ignore_patterns, exclude_hidden));

        for entry in walker {
            match entry {
                Ok(e) if e.file_type().is_file() => {
                    if let Ok(meta) = e.metadata() {
                        let name = e.file_name().to_string_lossy().to_string();
                        let path = e.path().to_string_lossy().to_string();
                        let size = meta.len() as i64;
                        let modified = meta.modified()
                            .map(|t| chrono::DateTime::<Utc>::from(t).to_rfc3339())
                            .unwrap_or_default();

                        conn.execute(
                            "INSERT INTO files(name, path) VALUES (?1, ?2)",
                            params![name, path],
                        )?;
                        let rowid = conn.last_insert_rowid();
                        conn.execute(
                            "INSERT INTO files_meta(rowid, size, modified) VALUES (?1, ?2, ?3)",
                            params![rowid, size, modified],
                        )?;
                        count += 1;
                    }
                }
                _ => {}
            }
        }
    }

    // Update metadata
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO index_meta(key, value) VALUES ('last_updated', ?1)",
        params![now],
    )?;
    let paths_str = paths.iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    conn.execute(
        "INSERT OR REPLACE INTO index_meta(key, value) VALUES ('indexed_paths', ?1)",
        params![paths_str],
    )?;

    println!();
    if count == 0 {
        ui::fail("No files indexed — all configured paths were missing or empty.");
        ui::skip("Update your paths:  vg config edit");
        ui::skip("Or specify directly: vg index --paths /home/you");
    } else {
        ui::success(&format!("Indexed {} files into SQLite FTS5", count));
        ui::info_line("Database", &get_db_path().display().to_string());
    }
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

pub fn search(query: String, config: &ConfigManager) -> Result<()> {
    ui::print_header("SEARCH");

    let db_path = get_db_path();
    if !db_path.exists() {
        ui::skip("No index found. Run 'vg index' first.");
        return Ok(());
    }

    let conn = open_db()?;

    ui::section(&format!("Results for '{}'", query));

    let start = std::time::Instant::now();

    // FTS5 query — escape special chars for safety
    let fts_query = sanitize_fts_query(&query);

    let max = config.config.search.max_results as i64;

    let mut stmt = conn.prepare(
        "SELECT f.path, f.name, m.size
         FROM files f
         JOIN files_meta m ON f.rowid = m.rowid
         WHERE files MATCH ?1
         ORDER BY rank
         LIMIT ?2"
    )?;

    let results: Vec<(String, String, i64)> = stmt.query_map(params![fts_query, max], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?.filter_map(|r| r.ok()).collect();

    let elapsed = start.elapsed();

    if results.is_empty() {
        ui::skip("No results found.");
        return Ok(());
    }

    for (i, (path, _name, size)) in results.iter().enumerate() {
        println!("  {:>3}.  {}  {}",
            i + 1,
            path.truecolor(96, 165, 250),
            fmt_bytes(*size as u64).truecolor(71, 85, 105)
        );
    }

    println!();
    ui::info_line("Results", &results.len().to_string());
    ui::info_line("Search time", &format!("{:.2}ms", elapsed.as_secs_f64() * 1000.0));

    Ok(())
}

fn sanitize_fts_query(query: &str) -> String {
    // Wrap in quotes for phrase search, or use prefix search
    // FTS5 special chars: " * ^ ( )
    let clean: String = query.chars().filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '.' || *c == '_' || *c == '-').collect();
    if clean.trim().is_empty() { return query.to_string(); }
    // Add * for prefix matching on each token
    clean.split_whitespace()
        .map(|token| format!("{}*", token))
        .collect::<Vec<_>>()
        .join(" ")
}

fn fmt_bytes(bytes: u64) -> String {
    const UNIT: u64 = 1024;
    if bytes < UNIT { return format!("{} B", bytes); }
    let div = UNIT as f64;
    let exp = (bytes as f64).log(div).floor() as i32;
    let pre = "KMGTPE".chars().nth((exp - 1) as usize).unwrap_or('?');
    format!("{:.1} {}B", (bytes as f64) / div.powi(exp), pre)
}

pub fn info() -> Result<()> {
    ui::print_header("INDEX INFO");

    let db_path = get_db_path();
    if !db_path.exists() {
        ui::skip("No index found. Run 'vg index' first.");
        return Ok(());
    }

    let conn = open_db()?;

    let count: i64 = conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
    let last_updated: String = conn.query_row(
        "SELECT value FROM index_meta WHERE key='last_updated'",
        [], |r| r.get(0)
    ).unwrap_or_else(|_| "unknown".to_string());
    let indexed_paths: String = conn.query_row(
        "SELECT value FROM index_meta WHERE key='indexed_paths'",
        [], |r| r.get(0)
    ).unwrap_or_default();

    ui::section("SQLite FTS5 Index");
    ui::info_line("Database", &db_path.display().to_string());
    ui::info_line("Files indexed", &count.to_string());
    ui::info_line("Last updated", &last_updated);

    ui::section("Indexed Paths");
    for path in indexed_paths.lines() {
        if !path.is_empty() {
            ui::info_line("·", path);
        }
    }

    // DB file size
    if let Ok(meta) = std::fs::metadata(&db_path) {
        ui::info_line("DB size", &fmt_bytes(meta.len()));
    }

    Ok(())
}
