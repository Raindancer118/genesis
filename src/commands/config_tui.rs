// src/commands/config_tui.rs
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::io::{self, IsTerminal};
use crate::config::ConfigManager;

// ── Field definitions ──────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq)]
enum FieldKind { Bool, Text, ReadOnly }

struct FieldDef {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    kind: FieldKind,
}

// Separator between sections
struct SectionDef { title: &'static str }

enum Row { Section(SectionDef), Field(FieldDef) }

fn rows() -> Vec<Row> {
    vec![
        Row::Section(SectionDef { title: "Search — Indexing" }),
        Row::Field(FieldDef {
            key: "search.full_system_index",
            label: "full_system_index",
            description: "Index the entire filesystem (not just home). When enabled, vg index walks system_index_roots. WARNING: first run can take minutes.",
            kind: FieldKind::Bool,
        }),
        Row::Field(FieldDef {
            key: "search.default_paths",
            label: "default_paths",
            description: "Paths indexed as user scope (searched by default). Comma-separated list of absolute paths.",
            kind: FieldKind::Text,
        }),
        Row::Field(FieldDef {
            key: "search.system_index_roots",
            label: "system_index_roots",
            description: "Root paths to walk when full_system_index is on. Default: /  (comma-separated).",
            kind: FieldKind::Text,
        }),
        Row::Field(FieldDef {
            key: "search.system_exclude_paths",
            label: "system_exclude_paths",
            description: "Paths NEVER indexed even with full_system_index. Add /proc /sys /dev /run etc. (comma-separated).",
            kind: FieldKind::Text,
        }),
        Row::Field(FieldDef {
            key: "search.max_depth",
            label: "max_depth",
            description: "How many directory levels to descend when indexing user paths. 0 = unlimited.",
            kind: FieldKind::Text,
        }),
        Row::Field(FieldDef {
            key: "search.exclude_hidden",
            label: "exclude_hidden",
            description: "Skip hidden files and directories (names starting with .) when indexing user paths.",
            kind: FieldKind::Bool,
        }),
        Row::Section(SectionDef { title: "Search — Results" }),
        Row::Field(FieldDef {
            key: "search.max_results",
            label: "max_results",
            description: "Maximum number of results returned by vg search.",
            kind: FieldKind::Text,
        }),
        Row::Field(FieldDef {
            key: "search.fuzzy_threshold",
            label: "fuzzy_threshold",
            description: "Minimum nucleo fuzzy score (0–1000) for a fuzzy match to appear.",
            kind: FieldKind::Text,
        }),
        Row::Section(SectionDef { title: "System" }),
        Row::Field(FieldDef {
            key: "system.auto_confirm_update",
            label: "auto_confirm_update",
            description: "Automatically confirm package manager updates without prompting.",
            kind: FieldKind::Bool,
        }),
        Row::Section(SectionDef { title: "Analytics" }),
        Row::Field(FieldDef {
            key: "analytics.enabled",
            label: "enabled",
            description: "Send an anonymous daily ping to analytics.volantic.de (opt-in).",
            kind: FieldKind::Bool,
        }),
        Row::Field(FieldDef {
            key: "analytics.track_commands",
            label: "track_commands",
            description: "Also track which vg commands are used (still anonymous, no arguments).",
            kind: FieldKind::Bool,
        }),
        Row::Field(FieldDef {
            key: "analytics.client_id",
            label: "client_id",
            description: "Anonymous client identifier (auto-generated SHA256 hash, read-only).",
            kind: FieldKind::ReadOnly,
        }),
    ]
}

// ── Value access ────────────────────────────────────────────────────────────────

fn get_value(key: &str, config: &ConfigManager) -> String {
    match key {
        "search.full_system_index"    => config.config.search.full_system_index.to_string(),
        "search.default_paths"        => config.config.search.default_paths.join(", "),
        "search.system_index_roots"   => config.config.search.system_index_roots.join(", "),
        "search.system_exclude_paths" => config.config.search.system_exclude_paths.join(", "),
        "search.max_depth"            => config.config.search.max_depth.to_string(),
        "search.exclude_hidden"       => config.config.search.exclude_hidden.to_string(),
        "search.max_results"          => config.config.search.max_results.to_string(),
        "search.fuzzy_threshold"      => config.config.search.fuzzy_threshold.to_string(),
        "system.auto_confirm_update"  => config.config.system.auto_confirm_update.to_string(),
        "analytics.enabled"           => config.config.analytics.enabled.to_string(),
        "analytics.track_commands"    => config.config.analytics.track_commands.to_string(),
        "analytics.client_id"         => format!("{}…", &config.config.analytics.client_id.chars().take(8).collect::<String>()),
        _ => "—".to_string(),
    }
}

fn toggle_bool(key: &str, config: &mut ConfigManager) {
    match key {
        "search.full_system_index"    => config.config.search.full_system_index    = !config.config.search.full_system_index,
        "search.exclude_hidden"       => config.config.search.exclude_hidden       = !config.config.search.exclude_hidden,
        "system.auto_confirm_update"  => config.config.system.auto_confirm_update  = !config.config.system.auto_confirm_update,
        "analytics.enabled"           => config.config.analytics.enabled           = !config.config.analytics.enabled,
        "analytics.track_commands"    => config.config.analytics.track_commands    = !config.config.analytics.track_commands,
        _ => {}
    }
}

fn apply_text(key: &str, value: &str, config: &mut ConfigManager) {
    let vec_val = || -> Vec<String> {
        value.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    };
    match key {
        "search.default_paths"        => config.config.search.default_paths        = vec_val(),
        "search.system_index_roots"   => config.config.search.system_index_roots   = vec_val(),
        "search.system_exclude_paths" => config.config.search.system_exclude_paths = vec_val(),
        "search.max_depth"            => { if let Ok(n) = value.parse() { config.config.search.max_depth = n; } }
        "search.max_results"          => { if let Ok(n) = value.parse() { config.config.search.max_results = n; } }
        "search.fuzzy_threshold"      => { if let Ok(n) = value.parse() { config.config.search.fuzzy_threshold = n; } }
        _ => {}
    }
}

// ── State ───────────────────────────────────────────────────────────────────────

enum Mode {
    Browse,
    Editing { key: &'static str, input: String, cursor: usize },
}

struct TuiState {
    rows: Vec<Row>,
    /// Indices of rows that are Fields (selectable)
    selectable: Vec<usize>,
    /// Which selectable index is highlighted
    sel: usize,
    mode: Mode,
    dirty: bool,
    message: Option<String>,
    quit: bool,
}

impl TuiState {
    fn new() -> Self {
        let rows = rows();
        let selectable: Vec<usize> = rows.iter().enumerate()
            .filter_map(|(i, r)| if matches!(r, Row::Field(_)) { Some(i) } else { None })
            .collect();
        Self { rows, selectable, sel: 0, mode: Mode::Browse, dirty: false, message: None, quit: false }
    }

    fn selected_row_idx(&self) -> usize {
        self.selectable[self.sel]
    }

    fn selected_field(&self) -> Option<&FieldDef> {
        match &self.rows[self.selected_row_idx()] {
            Row::Field(f) => Some(f),
            _ => None,
        }
    }

    fn move_up(&mut self) {
        if self.sel > 0 { self.sel -= 1; }
    }

    fn move_down(&mut self) {
        if self.sel + 1 < self.selectable.len() { self.sel += 1; }
    }
}

// ── Rendering ───────────────────────────────────────────────────────────────────

fn render(f: &mut Frame, state: &TuiState, config: &ConfigManager) {
    let area = f.area();

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(1),     // main (list + description)
            Constraint::Length(1),  // status bar
        ])
        .split(area);

    // ── Header ──
    let title_text = if state.dirty { " vg config  [unsaved changes] " } else { " vg config " };
    let title_style = if state.dirty {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
    };
    let header = Paragraph::new(title_text)
        .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Blue)))
        .style(title_style);
    f.render_widget(header, outer[0]);

    // ── Main: left list | right description ──
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(outer[1]);

    render_list(f, state, config, main[0]);
    render_description(f, state, config, main[1]);

    // ── Status bar ──
    let msg = state.message.as_deref().unwrap_or(
        match &state.mode {
            Mode::Browse   => "↑↓ navigate  Enter/Space edit  s save  q quit",
            Mode::Editing { .. } => "Type value  Enter confirm  Esc cancel",
        }
    );
    let bar_color = if state.message.is_some() { Color::Green } else { Color::DarkGray };
    let bar = Paragraph::new(format!("  {}", msg)).style(Style::default().fg(bar_color));
    f.render_widget(bar, outer[2]);

    // ── Edit popup (rendered on top) ──
    if let Mode::Editing { key, input, cursor } = &state.mode {
        render_edit_popup(f, area, key, input, *cursor);
    }
}

fn render_list(f: &mut Frame, state: &TuiState, config: &ConfigManager, area: Rect) {
    let block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let items: Vec<ListItem> = state.rows.iter().enumerate().map(|(row_idx, row)| {
        match row {
            Row::Section(s) => {
                let line = Line::from(vec![
                    Span::styled(
                        format!("  {}  ", s.title),
                        Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                    ),
                ]);
                ListItem::new(line)
            }
            Row::Field(f_def) => {
                let is_selected = state.selectable[state.sel] == row_idx;
                let value = get_value(f_def.key, config);

                let (label_style, value_style) = if is_selected {
                    (
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        Style::default().fg(Color::Yellow),
                    )
                } else {
                    (
                        Style::default().fg(Color::White),
                        match f_def.kind {
                            FieldKind::Bool if value == "true" => Style::default().fg(Color::Green),
                            FieldKind::Bool => Style::default().fg(Color::Red),
                            FieldKind::ReadOnly => Style::default().fg(Color::DarkGray),
                            _ => Style::default().fg(Color::Cyan),
                        },
                    )
                };

                let cursor = if is_selected { "▶ " } else { "  " };
                let cursor_style = if is_selected {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                // Truncate value to fit
                let max_value_len = area.width.saturating_sub(30) as usize;
                let display_value = if value.len() > max_value_len && max_value_len > 3 {
                    format!("{}…", &value[..max_value_len - 1])
                } else {
                    value
                };

                let line = Line::from(vec![
                    Span::styled(cursor, cursor_style),
                    Span::styled(format!("{:<28}", f_def.label), label_style),
                    Span::styled(display_value, value_style),
                ]);
                ListItem::new(line)
            }
        }
    }).collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected_row_idx()));

    let list = List::new(items).block(block);
    f.render_stateful_widget(list, area, &mut list_state);
}

fn render_description(f: &mut Frame, state: &TuiState, config: &ConfigManager, area: Rect) {
    let block = Block::default()
        .title(" Description ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    if let Some(field) = state.selected_field() {
        let value = get_value(field.key, config);
        let is_readonly = field.kind == FieldKind::ReadOnly;
        let action_hint = match field.kind {
            FieldKind::Bool     => "  Press Enter or Space to toggle.",
            FieldKind::Text     => "  Press Enter to edit.",
            FieldKind::ReadOnly => "  Read-only field.",
        };

        let lines: Vec<Line> = vec![
            Line::from(Span::styled(
                format!("  {}", field.key),
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                format!("  Current value:"),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                format!("  {}", value),
                if is_readonly {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                },
            )),
            Line::raw(""),
            Line::from(Span::styled(
                format!("  {}", field.description),
                Style::default().fg(Color::Gray),
            )),
            Line::raw(""),
            Line::from(Span::styled(
                action_hint,
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            )),
        ];

        let desc = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
        f.render_widget(desc, area);
    } else {
        f.render_widget(Paragraph::new("").block(block), area);
    }
}

fn render_edit_popup(f: &mut Frame, area: Rect, key: &str, input: &str, cursor: usize) {
    let popup_width = (area.width * 2 / 3).max(50).min(area.width.saturating_sub(4));
    let popup_height = 7u16;
    let x = area.x + area.width.saturating_sub(popup_width) / 2;
    let y = area.y + area.height.saturating_sub(popup_height) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(format!(" Edit: {} ", key))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1), Constraint::Length(1)])
        .margin(1)
        .split(inner);

    // Input line with cursor
    let before = &input[..input.char_indices().nth(cursor).map(|(i, _)| i).unwrap_or(input.len())];
    let after  = if cursor < input.chars().count() {
        let byte_start = input.char_indices().nth(cursor).map(|(i, _)| i).unwrap_or(input.len());
        &input[byte_start..]
    } else {
        ""
    };

    let input_line = Line::from(vec![
        Span::styled(before, Style::default().fg(Color::White)),
        Span::styled("█", Style::default().fg(Color::Yellow)),
        Span::styled(after, Style::default().fg(Color::White)),
    ]);
    f.render_widget(Paragraph::new(input_line), layout[0]);

    // Hint
    let hint = Paragraph::new(Line::from(Span::styled(
        "Enter: confirm   Esc: cancel",
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(hint, layout[2]);
}

// ── RAII terminal guard ─────────────────────────────────────────────────────────

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

// ── Public entry point ─────────────────────────────────────────────────────────

pub fn run(config: &mut ConfigManager) -> Result<()> {
    if !io::stdout().is_terminal() {
        // Non-interactive: fall back to plain list
        println!("Config file: {}", config.config_path().display());
        return Ok(());
    }

    let _guard = TermGuard::new()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut state = TuiState::new();

    loop {
        terminal.draw(|f| render(f, &state, config))?;

        if state.quit { break; }

        if !event::poll(std::time::Duration::from_millis(200))? {
            if state.message.is_some() {
                state.message = None;
            }
            continue;
        }

        if let Event::Key(key) = event::read()? {
            state.message = None;
            match &state.mode {
                Mode::Browse => handle_browse(key, &mut state, config)?,
                Mode::Editing { .. } => handle_edit(key, &mut state, config),
            }
        }

        if state.quit { break; }
    }

    Ok(())
}

fn handle_browse(key: event::KeyEvent, state: &mut TuiState, config: &mut ConfigManager) -> Result<()> {
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            if state.dirty {
                config.save()?;
            }
            state.quit = true;
        }
        (KeyCode::Char('s'), _) => {
            config.save()?;
            state.dirty = false;
            state.message = Some("  ✓ Settings saved.".to_string());
        }
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
            if !state.selectable.is_empty() { state.move_up(); }
        }
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
            if !state.selectable.is_empty() { state.move_down(); }
        }
        (KeyCode::Enter, _) | (KeyCode::Char(' '), _) => {
            if state.selectable.is_empty() { return Ok(()); }
            let row_idx = state.selected_row_idx();
            if let Row::Field(f) = &state.rows[row_idx] {
                match f.kind {
                    FieldKind::ReadOnly => {}
                    FieldKind::Bool => {
                        let key = f.key;
                        toggle_bool(key, config);
                        state.dirty = true;
                    }
                    FieldKind::Text => {
                        let key = f.key;
                        let current = get_value(key, config);
                        let cursor = current.chars().count();
                        state.mode = Mode::Editing {
                            key,
                            input: current,
                            cursor,
                        };
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_edit(key: event::KeyEvent, state: &mut TuiState, config: &mut ConfigManager) {
    let Mode::Editing { key: field_key, input, cursor } = &mut state.mode else { return };

    match key.code {
        KeyCode::Esc => {
            state.mode = Mode::Browse;
        }
        KeyCode::Enter => {
            let k = *field_key;
            let v = input.clone();
            apply_text(k, &v, config);
            state.dirty = true;
            state.mode = Mode::Browse;
        }
        KeyCode::Backspace => {
            if *cursor > 0 {
                let byte_pos = input.char_indices()
                    .nth(*cursor - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                input.remove(byte_pos);
                *cursor -= 1;
            }
        }
        KeyCode::Delete => {
            if *cursor < input.chars().count() {
                let byte_pos = input.char_indices()
                    .nth(*cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(input.len());
                input.remove(byte_pos);
            }
        }
        KeyCode::Left => {
            if *cursor > 0 { *cursor -= 1; }
        }
        KeyCode::Right => {
            if *cursor < input.chars().count() { *cursor += 1; }
        }
        KeyCode::Home => { *cursor = 0; }
        KeyCode::End  => { *cursor = input.chars().count(); }
        KeyCode::Char(c) if key.modifiers == KeyModifiers::NONE || key.modifiers == KeyModifiers::SHIFT => {
            let byte_pos = input.char_indices()
                .nth(*cursor)
                .map(|(i, _)| i)
                .unwrap_or(input.len());
            input.insert(byte_pos, c);
            *cursor += 1;
        }
        _ => {}
    }
}
