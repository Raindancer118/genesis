use anyhow::Result;
use colored::Colorize;
use std::env;
use comfy_table::{Table, presets::UTF8_FULL};
use inquire::{Text, Select};

pub fn run(action: Option<String>) -> Result<()> {
    println!("{}", "🌍 Environment Variables".bold().green());
    
    let action = match action {
        Some(a) => a,
        None => {
            let options = vec!["List All", "Search", "Get Variable", "Export (Show Command)"];
            Select::new("Select action:", options).prompt()?.to_string()
        }
    };
    
    match action.as_str() {
        "List All" | "list" | "ls" => list_env()?,
        "Search" | "search" | "find" => search_env()?,
        "Get Variable" | "get" | "show" => get_env()?,
        "Export (Show Command)" | "export" => show_export()?,
        _ => println!("{}", "Unknown action".red()),
    }
    
    Ok(())
}

fn list_env() -> Result<()> {
    let mut vars: Vec<(String, String)> = env::vars().collect();
    vars.sort_by(|a, b| a.0.cmp(&b.0));
    
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Variable", "Value"]);
    
    for (key, value) in vars {
        // Truncate long values
        let display_value = if value.len() > 80 {
            format!("{}...", &value[..77])
        } else {
            value
        };
        
        table.add_row(vec![key.cyan().to_string(), display_value]);
    }
    
    println!("{}", table);
    println!("\n{} environment variables", env::vars().count().to_string().yellow().bold());
    
    Ok(())
}

fn search_env() -> Result<()> {
    let query = Text::new("Search query:").prompt()?;
    let query_lower = query.to_lowercase();
    
    let results: Vec<(String, String)> = env::vars()
        .filter(|(k, v)| {
            k.to_lowercase().contains(&query_lower) || 
            v.to_lowercase().contains(&query_lower)
        })
        .collect();
    
    if results.is_empty() {
        println!("{}", "No matching environment variables found.".yellow());
        return Ok(());
    }
    
    println!("\n{} matching variable(s):", results.len().to_string().cyan().bold());
    
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Variable", "Value"]);
    
    for (key, value) in results {
        let display_value = if value.len() > 80 {
            format!("{}...", &value[..77])
        } else {
            value
        };
        
        table.add_row(vec![key.cyan().to_string(), display_value]);
    }
    
    println!("{}", table);
    
    Ok(())
}

fn get_env() -> Result<()> {
    let var_name = Text::new("Variable name:").prompt()?;
    
    match env::var(&var_name) {
        Ok(value) => {
            println!("\n{}: {}", var_name.cyan().bold(), value.green());
            println!("\nTo use in shell:");
            println!("  echo ${}", var_name);
        },
        Err(_) => {
            println!("{}", format!("Environment variable '{}' not found", var_name).red());
        }
    }
    
    Ok(())
}

fn show_export() -> Result<()> {
    println!("\n{}", "Common environment variable commands:".yellow().bold());
    println!("\n{}", "Bash/Zsh:".cyan());
    println!("  export VAR_NAME=\"value\"");
    println!("  export PATH=\"$PATH:/new/path\"");
    
    println!("\n{}", "Fish:".cyan());
    println!("  set -x VAR_NAME \"value\"");
    println!("  set -x PATH $PATH /new/path");
    
    println!("\n{}", "Windows (PowerShell):".cyan());
    println!("  $env:VAR_NAME = \"value\"");
    
    println!("\n{}", "Windows (CMD):".cyan());
    println!("  set VAR_NAME=value");
    
    println!("\n{}", "Note: These commands only affect the current session.".dimmed());
    println!("{}", "For permanent changes, add them to your shell's configuration file.".dimmed());
    
    Ok(())
}
