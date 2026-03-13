// src/commands/search_tui.rs
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use rusqlite::params;
use std::io::{self, IsTerminal};

use crate::config::ConfigManager;
use super::search::{
    get_db_path, sanitize_fts_query, compute_score, determine_match_type, fmt_age, fmt_bytes,
};

const PREVIEW_MAX_BYTES: u64 = 2 * 1024 * 1024; // 2 MB
const DEBOUNCE_MS: u64 = 150;

#[derive(Debug, Clone)]
struct TuiResult {
    name: String,
    path: String,
    size: i64,
    match_type: String,
    modified_unix: i64,
    scope: String,
}

enum Focus {
    Results,
    Preview,
}

struct TuiState {
    query: String,
    cursor_pos: usize,
    results: Vec<TuiResult>,
    selected_idx: usize,
    focus: Focus,
    preview_scroll: u16,
    search_elapsed_ms: f64,
    last_query: String,
    last_search_time: std::time::Instant,
    needs_search: bool,
    preview_lines: Vec<String>,
    /// When true, search includes system-indexed files
    all_scopes: bool,
}

impl TuiState {
    fn new(initial_query: &str) -> Self {
        TuiState {
            query: initial_query.to_string(),
            cursor_pos: initial_query.chars().count(),
            results: Vec::new(),
            selected_idx: 0,
            focus: Focus::Results,
            preview_scroll: 0,
            search_elapsed_ms: 0.0,
            last_query: String::new(),
            last_search_time: std::time::Instant::now()
                .checked_sub(std::time::Duration::from_secs(1))
                .unwrap_or_else(std::time::Instant::now),
            needs_search: !initial_query.is_empty(),
            preview_lines: Vec::new(),
            all_scopes: false,
        }
    }

    fn selected_path(&self) -> Option<&str> {
        self.results.get(self.selected_idx).map(|r| r.path.as_str())
    }

    fn load_preview(&mut self) {
        self.preview_lines.clear();
        self.preview_scroll = 0;
        let path = match self.selected_path() {
            Some(p) => p.to_string(),
            None => return,
        };
        // Guard: 2 MB max
        if let Ok(m) = std::fs::metadata(&path) {
            if m.len() > PREVIEW_MAX_BYTES {
                self.preview_lines.push(format!("[File too large to preview: {}]", fmt_bytes(m.len())));
                return;
            }
        }
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                for line in content.lines() {
                    self.preview_lines.push(line.to_string());
                }
            }
            Err(e) => {
                self.preview_lines.push(format!("[Cannot read file: {}]", e));
            }
        }
    }
}

/// RAII guard that restores terminal state even on panic.
struct TermGuard;

impl TermGuard {
    fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(TermGuard)
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

fn do_search(query: &str, all_scopes: bool, conn: &rusqlite::Connection) -> (Vec<TuiResult>, f64) {
    if query.trim().is_empty() {
        return (Vec::new(), 0.0);
    }
    let start = std::time::Instant::now();
    let fts_query = sanitize_fts_query(query);
    let limit = 50i64;
    let scope_clause = if all_scopes { "" } else { " AND m.scope = 'user'" };

    let sql = format!(
        "SELECT f.rowid, f.name, f.path, m.size, m.ext,
                bm25(files, 10.0, 5.0, 1.0) as bm25_score,
                m.modified_unix, m.scope
         FROM files f
         JOIN files_meta m ON f.rowid = m.rowid
         WHERE files MATCH ?1{}
         ORDER BY bm25(files, 10.0, 5.0, 1.0)
         LIMIT ?2",
        scope_clause
    );

    let mut results: Vec<TuiResult> = Vec::new();

    if let Ok(mut stmt) = conn.prepare(&sql) {
        type Row = (i64, String, String, i64, String, f64, i64, String);
        let rows: Vec<Row> = stmt
            .query_map(params![fts_query, limit], |row| Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, f64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
            )))
            .map(|iter| iter.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();

        let mut scored: Vec<(f64, String, String, i64, String, i64, String)> = rows
            .into_iter()
            .map(|(_, name, path, size, _ext, bm25, modified_unix, scope)| {
                let score = compute_score(bm25, &name, &path, query, modified_unix);
                let match_type = determine_match_type(query, &name, &path, false);
                (score, name, path, size, match_type, modified_unix, scope)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        for (_, name, path, size, match_type, modified_unix, scope) in scored {
            results.push(TuiResult { name, path, size, match_type, modified_unix, scope });
        }
    }

    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    (results, elapsed)
}

fn cursor_display_col(s: &str, char_pos: usize) -> u16 {
    s.chars().take(char_pos).count() as u16
}

fn open_selected(state: &TuiState) {
    if let Some(path) = state.selected_path() {
        let editor = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| "nano".to_string());
        let _ = std::process::Command::new(&editor).arg(path).status();
    }
}

fn render(f: &mut Frame, state: &TuiState) {
    let size = f.area();

    // Layout: input bar (3) | main area (fill) | status bar (1)
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(size);

    // ── Input bar ──
    let input_block = Block::default()
        .title(" vg search ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    let input_text = format!("> {}", state.query);
    let input = Paragraph::new(input_text.as_str())
        .block(input_block)
        .style(Style::default().fg(Color::White));
    f.render_widget(input, outer[0]);

    // Cursor position (char-safe)
    let cursor_x = outer[0].x + 1 + 2 + cursor_display_col(&state.query, state.cursor_pos);
    let cursor_y = outer[0].y + 1;
    f.set_cursor_position((cursor_x, cursor_y));

    // ── Main: results (40%) | preview (60%) ──
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(outer[1]);

    // Results list
    let results_border = match state.focus {
        Focus::Results => Style::default().fg(Color::Blue),
        Focus::Preview => Style::default().fg(Color::DarkGray),
    };
    let results_title = format!(" {} results  {:.1}ms ", state.results.len(), state.search_elapsed_ms);
    let results_block = Block::default()
        .title(results_title)
        .borders(Borders::ALL)
        .border_style(results_border);

    let items: Vec<ListItem> = state.results.iter().enumerate().map(|(i, r)| {
        let selected = i == state.selected_idx;
        let name_style = if selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let type_color = match r.match_type.as_str() {
            "name"  => Color::Green,
            "fuzzy" => Color::Yellow,
            "path"  => Color::Cyan,
            _       => Color::DarkGray,
        };
        let age = fmt_age(r.modified_unix);
        let sys_span = if r.scope == "system" {
            Span::styled(" sys", Style::default().fg(Color::Rgb(148, 103, 189)))
        } else {
            Span::raw("")
        };
        let line = Line::from(vec![
            Span::styled(format!("{:<6}", r.match_type), Style::default().fg(type_color)),
            Span::styled(r.name.clone(), name_style),
            sys_span,
            Span::styled(format!("  {}", age), Style::default().fg(Color::DarkGray)),
        ]);
        ListItem::new(line)
    }).collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_idx));
    let list = List::new(items)
        .block(results_block)
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(list, main[0], &mut list_state);

    // Preview panel
    let preview_border = match state.focus {
        Focus::Preview => Style::default().fg(Color::Blue),
        Focus::Results => Style::default().fg(Color::DarkGray),
    };
    let preview_title = if let Some(r) = state.results.get(state.selected_idx) {
        let size_str = fmt_bytes(r.size as u64);
        format!(" {}  {} ", r.path, size_str)
    } else {
        " preview ".to_string()
    };
    let preview_block = Block::default()
        .title(preview_title)
        .borders(Borders::ALL)
        .border_style(preview_border);

    let query_lower = state.query.to_lowercase();
    let preview_lines: Vec<Line> = state.preview_lines.iter().enumerate().map(|(i, line)| {
        let is_match = !query_lower.is_empty() && line.to_lowercase().contains(&query_lower);
        let line_style = if is_match {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        };
        Line::from(vec![
            Span::styled(format!("{:>4} │ ", i + 1), Style::default().fg(Color::DarkGray)),
            Span::styled(line.clone(), line_style),
        ])
    }).collect();

    let preview = Paragraph::new(preview_lines)
        .block(preview_block)
        .scroll((state.preview_scroll, 0));
    f.render_widget(preview, main[1]);

    // ── Status bar ──
    let scope_indicator = if state.all_scopes {
        "  [ALL]"
    } else {
        "  [user]"
    };
    let status_text = format!(
        "↑↓ navigate  Enter open  Tab toggle focus  ^A toggle scope{}  Esc exit",
        scope_indicator
    );
    let scope_color = if state.all_scopes { Color::Rgb(148, 103, 189) } else { Color::DarkGray };
    let status = Paragraph::new(status_text).style(Style::default().fg(scope_color));
    f.render_widget(status, outer[2]);
}

pub fn run_interactive(config: &ConfigManager) -> Result<()> {
    run_interactive_with_query(config, "")
}

pub fn run_interactive_with_query(_config: &ConfigManager, initial_query: &str) -> Result<()> {
    // TTY check
    if !io::stdout().is_terminal() {
        println!("vg search: interactive mode requires a terminal (stdout is not a TTY)");
        return Ok(());
    }

    let db_path = get_db_path();
    if !db_path.exists() {
        eprintln!("No index found. Run 'vg index' first.");
        return Ok(());
    }

    let conn = rusqlite::Connection::open(&db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

    let _guard = TermGuard::new()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut state = TuiState::new(initial_query);

    // Perform initial search if query was provided
    if !initial_query.is_empty() {
        let (results, elapsed) = do_search(initial_query, state.all_scopes, &conn);
        state.results = results;
        state.search_elapsed_ms = elapsed;
        state.last_query = initial_query.to_string();
        state.needs_search = false;
        state.load_preview();
    }

    loop {
        terminal.draw(|f| render(f, &state))?;

        // Debounced search: trigger after DEBOUNCE_MS of inactivity
        if state.needs_search
            && state.last_search_time.elapsed().as_millis() as u64 >= DEBOUNCE_MS
        {
            if state.query != state.last_query || state.needs_search {
                let (results, elapsed) = do_search(&state.query, state.all_scopes, &conn);
                state.results = results;
                state.search_elapsed_ms = elapsed;
                state.last_query = state.query.clone();
                state.selected_idx = 0;
                state.load_preview();
            }
            state.needs_search = false;
        }

        let timeout = if state.needs_search {
            std::time::Duration::from_millis(DEBOUNCE_MS)
        } else {
            std::time::Duration::from_millis(200)
        };

        if !event::poll(timeout)? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => break,

                // Ctrl+A: toggle all-scopes (user-only ↔ entire system)
                (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                    state.all_scopes = !state.all_scopes;
                    state.last_search_time = std::time::Instant::now();
                    state.needs_search = true;
                    state.last_query.clear(); // force re-search
                }

                (KeyCode::Enter, _) => {
                    // Temporarily leave TUI, open editor, then restore
                    let _ = disable_raw_mode();
                    let _ = execute!(io::stdout(), LeaveAlternateScreen);
                    open_selected(&state);
                    let _ = enable_raw_mode();
                    let _ = execute!(io::stdout(), EnterAlternateScreen);
                    terminal.clear()?;
                }

                (KeyCode::Tab, _) => {
                    state.focus = match state.focus {
                        Focus::Results => Focus::Preview,
                        Focus::Preview => Focus::Results,
                    };
                }

                (KeyCode::Up, _) => match state.focus {
                    Focus::Results => {
                        if state.selected_idx > 0 {
                            state.selected_idx -= 1;
                            state.load_preview();
                        }
                    }
                    Focus::Preview => {
                        state.preview_scroll = state.preview_scroll.saturating_sub(1);
                    }
                },

                (KeyCode::Down, _) => match state.focus {
                    Focus::Results => {
                        if state.selected_idx + 1 < state.results.len() {
                            state.selected_idx += 1;
                            state.load_preview();
                        }
                    }
                    Focus::Preview => {
                        state.preview_scroll = state.preview_scroll.saturating_add(1);
                    }
                },

                (KeyCode::Left, _) => {
                    if state.cursor_pos > 0 {
                        state.cursor_pos -= 1;
                    }
                }

                (KeyCode::Right, _) => {
                    if state.cursor_pos < state.query.chars().count() {
                        state.cursor_pos += 1;
                    }
                }

                (KeyCode::Backspace, _) => {
                    if state.cursor_pos > 0 {
                        let byte_pos = state.query.char_indices()
                            .nth(state.cursor_pos - 1)
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        state.query.remove(byte_pos);
                        state.cursor_pos -= 1;
                        state.last_search_time = std::time::Instant::now();
                        state.needs_search = true;
                    }
                }

                (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                    let byte_pos = state.query.char_indices()
                        .nth(state.cursor_pos)
                        .map(|(i, _)| i)
                        .unwrap_or(state.query.len());
                    state.query.insert(byte_pos, c);
                    state.cursor_pos += 1;
                    state.last_search_time = std::time::Instant::now();
                    state.needs_search = true;
                }

                _ => {}
            }
        }
    }

    Ok(())
}
