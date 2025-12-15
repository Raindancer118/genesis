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
struct Task {
    id: usize,
    title: String,
    description: String,
    priority: Priority,
    status: Status,
    created: DateTime<Utc>,
    due: Option<DateTime<Utc>>,
    completed: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum Priority {
    Low,
    Medium,
    High,
    Urgent,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Priority::Low => write!(f, "Low"),
            Priority::Medium => write!(f, "Medium"),
            Priority::High => write!(f, "High"),
            Priority::Urgent => write!(f, "Urgent"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
enum Status {
    Todo,
    InProgress,
    Done,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Status::Todo => write!(f, "Todo"),
            Status::InProgress => write!(f, "In Progress"),
            Status::Done => write!(f, "Done"),
        }
    }
}

pub fn run(action: Option<String>) -> Result<()> {
    println!("{}", "✅ Todo Manager".bold().green());
    
    let todos_path = get_todos_path()?;
    
    let action = match action {
        Some(a) => a,
        None => {
            let options = vec![
                "New Task",
                "List Tasks",
                "View Task",
                "Update Status",
                "Complete Task",
                "Delete Task",
            ];
            Select::new("Select action:", options).prompt()?.to_string()
        }
    };
    
    match action.as_str() {
        "New Task" | "new" | "add" => create_task(&todos_path)?,
        "List Tasks" | "list" | "ls" => list_tasks(&todos_path)?,
        "View Task" | "view" | "show" => view_task(&todos_path)?,
        "Update Status" | "update" | "status" => update_status(&todos_path)?,
        "Complete Task" | "complete" | "done" => complete_task(&todos_path)?,
        "Delete Task" | "delete" | "rm" => delete_task(&todos_path)?,
        _ => println!("{}", "Unknown action".red()),
    }
    
    Ok(())
}

fn get_todos_path() -> Result<PathBuf> {
    let dir = if let Some(proj_dirs) = ProjectDirs::from("", "", "genesis") {
        proj_dirs.data_dir().to_path_buf()
    } else {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".local/share/genesis")
    };
    
    fs::create_dir_all(&dir)?;
    Ok(dir.join("todos.json"))
}

fn load_tasks(path: &PathBuf) -> Result<Vec<Task>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    
    let content = fs::read_to_string(path)?;
    let tasks: Vec<Task> = serde_json::from_str(&content)?;
    Ok(tasks)
}

fn save_tasks(path: &PathBuf, tasks: &Vec<Task>) -> Result<()> {
    let content = serde_json::to_string_pretty(tasks)?;
    fs::write(path, content)?;
    Ok(())
}

fn create_task(todos_path: &PathBuf) -> Result<()> {
    let title = Text::new("Task title:").prompt()?;
    let description = Text::new("Description (optional):")
        .with_default("")
        .prompt()?;
    
    let priority_options = vec!["Low", "Medium", "High", "Urgent"];
    let priority_str = Select::new("Priority:", priority_options).prompt()?;
    let priority = match priority_str {
        "Low" => Priority::Low,
        "Medium" => Priority::Medium,
        "High" => Priority::High,
        "Urgent" => Priority::Urgent,
        _ => Priority::Medium,
    };
    
    let mut tasks = load_tasks(todos_path)?;
    let id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    
    let task = Task {
        id,
        title,
        description,
        priority,
        status: Status::Todo,
        created: Utc::now(),
        due: None,
        completed: None,
    };
    
    tasks.push(task);
    save_tasks(todos_path, &tasks)?;
    
    println!("{}", "✅ Task created successfully!".green());
    
    Ok(())
}

fn list_tasks(todos_path: &PathBuf) -> Result<()> {
    let tasks = load_tasks(todos_path)?;
    
    if tasks.is_empty() {
        println!("{}", "No tasks found. Create one with 'todo new'".yellow());
        return Ok(());
    }
    
    // Separate by status
    let todo: Vec<_> = tasks.iter().filter(|t| t.status == Status::Todo).collect();
    let in_progress: Vec<_> = tasks.iter().filter(|t| t.status == Status::InProgress).collect();
    let done: Vec<_> = tasks.iter().filter(|t| t.status == Status::Done).collect();
    
    if !todo.is_empty() {
        println!("\n{}", "📝 TODO".bold().yellow());
        print_task_table(&todo);
    }
    
    if !in_progress.is_empty() {
        println!("\n{}", "🔄 IN PROGRESS".bold().cyan());
        print_task_table(&in_progress);
    }
    
    if !done.is_empty() {
        println!("\n{}", "✅ DONE".bold().green());
        print_task_table(&done);
    }
    
    Ok(())
}

fn print_task_table(tasks: &[&Task]) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["ID", "Title", "Priority", "Created"]);
    
    for task in tasks {
        let priority_str = match task.priority {
            Priority::Urgent => task.priority.to_string().red().to_string(),
            Priority::High => task.priority.to_string().yellow().to_string(),
            Priority::Medium => task.priority.to_string().cyan().to_string(),
            Priority::Low => task.priority.to_string().dimmed().to_string(),
        };
        
        table.add_row(vec![
            task.id.to_string(),
            task.title.clone(),
            priority_str,
            task.created.format("%Y-%m-%d").to_string(),
        ]);
    }
    
    println!("{}", table);
}

fn view_task(todos_path: &PathBuf) -> Result<()> {
    let tasks = load_tasks(todos_path)?;
    
    if tasks.is_empty() {
        println!("{}", "No tasks found.".yellow());
        return Ok(());
    }
    
    let task_titles: Vec<String> = tasks.iter()
        .map(|t| format!("{}: {}", t.id, t.title))
        .collect();
    
    let selection = Select::new("Select task to view:", task_titles).prompt()?;
    let id: usize = selection.split(':').next().unwrap().trim().parse()?;
    
    if let Some(task) = tasks.iter().find(|t| t.id == id) {
        println!("\n{}", "═".repeat(60).cyan());
        println!("{}: {}", "Title".bold(), task.title.cyan().bold());
        println!("{}: {}", "ID".bold(), task.id);
        println!("{}: {}", "Status".bold(), task.status);
        println!("{}: {}", "Priority".bold(), task.priority);
        println!("{}: {}", "Created".bold(), task.created.format("%Y-%m-%d %H:%M:%S"));
        if !task.description.is_empty() {
            println!("{}: {}", "Description".bold(), task.description);
        }
        if let Some(completed) = task.completed {
            println!("{}: {}", "Completed".bold().green(), completed.format("%Y-%m-%d %H:%M:%S"));
        }
        println!("{}", "═".repeat(60).cyan());
    }
    
    Ok(())
}

fn update_status(todos_path: &PathBuf) -> Result<()> {
    let mut tasks = load_tasks(todos_path)?;
    
    if tasks.is_empty() {
        println!("{}", "No tasks found.".yellow());
        return Ok(());
    }
    
    let task_titles: Vec<String> = tasks.iter()
        .map(|t| format!("{}: {} [{}]", t.id, t.title, t.status))
        .collect();
    
    let selection = Select::new("Select task to update:", task_titles).prompt()?;
    let id: usize = selection.split(':').next().unwrap().trim().parse()?;
    
    if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
        let status_options = vec!["Todo", "In Progress", "Done"];
        let new_status_str = Select::new("New status:", status_options).prompt()?;
        
        task.status = match new_status_str {
            "Todo" => Status::Todo,
            "In Progress" => Status::InProgress,
            "Done" => {
                task.completed = Some(Utc::now());
                Status::Done
            },
            _ => Status::Todo,
        };
        
        save_tasks(todos_path, &tasks)?;
        println!("{}", "✅ Task status updated!".green());
    }
    
    Ok(())
}

fn complete_task(todos_path: &PathBuf) -> Result<()> {
    let mut tasks = load_tasks(todos_path)?;
    
    if tasks.is_empty() {
        println!("{}", "No tasks found.".yellow());
        return Ok(());
    }
    
    let incomplete: Vec<String> = tasks.iter()
        .filter(|t| t.status != Status::Done)
        .map(|t| format!("{}: {}", t.id, t.title))
        .collect();
    
    if incomplete.is_empty() {
        println!("{}", "All tasks are already complete!".green());
        return Ok(());
    }
    
    let selection = Select::new("Select task to complete:", incomplete).prompt()?;
    let id: usize = selection.split(':').next().unwrap().trim().parse()?;
    
    if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
        task.status = Status::Done;
        task.completed = Some(Utc::now());
        
        save_tasks(todos_path, &tasks)?;
        println!("{}", "✅ Task completed!".green());
    }
    
    Ok(())
}

fn delete_task(todos_path: &PathBuf) -> Result<()> {
    let mut tasks = load_tasks(todos_path)?;
    
    if tasks.is_empty() {
        println!("{}", "No tasks found.".yellow());
        return Ok(());
    }
    
    let task_titles: Vec<String> = tasks.iter()
        .map(|t| format!("{}: {}", t.id, t.title))
        .collect();
    
    let selection = Select::new("Select task to delete:", task_titles).prompt()?;
    let id: usize = selection.split(':').next().unwrap().trim().parse()?;
    
    if Confirm::new(&format!("Delete task #{}?", id))
        .with_default(false)
        .prompt()?
    {
        tasks.retain(|t| t.id != id);
        save_tasks(todos_path, &tasks)?;
        println!("{}", "✅ Task deleted successfully!".green());
    }
    
    Ok(())
}
