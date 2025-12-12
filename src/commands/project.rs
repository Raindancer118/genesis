use crate::config::ConfigManager;
use anyhow::{Result, Context};
use colored::Colorize;
use inquire::{Text, Confirm, Select};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run_new(
    name: Option<String>,
    template: Option<String>,
    git: bool,
    yes: bool,
    structure: Option<String>,
    config_manager: &ConfigManager,
) -> Result<()> {
    
    // 1. Name
    let project_name = match name {
        Some(n) => n,
        None => Text::new("Project Name:").prompt()?,
    };

    let target_dir = Path::new(".").join(&project_name);
    if target_dir.exists() {
         return Err(anyhow::anyhow!("Directory '{}' already exists.", project_name));
    }

    // 2. Template / Structure
    if let Some(s) = structure {
         // JSON Structure handling
         println!("Creating project from JSON structure...");
         create_from_structure(&target_dir, &s)?;
    } else {
        // Template
        let tmpl_key = match template {
            Some(t) => t,
            None => {
                let options = vec!["python", "rust", "web", "empty"];
                Select::new("Select Template:", options).prompt()?.to_string()
            }
        };
        
        create_from_template(&target_dir, &tmpl_key, config_manager)?;
    }

    // 3. Git Init
    // If flag is true, do it. If not, check config default or ask (unless yes=true).
    let should_git = if git {
        true
    } else if yes {
        config_manager.config.project.use_git_init
    } else {
        // Check config default
        let def = config_manager.config.project.use_git_init;
        Confirm::new("Initialize Git repository?").with_default(def).prompt()?
    };

    if should_git {
        println!("Initializing Git...");
        Command::new("git").arg("init").arg(&target_dir).output()?;
    }

    println!("{} Project '{}' created successfully.", "✅".green(), project_name);

    Ok(())
}

fn create_from_template(target: &Path, template: &str, config: &ConfigManager) -> Result<()> {
    fs::create_dir_all(target)?;

    let author = &config.config.project.default_author;
    let email = &config.config.project.default_email;

    match template {
        "python" => {
            // Basic Python structure
            fs::create_dir(target.join("src"))?;
            fs::create_dir(target.join("tests"))?;
            fs::write(target.join("src/main.py"), format!("# Author: {}\n\ndef main():\n    print('Hello World')\n\nif __name__ == '__main__':\n    main()\n", author))?;
            fs::write(target.join("requirements.txt"), "")?;
            fs::write(target.join("README.md"), format!("# Python Project\n\nAuthor: {} <{}>", author, email))?;
        },
        "rust" => {
            // Cargo new
            // Since we created dir, maybe run cargo init inside?
            Command::new("cargo").arg("init").arg(target).output()?;
            // Update Cargo.toml author if possible? Cargo usually handles it from git config.
        },
        "web" => {
             fs::create_dir(target.join("public"))?;
             fs::create_dir(target.join("src"))?;
             fs::write(target.join("index.html"), "<!DOCTYPE html><html><body><h1>Hello</h1></body></html>")?;
             fs::write(target.join("src/style.css"), "body { background: #f0f0f0; }")?;
             fs::write(target.join("src/app.js"), "console.log('Hello');")?;
        },
        "empty" | _ => {
            fs::write(target.join("README.md"), "# New Project")?;
        }
    }
    Ok(())
}

fn create_from_structure(target: &Path, json_str: &str) -> Result<()> {
    // Parse JSON and create struct
    let v: serde_json::Value = serde_json::from_str(json_str).context("Invalid JSON structure")?;
    
    // Recursive creation
    create_recursive(target, &v)?;
    Ok(())
}

fn create_recursive(base: &Path, value: &serde_json::Value) -> Result<()> {
    if let Some(map) = value.as_object() {
        // It's a directory (or root) containing items
        if !base.exists() {
            fs::create_dir_all(base)?;
        }
        for (name, content) in map {
            let path = base.join(name);
            create_recursive(&path, content)?;
        }
    } else if let Some(s) = value.as_str() {
        // It's a file with content
        // Ensure parent exists
        if let Some(p) = base.parent() {
            fs::create_dir_all(p)?;
        }
        fs::write(base, s)?;
    } else if value.is_null() {
        // Empty file
        if let Some(p) = base.parent() {
            fs::create_dir_all(p)?;
        }
        fs::File::create(base)?;
    }
    Ok(())
}

pub fn run_build(name: String, template_str: Option<String>) -> Result<()> {
    // If template_str is None, maybe read from stdin or file?
    // Python version takes a string. CLI usually passes it?
    // Or maybe it's interactive?
    // Let's assume passed as arg or read from file if arg is path?
    // For now, let's say it's passed or we prompt (if huge).
    
    let content = match template_str {
        Some(s) => s,
        None => {
            // Read from stdin or prompt? 
            // Inquire text is single line. We need mulitline.
            // Using editor?
            inquire::Editor::new("Enter project structure (indented):")
                .with_file_extension(".txt")
                .prompt()?
        }
    };

    println!("Building project '{}'...", name);
    let root = Path::new(&name);
    if root.exists() {
         return Err(anyhow::anyhow!("Directory '{}' already exists.", name));
    }
    fs::create_dir(root)?;

    let mut stack = vec![root.to_path_buf()];

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }

        let indent = line.chars().take_while(|c| *c == ' ').count();
        // Assuming 4 spaces per level or standard indentation
        // We can just track change in indent depth.
        // A simple approach: 1 level = 2 or 4 spaces.
        // Let's assume 4 spaces per level as in Python version (lines 781)
        
        let depth = indent / 4;
        
        // stack[0] is root (depth 0, effectively).
        // Items under root should have depth 0 relative to content?
        // Python: "stack = [project_path]". Depth 0 line means child of root?
        // If line has indent 0, it's inside root.
        // stack has size 1 initially.
        // depth 0 -> stack index 0 is parent.
        
        while stack.len() > depth + 1 {
            stack.pop();
        }

        let parent = stack.last().unwrap();
        let name = trimmed.trim_end_matches('/');
        let is_dir = trimmed.ends_with('/');
        
        let path = parent.join(name);
        
        if is_dir {
            fs::create_dir_all(&path)?;
            stack.push(path);
        } else {
            if let Some(p) = path.parent() {
                 if !p.exists() { fs::create_dir_all(p)?; }
            }
            fs::File::create(&path)?;
        }
    }
    
    println!("{} Structure built.", "✅".green());
    Ok(())
}
