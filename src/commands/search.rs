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
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

// Text extensions whose content will be indexed
const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "rs", "py", "js", "ts", "jsx", "tsx", "toml", "yaml", "yml", "json",
    "html", "css", "scss", "sh", "bash", "zsh", "fish", "c", "cpp", "h", "hpp",
    "go", "java", "kt", "swift", "rb", "php", "sql", "xml", "ini", "conf", "log",
    "env", "gitignore", "dockerfile", "makefile",
];

const MAX_CONTENT_BYTES: usize = 256 * 1024; // 256 KB
const FUZZY_SCAN_LIMIT: i64 = 50_000;
const FUZZY_SCORE_THRESHOLD: i64 = 30;
const FUZZY_FALLBACK_THRESHOLD: usize = 5;
const PROGRESS_INTERVAL: u64 = 10_000;

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
    migrate_schema(&conn)?;
    Ok(conn)
}

fn migrate_schema(conn: &Connection) -> Result<()> {
    // Check if the files FTS table has the content column
    let content_col_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name='content'",
        [],
        |r| r.get(0),
    ).unwrap_or(0);

    if content_col_count == 0 {
        // Drop old tables and recreate with new schema
        conn.execute_batch("
            DROP TABLE IF EXISTS files;
            DROP TABLE IF EXISTS files_meta;
        ")?;
        ui::skip("Index schema updated — please run 'vg index' to rebuild.");
    }

    Ok(())
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
            content,
            tokenize='unicode61'
        );
        CREATE TABLE IF NOT EXISTS files_meta (
            rowid INTEGER PRIMARY KEY,
            size INTEGER NOT NULL,
            modified TEXT NOT NULL,
            ext TEXT NOT NULL DEFAULT ''
        );
    ")?;
    Ok(())
}

fn is_text_extension(ext: &str) -> bool {
    let lower = ext.to_lowercase();
    TEXT_EXTENSIONS.contains(&lower.as_str())
}

fn read_file_content(path: &str, ext: &str) -> String {
    if !is_text_extension(ext) {
        return String::new();
    }
    match std::fs::read(path) {
        Ok(bytes) => {
            let truncated = if bytes.len() > MAX_CONTENT_BYTES {
                &bytes[..MAX_CONTENT_BYTES]
            } else {
                &bytes
            };
            // Strip null bytes and convert
            let s = String::from_utf8_lossy(truncated);
            s.chars().filter(|&c| c != '\0').collect()
        }
        Err(_) => String::new(),
    }
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

                        // Get extension
                        let ext = e.path()
                            .extension()
                            .map(|s| s.to_string_lossy().to_lowercase())
                            .unwrap_or_default();

                        // Read content for text files
                        let content = read_file_content(&path, &ext);

                        conn.execute(
                            "INSERT INTO files(name, path, content) VALUES (?1, ?2, ?3)",
                            params![name, path, content],
                        )?;
                        let rowid = conn.last_insert_rowid();
                        conn.execute(
                            "INSERT INTO files_meta(rowid, size, modified, ext) VALUES (?1, ?2, ?3, ?4)",
                            params![rowid, size, modified, ext],
                        )?;
                        count += 1;

                        if count % PROGRESS_INTERVAL == 0 {
                            ui::info_line("Progress", &format!("{} files...", format_number(count)));
                        }
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

pub struct SearchParams {
    pub query: String,
    pub ext: Option<String>,
    pub path_filter: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct SearchResult {
    rowid: i64,
    name: String,
    path: String,
    size: i64,
    ext: String,
    snippet: Option<String>,
    match_type: String,
    is_fuzzy: bool,
}

fn validate_ext_part(ext: &str) -> bool {
    ext.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

pub fn search(params: SearchParams, config: &ConfigManager) -> Result<()> {
    ui::print_header("SEARCH");

    let db_path = get_db_path();
    if !db_path.exists() {
        ui::skip("No index found. Run 'vg index' first.");
        return Ok(());
    }

    let conn = open_db()?;

    ui::section(&format!("Results for '{}'", params.query));

    let start = std::time::Instant::now();

    let fts_query = sanitize_fts_query(&params.query);
    // Default: show 10; user can override with --limit
    let limit = params.limit.unwrap_or(10);
    // Fetch more than needed so we know if there are additional results
    let fetch_limit = limit + 1;

    // Build ext filter clause
    let ext_clause = if let Some(ref ext_str) = params.ext {
        let parts: Vec<&str> = ext_str.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let valid_parts: Vec<&str> = parts.into_iter().filter(|s| validate_ext_part(s)).collect();
        if valid_parts.is_empty() {
            None
        } else {
            let quoted: Vec<String> = valid_parts.iter().map(|s| format!("'{}'", s)).collect();
            Some(format!("m.ext IN ({})", quoted.join(", ")))
        }
    } else {
        None
    };

    // Build path filter
    let path_pattern = params.path_filter.as_ref().map(|p| format!("{}%", p));

    // Build the full SQL query
    let sql = {
        let mut conditions = vec!["files MATCH ?1".to_string()];
        if let Some(ref ec) = ext_clause {
            conditions.push(ec.clone());
        }
        if path_pattern.is_some() {
            conditions.push("f.path LIKE ?3".to_string());
        }
        format!(
            "SELECT f.rowid, f.name, f.path, m.size, m.ext,
                    snippet(files, 2, '[', ']', '...', 20) as snip
             FROM files f
             JOIN files_meta m ON f.rowid = m.rowid
             WHERE {}
             ORDER BY bm25(files, 10.0, 5.0, 1.0)
             LIMIT ?2",
            conditions.join(" AND ")
        )
    };

    let mut fts_results: Vec<SearchResult> = {
        let mut stmt = conn.prepare(&sql)?;
        let fetch_limit_i64 = fetch_limit as i64;

        if path_pattern.is_some() {
            stmt.query_map(
                params![fts_query, fetch_limit_i64, path_pattern.as_deref()],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )?.filter_map(|r| r.ok()).map(|(rowid, name, path, size, ext, snip)| {
                let match_type = determine_match_type(&params.query, &name, &path, false);
                let snippet = if snip.contains('[') { Some(snip) } else { None };
                SearchResult { rowid, name, path, size, ext, snippet, match_type, is_fuzzy: false }
            }).collect()
        } else {
            stmt.query_map(
                params![fts_query, fetch_limit_i64],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )?.filter_map(|r| r.ok()).map(|(rowid, name, path, size, ext, snip)| {
                let match_type = determine_match_type(&params.query, &name, &path, false);
                let snippet = if snip.contains('[') { Some(snip) } else { None };
                SearchResult { rowid, name, path, size, ext, snippet, match_type, is_fuzzy: false }
            }).collect()
        }
    };

    // Fuzzy fallback if not enough FTS results
    if fts_results.len() < FUZZY_FALLBACK_THRESHOLD {
        let existing_rowids: std::collections::HashSet<i64> = fts_results.iter().map(|r| r.rowid).collect();

        let mut scan_stmt = conn.prepare(
            "SELECT f.rowid, f.name, f.path, m.size, m.ext
             FROM files f
             JOIN files_meta m ON f.rowid = m.rowid
             LIMIT ?1"
        )?;

        let fuzzy_candidates: Vec<(i64, String, String, i64, String)> = scan_stmt
            .query_map(params![FUZZY_SCAN_LIMIT], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let matcher = SkimMatcherV2::default();
        let query_lower = params.query.to_lowercase();

        for (rowid, name, path, size, ext) in fuzzy_candidates {
            if existing_rowids.contains(&rowid) {
                continue;
            }
            let score_name = matcher.fuzzy_match(&name.to_lowercase(), &query_lower).unwrap_or(0);
            let score_path = matcher.fuzzy_match(&path.to_lowercase(), &query_lower).unwrap_or(0);
            let score = score_name.max(score_path);
            if score > FUZZY_SCORE_THRESHOLD {
                let match_type = determine_match_type(&params.query, &name, &path, true);
                fts_results.push(SearchResult {
                    rowid,
                    name,
                    path,
                    size,
                    ext,
                    snippet: None,
                    match_type,
                    is_fuzzy: true,
                });
            }
        }
    }

    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

    if fts_results.is_empty() {
        ui::skip("No results found.");
        return Ok(());
    }

    // has_more: we fetched limit+1 results; if we got exactly that, there are more
    let has_more = fts_results.len() > limit;
    if has_more {
        fts_results.truncate(limit);
    }

    let total = fts_results.len();
    let top_count = total.min(3);

    // ── Top Results ───────────────────────────────────
    println!();
    println!("  {} {}",
        "──".truecolor(37, 99, 235),
        "Top Results".truecolor(96, 165, 250).bold()
    );
    println!();

    for (i, result) in fts_results.iter().take(top_count).enumerate() {
        let rank_str = format!("{}", i + 1).truecolor(96, 165, 250);
        let star = "★".truecolor(250, 204, 21);
        let path_str = result.path.truecolor(224, 242, 254);
        let badge = format!("{:<8}", result.match_type).truecolor(71, 85, 105);

        println!("   {}  {}   {}   {}", star, rank_str, path_str, badge);

        if result.match_type == "content" {
            if let Some(ref snip) = result.snippet {
                println!("        {}", snip.truecolor(71, 85, 105));
            }
        }
        println!();
    }

    // ── All Results ───────────────────────────────────
    if total > 3 {
        let section_title = format!("All Results · {:.1}ms", elapsed_ms);
        let fill = 44usize.saturating_sub(section_title.chars().count());
        let line = "─".repeat(fill);
        println!("\n  {} {} {}",
            "──".truecolor(37, 99, 235),
            section_title.truecolor(96, 165, 250).bold(),
            line.truecolor(37, 99, 235)
        );
        println!();

        for (i, result) in fts_results.iter().enumerate().skip(3) {
            let rank_str = format!("{:>3}", i + 1).truecolor(96, 165, 250);
            let path_str = result.path.truecolor(224, 242, 254);
            let badge = format!("{:<8}", result.match_type).truecolor(71, 85, 105);
            println!("      {}   {}   {}", rank_str, path_str, badge);
        }
        println!();
    } else {
        println!("  {} {} · {:.1}ms",
            "──".truecolor(37, 99, 235),
            format!("{} found", total).truecolor(96, 165, 250),
            elapsed_ms
        );
    }

    if has_more {
        ui::skip(&format!("More results available — use --limit {} to show more", limit * 2));
    }

    Ok(())
}

fn determine_match_type(query: &str, name: &str, path: &str, is_fuzzy: bool) -> String {
    if is_fuzzy {
        return "fuzzy".to_string();
    }
    let q = query.to_lowercase();
    let n = name.to_lowercase();
    let p = path.to_lowercase();
    if n.contains(&q) {
        "name".to_string()
    } else if p.contains(&q) {
        "path".to_string()
    } else {
        "content".to_string()
    }
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

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
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
