// src/commands/search.rs
use crate::ui;
use crate::config::ConfigManager;
use anyhow::{Result, Context};
use colored::Colorize;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use ignore::WalkBuilder;
use directories::ProjectDirs;
use chrono::Utc;
use nucleo_matcher::{Matcher, Config as NucleoConfig};
use nucleo_matcher::pattern::{Pattern, CaseMatching, Normalization};
use rayon::prelude::*;

// Text extensions whose content will be indexed
const TEXT_EXTENSIONS: &[&str] = &[
    "txt", "md", "rs", "py", "js", "ts", "jsx", "tsx", "toml", "yaml", "yml", "json",
    "html", "css", "scss", "sh", "bash", "zsh", "fish", "c", "cpp", "h", "hpp",
    "go", "java", "kt", "swift", "rb", "php", "sql", "xml", "ini", "conf", "log",
    "env", "gitignore", "dockerfile", "makefile",
];

const MAX_CONTENT_BYTES: usize = 256 * 1024; // 256 KB
const FUZZY_SCAN_LIMIT: i64 = 50_000;
const FUZZY_SCORE_THRESHOLD: u32 = 150;
const FUZZY_MAX_RESULTS: usize = 5;
const FUZZY_FALLBACK_THRESHOLD: usize = 5;
const PROGRESS_INTERVAL: u64 = 10_000;
const INDEX_BATCH_SIZE: usize = 500;

pub(crate) fn get_db_path() -> PathBuf {
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
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    migrate_schema(&conn)?;
    Ok(conn)
}

fn migrate_schema(conn: &Connection) -> Result<()> {
    // Check for 'content' column in FTS table
    let content_col_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name='content'",
        [],
        |r| r.get(0),
    ).unwrap_or(0);

    if content_col_count == 0 {
        conn.execute_batch("
            DROP TABLE IF EXISTS files;
            DROP TABLE IF EXISTS files_meta;
        ")?;
        ui::skip("Index schema updated — please run 'vg index' to rebuild.");
        return Ok(());
    }

    // Add modified_unix column if missing (non-destructive)
    let modified_unix_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('files_meta') WHERE name='modified_unix'",
        [],
        |r| r.get(0),
    ).unwrap_or(0);
    if modified_unix_count == 0 {
        conn.execute_batch(
            "ALTER TABLE files_meta ADD COLUMN modified_unix INTEGER NOT NULL DEFAULT 0;"
        )?;
    }

    // Add scope column if missing (non-destructive)
    let scope_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('files_meta') WHERE name='scope'",
        [],
        |r| r.get(0),
    ).unwrap_or(0);
    if scope_count == 0 {
        conn.execute_batch(
            "ALTER TABLE files_meta ADD COLUMN scope TEXT NOT NULL DEFAULT 'user';"
        )?;
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
            ext TEXT NOT NULL DEFAULT '',
            modified_unix INTEGER NOT NULL DEFAULT 0,
            scope TEXT NOT NULL DEFAULT 'user'
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
            let s = String::from_utf8_lossy(truncated);
            s.chars().filter(|&c| c != '\0').collect()
        }
        Err(_) => String::new(),
    }
}

struct FileEntry {
    name: String,
    path: String,
    size: i64,
    modified: String,
    modified_unix: i64,
    ext: String,
    content: String,
    scope: &'static str,
}

pub fn build_index(user_paths: Vec<PathBuf>, config: &ConfigManager) -> Result<()> {
    ui::print_header("INDEX BUILD");

    let conn = open_db()?;
    init_db(&conn)?;
    conn.execute_batch("DELETE FROM files; DELETE FROM files_meta;")?;

    let ignore_patterns = config.config.search.ignore_patterns.clone();
    let max_depth = config.config.search.max_depth;
    let exclude_hidden = config.config.search.exclude_hidden;
    let full_system = config.config.search.full_system_index;
    let system_roots: Vec<PathBuf> = config.config.search.system_index_roots
        .iter().map(PathBuf::from).collect();
    let system_excludes = config.config.search.system_exclude_paths.clone();

    let mut user_count: u64 = 0;
    let mut system_count: u64 = 0;
    let index_start = std::time::Instant::now();

    // ── User paths (scope = "user") ──────────────────────────────
    for base_path in &user_paths {
        index_path_into(
            base_path, "user", Some(max_depth), exclude_hidden,
            &ignore_patterns, &[], &conn, &mut user_count, &index_start,
        )?;
    }

    // ── System paths (scope = "system") ──────────────────────────
    if full_system {
        println!();
        ui::info_line("Mode", "Full system index enabled — walking entire filesystem");
        ui::skip("This may take several minutes and use significant disk space.");
        println!();
        for root in &system_roots {
            if !root.exists() { continue; }
            index_path_into(
                root, "system", None, false,
                &[], &system_excludes, &conn, &mut system_count, &index_start,
            )?;
        }
        // Subtract user-path files that got double-counted
        // (WalkBuilder will enter user dirs again — mark them system, that's fine,
        //  but we skip paths already indexed under user scope to avoid duplicates)
    }

    let total = user_count + system_count;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR REPLACE INTO index_meta(key, value) VALUES ('last_updated', ?1)",
        params![now],
    )?;
    conn.execute(
        "INSERT OR REPLACE INTO index_meta(key, value) VALUES ('full_system_index', ?1)",
        params![if full_system { "true" } else { "false" }],
    )?;
    let paths_str = user_paths.iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    conn.execute(
        "INSERT OR REPLACE INTO index_meta(key, value) VALUES ('indexed_paths', ?1)",
        params![paths_str],
    )?;

    println!();
    if total == 0 {
        ui::fail("No files indexed — all configured paths were missing or empty.");
        ui::skip("Update your paths:  vg config edit");
        ui::skip("Or specify directly: vg index --paths /home/you");
    } else {
        let system_note = if full_system {
            format!(" · {} system", format_number(system_count))
        } else {
            String::new()
        };
        ui::success(&format!(
            "Indexed {} files ({} user{})",
            format_number(total), format_number(user_count), system_note
        ));
        if !full_system {
            ui::skip("Tip: set full_system_index = true in config to index the whole system");
        }
        ui::info_line("Database", &get_db_path().display().to_string());
    }
    Ok(())
}

fn is_excluded(path_str: &str, excludes: &[String]) -> bool {
    excludes.iter().any(|ex| path_str == ex.as_str() || path_str.starts_with(&format!("{}/", ex)))
}

fn index_path_into(
    base_path: &PathBuf,
    scope: &'static str,
    max_depth: Option<usize>,
    exclude_hidden: bool,
    ignore_patterns: &[String],
    hard_excludes: &[String],
    conn: &Connection,
    count: &mut u64,
    index_start: &std::time::Instant,
) -> Result<()> {
    if !base_path.exists() {
        if scope == "user" {
            ui::skip(&format!("Path not found: {}", base_path.display()));
        }
        return Ok(());
    }
    if scope == "user" {
        ui::info_line("Indexing", &base_path.display().to_string());
    }

    let mut walker = WalkBuilder::new(base_path);
    walker
        .max_depth(max_depth)
        .hidden(exclude_hidden)
        .git_ignore(scope == "user")
        .git_global(scope == "user")
        .ignore(scope == "user")
        .follow_links(false);
    let walker = walker.build();

    let mut pending: Vec<(String, String, i64, String, i64, String)> = Vec::new();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path_str = entry.path().to_string_lossy().to_string();

        // Hard-exclude certain filesystem paths (e.g. /proc, /sys)
        if is_excluded(&path_str, hard_excludes) {
            continue;
        }

        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }

        // User ignore patterns (substring match)
        if ignore_patterns.iter().any(|p| path_str.contains(p.as_str())) {
            continue;
        }

        if let Ok(meta) = entry.metadata() {
            let name = entry.file_name().to_string_lossy().to_string();
            let size = meta.len() as i64;
            let modified_unix = meta.modified()
                .map(|t| t.duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0))
                .unwrap_or(0);
            let modified = meta.modified()
                .map(|t| chrono::DateTime::<Utc>::from(t).to_rfc3339())
                .unwrap_or_default();
            let ext = entry.path()
                .extension()
                .map(|s| s.to_string_lossy().to_lowercase().to_string())
                .unwrap_or_default();
            pending.push((name, path_str, size, modified, modified_unix, ext));
        }
    }

    for chunk in pending.chunks(INDEX_BATCH_SIZE) {
        let entries: Vec<FileEntry> = chunk
            .par_iter()
            .map(|(name, path, size, modified, modified_unix, ext)| {
                let content = read_file_content(path, ext);
                FileEntry {
                    name: name.clone(),
                    path: path.clone(),
                    size: *size,
                    modified: modified.clone(),
                    modified_unix: *modified_unix,
                    ext: ext.clone(),
                    content,
                    scope,
                }
            })
            .collect();

        for fe in entries {
            conn.execute(
                "INSERT INTO files(name, path, content) VALUES (?1, ?2, ?3)",
                params![fe.name, fe.path, fe.content],
            )?;
            let rowid = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO files_meta(rowid, size, modified, ext, modified_unix, scope) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![rowid, fe.size, fe.modified, fe.ext, fe.modified_unix, fe.scope],
            )?;
            *count += 1;

            if *count % PROGRESS_INTERVAL == 0 {
                let elapsed = index_start.elapsed().as_secs_f64();
                let rate = if elapsed > 0.0 { *count as f64 / elapsed } else { 0.0 };
                ui::info_line("Progress", &format!("{} files ({:.0}/s)...", format_number(*count), rate));
            }
        }
    }
    Ok(())
}

pub struct SearchParams {
    pub query: String,
    pub ext: Option<String>,
    pub path_filter: Option<String>,
    pub limit: Option<usize>,
    pub verbose: bool,
    /// Include system-indexed paths in results (default: user only)
    pub all_scopes: bool,
}

#[derive(Debug)]
struct SearchResult {
    rowid: i64,
    name: String,
    path: String,
    size: i64,
    #[allow(dead_code)]
    ext: String,
    snippet: Option<String>,
    match_type: String,
    is_fuzzy: bool,
    bm25: f64,
    modified_unix: i64,
    final_score: f64,
    scope: String,
}

fn validate_ext_part(ext: &str) -> bool {
    ext.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

pub(crate) fn sanitize_fts_query(query: &str) -> String {
    let trimmed = query.trim();
    // Phrase search: user wrapped in quotes
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() > 2 {
        let inner = &trimmed[1..trimmed.len() - 1];
        let inner_escaped = inner.replace('"', "");
        return format!("\"{}\"", inner_escaped);
    }
    let clean: String = query.chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '.' || *c == '_' || *c == '-')
        .collect();
    if clean.trim().is_empty() {
        return query.to_string();
    }
    // Multi-word: each token gets prefix search; FTS5 AND is implicit
    clean.split_whitespace()
        .map(|token| format!("{}*", token))
        .collect::<Vec<_>>()
        .join(" AND ")
}

pub(crate) fn compute_score(bm25: f64, name: &str, path: &str, query: &str, modified_unix: i64) -> f64 {
    let base = -bm25; // FTS5 BM25 is negative; negate so higher = better
    let query_lower = query.to_lowercase();
    let name_lower = name.to_lowercase();
    let path_lower = path.to_lowercase();
    let multiplier = if name_lower == query_lower { 5.0 }
        else if name_lower.starts_with(&query_lower) { 3.0 }
        else if name_lower.contains(&query_lower) { 2.0 }
        else if path_lower.contains(&query_lower) { 1.5 }
        else { 1.0 };
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let age_days = if modified_unix > 0 { (now_unix - modified_unix).max(0) / 86400 } else { 9999 };
    let recency = if age_days < 7 { 200.0 }
        else if age_days < 30 { 100.0 }
        else if age_days < 90 { 30.0 }
        else { 0.0 };
    base * multiplier + recency
}

pub(crate) fn determine_match_type(query: &str, name: &str, path: &str, is_fuzzy: bool) -> String {
    if is_fuzzy {
        return "fuzzy".to_string();
    }
    let q = query.to_lowercase();
    let n = name.to_lowercase();
    let p = path.to_lowercase();
    if n.contains(&q) { "name".to_string() }
    else if p.contains(&q) { "path".to_string() }
    else { "content".to_string() }
}

pub(crate) fn fmt_age(modified_unix: i64) -> String {
    if modified_unix == 0 { return "—".to_string(); }
    let now_unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let diff = (now_unix - modified_unix).max(0);
    if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else if diff < 86400 * 30 {
        format!("{}d ago", diff / 86400)
    } else if diff < 86400 * 365 {
        format!("{}mo ago", diff / (86400 * 30))
    } else {
        format!("{}y ago", diff / (86400 * 365))
    }
}

pub fn search(params: SearchParams, _config: &ConfigManager) -> Result<()> {
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
    let limit = params.limit.unwrap_or(10);
    let fetch_limit = (limit * 2) as i64; // fetch 2× for reranking

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

    let path_pattern = params.path_filter.as_ref().map(|p| format!("{}%", p));

    let sql = {
        let mut conditions = vec!["files MATCH ?1".to_string()];
        if !params.all_scopes {
            conditions.push("m.scope = 'user'".to_string());
        }
        if let Some(ref ec) = ext_clause {
            conditions.push(ec.clone());
        }
        if path_pattern.is_some() {
            conditions.push("f.path LIKE ?3".to_string());
        }
        format!(
            "SELECT f.rowid, f.name, f.path, m.size, m.ext,
                    snippet(files, 2, '[', ']', '...', 20) as snip,
                    bm25(files, 10.0, 5.0, 1.0) as bm25_score,
                    m.modified_unix,
                    m.scope
             FROM files f
             JOIN files_meta m ON f.rowid = m.rowid
             WHERE {}
             ORDER BY bm25(files, 10.0, 5.0, 1.0)
             LIMIT ?2",
            conditions.join(" AND ")
        )
    };

    let fts_start = std::time::Instant::now();

    let mut fts_results: Vec<SearchResult> = {
        let mut stmt = conn.prepare(&sql)?;

        type Row = (i64, String, String, i64, String, String, f64, i64, String);
        let map_row = |row: &rusqlite::Row| Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, f64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, String>(8)?,
        ));
        let rows: Vec<Row> = if path_pattern.is_some() {
            stmt.query_map(params![fts_query, fetch_limit, path_pattern.as_deref()], map_row)?
                .filter_map(|r| r.ok()).collect()
        } else {
            stmt.query_map(params![fts_query, fetch_limit], map_row)?
                .filter_map(|r| r.ok()).collect()
        };

        rows.into_iter().map(|(rowid, name, path, size, ext, snip, bm25, modified_unix, scope)| {
            let match_type = determine_match_type(&params.query, &name, &path, false);
            let snippet = if snip.contains('[') { Some(snip) } else { None };
            let final_score = compute_score(bm25, &name, &path, &params.query, modified_unix);
            SearchResult { rowid, name, path, size, ext, snippet, match_type, is_fuzzy: false, bm25, modified_unix, final_score, scope }
        }).collect()
    };

    let fts_elapsed = fts_start.elapsed();

    // Sort by final_score descending
    fts_results.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal));

    let fuzzy_start = std::time::Instant::now();

    // Fuzzy fallback if not enough FTS results
    if fts_results.len() < FUZZY_FALLBACK_THRESHOLD {
        let existing_rowids: std::collections::HashSet<i64> = fts_results.iter().map(|r| r.rowid).collect();

        let scope_filter = if params.all_scopes { "" } else { " AND m.scope = 'user'" };
        let fuzzy_sql = format!(
            "SELECT f.rowid, f.name, f.path, m.size, m.ext, m.modified_unix, m.scope
             FROM files f JOIN files_meta m ON f.rowid = m.rowid
             WHERE 1=1{} LIMIT ?1",
            scope_filter
        );
        let mut scan_stmt = conn.prepare(&fuzzy_sql)?;

        let fuzzy_candidates: Vec<(i64, String, String, i64, String, i64, String)> = scan_stmt
            .query_map(params![FUZZY_SCAN_LIMIT], |row| Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
            )))?
            .filter_map(|r| r.ok())
            .collect();

        let mut matcher = Matcher::new(NucleoConfig::DEFAULT.match_paths());
        let pattern = Pattern::parse(&params.query, CaseMatching::Smart, Normalization::Smart);

        let mut fuzzy_scored: Vec<(u32, i64, String, String, i64, String, i64, String)> = fuzzy_candidates
            .into_iter()
            .filter(|(rowid, _, _, _, _, _, _)| !existing_rowids.contains(rowid))
            .filter_map(|(rowid, name, path, size, ext, modified_unix, scope)| {
                let haystack = nucleo_matcher::Utf32String::from(name.as_str());
                let score = pattern.score(haystack.slice(..), &mut matcher)?;
                if score >= FUZZY_SCORE_THRESHOLD {
                    Some((score, rowid, name, path, size, ext, modified_unix, scope))
                } else {
                    None
                }
            })
            .collect();

        fuzzy_scored.sort_by(|a, b| b.0.cmp(&a.0));
        fuzzy_scored.truncate(FUZZY_MAX_RESULTS);

        for (_, rowid, name, path, size, ext, modified_unix, scope) in fuzzy_scored {
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
                bm25: 0.0,
                modified_unix,
                final_score: 0.0,
                scope,
            });
        }
    }

    let fuzzy_elapsed = fuzzy_start.elapsed();

    let rank_start = std::time::Instant::now();
    let elapsed = start.elapsed();
    let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
    let rank_elapsed = rank_start.elapsed();

    if fts_results.is_empty() {
        ui::skip("No results found.");
        return Ok(());
    }

    let has_more = fts_results.len() > limit;
    if has_more {
        fts_results.truncate(limit);
    }

    let total = fts_results.len();
    let top_count = total.min(3);

    println!();
    println!("  {} {}",
        "──".truecolor(37, 99, 235),
        "Top Results".truecolor(96, 165, 250).bold()
    );
    println!();

    for (i, result) in fts_results.iter().take(top_count).enumerate() {
        let rank_str = format!("{}", i + 1).truecolor(96, 165, 250);
        let star = "★".truecolor(250, 204, 21);
        let path_colored = color_by_match_type(&result.path, &result.match_type);
        let badge = format_badge(&result.match_type);
        let age = fmt_age(result.modified_unix);
        let size_str = fmt_bytes(result.size as u64);

        let scope_badge = if result.scope == "system" {
            " [sys]".truecolor(148, 103, 189)
        } else {
            "".truecolor(0, 0, 0) // invisible
        };
        println!("   {}  {}   {}   {}  {}  {}{}",
            star, rank_str, path_colored, badge,
            size_str.truecolor(100, 116, 139),
            age.truecolor(100, 116, 139),
            scope_badge,
        );

        if !result.is_fuzzy {
            if let Some(ref snip) = result.snippet {
                println!("        {}", snip.truecolor(71, 85, 105));
            }
        }

        if params.verbose {
            println!("        {} bm25={:.2}  score={:.1}",
                "score:".truecolor(71, 85, 105),
                result.bm25,
                result.final_score,
            );
        }

        println!();
    }

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
            let path_colored = color_by_match_type(&result.path, &result.match_type);
            let badge = format_badge(&result.match_type);
            let age = fmt_age(result.modified_unix);
            let size_str = fmt_bytes(result.size as u64);
            let scope_badge = if result.scope == "system" {
                " [sys]".truecolor(148, 103, 189)
            } else {
                "".truecolor(0, 0, 0)
            };
            println!("      {}   {}   {}  {}  {}{}",
                rank_str, path_colored, badge,
                size_str.truecolor(100, 116, 139),
                age.truecolor(100, 116, 139),
                scope_badge,
            );
        }
        println!();
    } else {
        println!("  {} {} · {:.1}ms",
            "──".truecolor(37, 99, 235),
            format!("{} found", total).truecolor(96, 165, 250),
            elapsed_ms
        );
    }

    if params.verbose {
        println!();
        println!("  {} FTS: {:.1}ms  Fuzzy: {:.1}ms  Rank: {:.1}ms",
            "timing:".truecolor(71, 85, 105),
            fts_elapsed.as_secs_f64() * 1000.0,
            fuzzy_elapsed.as_secs_f64() * 1000.0,
            rank_elapsed.as_secs_f64() * 1000.0,
        );
    }

    if has_more {
        ui::skip(&format!("More results available — use --limit {} to show more", limit * 2));
    }

    Ok(())
}

fn color_by_match_type(path: &str, match_type: &str) -> colored::ColoredString {
    match match_type {
        "name"  => path.green(),
        "fuzzy" => path.yellow(),
        "path"  => path.cyan(),
        _       => path.truecolor(224, 242, 254),
    }
}

fn format_badge(match_type: &str) -> colored::ColoredString {
    let badge = format!("{:<8}", match_type);
    match match_type {
        "name"  => badge.green(),
        "fuzzy" => badge.yellow(),
        "path"  => badge.cyan(),
        _       => badge.truecolor(71, 85, 105),
    }
}

pub(crate) fn fmt_bytes(bytes: u64) -> String {
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
        if i > 0 && i % 3 == 0 { result.push(','); }
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

    if let Ok(meta) = std::fs::metadata(&db_path) {
        ui::info_line("DB size", &fmt_bytes(meta.len()));
    }

    Ok(())
}
