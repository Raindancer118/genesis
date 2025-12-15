use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;
use inquire::{Text, Select, Confirm};
use chrono::{DateTime, Utc};
use comfy_table::{Table, presets::UTF8_FULL};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Note {
    id: usize,
    title: String,
    content: String,
    created: DateTime<Utc>,
    modified: DateTime<Utc>,
    tags: Vec<String>,
}

pub fn run(action: Option<String>) -> Result<()> {
    println!("{}", "📝 Notes".bold().yellow());
    
    let notes_path = get_notes_path()?;
    
    let action = match action {
        Some(a) => a,
        None => {
            let options = vec!["New Note", "List Notes", "View Note", "Edit Note", "Delete Note", "Search"];
            Select::new("Select action:", options).prompt()?.to_string()
        }
    };
    
    match action.as_str() {
        "New Note" | "new" | "add" => create_note(&notes_path)?,
        "List Notes" | "list" | "ls" => list_notes(&notes_path)?,
        "View Note" | "view" | "show" => view_note(&notes_path)?,
        "Edit Note" | "edit" => edit_note(&notes_path)?,
        "Delete Note" | "delete" | "rm" => delete_note(&notes_path)?,
        "Search" | "search" | "find" => search_notes(&notes_path)?,
        _ => println!("{}", "Unknown action".red()),
    }
    
    Ok(())
}

fn get_notes_path() -> Result<PathBuf> {
    let dir = if let Some(proj_dirs) = ProjectDirs::from("", "", "genesis") {
        proj_dirs.data_dir().to_path_buf()
    } else {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".local/share/genesis")
    };
    
    fs::create_dir_all(&dir)?;
    Ok(dir.join("notes.json"))
}

fn load_notes(path: &PathBuf) -> Result<Vec<Note>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(path)?;
    let notes: Vec<Note> = serde_json::from_str(&content)?;
    Ok(notes)
}

fn save_notes(path: &PathBuf, notes: &Vec<Note>) -> Result<()> {
    let content = serde_json::to_string_pretty(notes)?;
    fs::write(path, content)?;
    Ok(())
}

fn create_note(notes_path: &PathBuf) -> Result<()> {
    let title = Text::new("Note title:").prompt()?;
    let content = Text::new("Note content (or press Enter to use editor):")
        .prompt()?;
    
    let content = if content.is_empty() {
        // Use default editor
        inquire::Editor::new("Write your note:")
            .with_predefined_text("")
            .prompt()?
    } else {
        content
    };
    
    let tags_input = Text::new("Tags (comma-separated, optional):")
        .with_default("")
        .prompt()?;
    
    let tags: Vec<String> = tags_input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    let mut notes = load_notes(notes_path)?;
    let id = notes.iter().map(|n| n.id).max().unwrap_or(0) + 1;
    
    let note = Note {
        id,
        title,
        content,
        created: Utc::now(),
        modified: Utc::now(),
        tags,
    };
    
    notes.push(note);
    save_notes(notes_path, &notes)?;
    
    println!("{}", "✅ Note created successfully!".green());
    
    Ok(())
}

fn list_notes(notes_path: &PathBuf) -> Result<()> {
    let notes = load_notes(notes_path)?;
    
    if notes.is_empty() {
        println!("{}", "No notes found. Create one with 'notes new'".yellow());
        return Ok(());
    }
    
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["ID", "Title", "Tags", "Created", "Modified"]);
    
    for note in notes {
        let tags = if note.tags.is_empty() {
            "-".to_string()
        } else {
            note.tags.join(", ")
        };
        
        table.add_row(vec![
            note.id.to_string(),
            note.title,
            tags,
            note.created.format("%Y-%m-%d %H:%M").to_string(),
            note.modified.format("%Y-%m-%d %H:%M").to_string(),
        ]);
    }
    
    println!("{}", table);
    
    Ok(())
}

fn view_note(notes_path: &PathBuf) -> Result<()> {
    let notes = load_notes(notes_path)?;
    
    if notes.is_empty() {
        println!("{}", "No notes found.".yellow());
        return Ok(());
    }
    
    let note_titles: Vec<String> = notes.iter()
        .map(|n| format!("{}: {}", n.id, n.title))
        .collect();
    
    let selection = Select::new("Select note to view:", note_titles).prompt()?;
    let id: usize = selection.split(':').next().unwrap().trim().parse()?;
    
    if let Some(note) = notes.iter().find(|n| n.id == id) {
        println!("\n{}", "═".repeat(60).cyan());
        println!("{}: {}", "Title".bold(), note.title.cyan().bold());
        println!("{}: {}", "ID".bold(), note.id);
        println!("{}: {}", "Created".bold(), note.created.format("%Y-%m-%d %H:%M:%S"));
        println!("{}: {}", "Modified".bold(), note.modified.format("%Y-%m-%d %H:%M:%S"));
        if !note.tags.is_empty() {
            println!("{}: {}", "Tags".bold(), note.tags.join(", ").yellow());
        }
        println!("{}", "═".repeat(60).cyan());
        println!("\n{}\n", note.content);
        println!("{}", "═".repeat(60).cyan());
    }
    
    Ok(())
}

fn edit_note(notes_path: &PathBuf) -> Result<()> {
    let mut notes = load_notes(notes_path)?;
    
    if notes.is_empty() {
        println!("{}", "No notes found.".yellow());
        return Ok(());
    }
    
    let note_titles: Vec<String> = notes.iter()
        .map(|n| format!("{}: {}", n.id, n.title))
        .collect();
    
    let selection = Select::new("Select note to edit:", note_titles).prompt()?;
    let id: usize = selection.split(':').next().unwrap().trim().parse()?;
    
    if let Some(note) = notes.iter_mut().find(|n| n.id == id) {
        let new_title = Text::new("Title:")
            .with_default(&note.title)
            .prompt()?;
        
        let new_content = inquire::Editor::new("Content:")
            .with_predefined_text(&note.content)
            .prompt()?;
        
        note.title = new_title;
        note.content = new_content;
        note.modified = Utc::now();
        
        save_notes(notes_path, &notes)?;
        println!("{}", "✅ Note updated successfully!".green());
    }
    
    Ok(())
}

fn delete_note(notes_path: &PathBuf) -> Result<()> {
    let mut notes = load_notes(notes_path)?;
    
    if notes.is_empty() {
        println!("{}", "No notes found.".yellow());
        return Ok(());
    }
    
    let note_titles: Vec<String> = notes.iter()
        .map(|n| format!("{}: {}", n.id, n.title))
        .collect();
    
    let selection = Select::new("Select note to delete:", note_titles).prompt()?;
    let id: usize = selection.split(':').next().unwrap().trim().parse()?;
    
    if Confirm::new(&format!("Delete note #{}?", id))
        .with_default(false)
        .prompt()?
    {
        notes.retain(|n| n.id != id);
        save_notes(notes_path, &notes)?;
        println!("{}", "✅ Note deleted successfully!".green());
    }
    
    Ok(())
}

fn search_notes(notes_path: &PathBuf) -> Result<()> {
    let notes = load_notes(notes_path)?;
    
    if notes.is_empty() {
        println!("{}", "No notes found.".yellow());
        return Ok(());
    }
    
    let query = Text::new("Search query:").prompt()?;
    let query = query.to_lowercase();
    
    let results: Vec<&Note> = notes.iter()
        .filter(|n| {
            n.title.to_lowercase().contains(&query) ||
            n.content.to_lowercase().contains(&query) ||
            n.tags.iter().any(|t| t.to_lowercase().contains(&query))
        })
        .collect();
    
    if results.is_empty() {
        println!("{}", "No matching notes found.".yellow());
        return Ok(());
    }
    
    println!("\n{} matching note(s):", results.len());
    
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["ID", "Title", "Tags", "Created"]);
    
    for note in results {
        let tags = if note.tags.is_empty() {
            "-".to_string()
        } else {
            note.tags.join(", ")
        };
        
        table.add_row(vec![
            note.id.to_string(),
            note.title.clone(),
            tags,
            note.created.format("%Y-%m-%d %H:%M").to_string(),
        ]);
    }
    
    println!("{}", table);
    
    Ok(())
}
