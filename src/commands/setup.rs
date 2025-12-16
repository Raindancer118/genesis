use crate::config::ConfigManager;
use crate::ai::GeminiClient;
use anyhow::Result;
use inquire::{Text, Confirm, Select};
use colored::Colorize;
use std::env;

pub fn run(config_manager: &mut ConfigManager) -> Result<()> {
    println!("{}", "🛠️  Genesis Configuration".bold().cyan());
    
    // Check if Gemini is available (CLI or API)
    if !GeminiClient::is_available() {
        println!("\n{}", "📝 Gemini Not Configured".yellow().bold());
        println!("{}", "To enable AI-assisted file sorting, you can either:");
        println!("\n{}", "Option 1: Use Gemini CLI (Recommended)".bold());
        println!("  • Install the gemini CLI tool");
        println!("  • No API key required when using CLI");
        println!("\n{}", "Option 2: Use Gemini API".bold());
        println!("\n{}", "How to get a Gemini API key:".bold());
        println!("  1. Visit: https://makersuite.google.com/app/apikey");
        println!("  2. Sign in with your Google account");
        println!("  3. Click 'Create API Key'");
        println!("  4. Copy the generated API key");
        println!("\n{}", "How to set up the API key:".bold());
        println!("  • Add to your shell profile (~/.bashrc, ~/.zshrc, etc.):");
        println!("    export GEMINI_API_KEY='your-api-key-here'");
        println!("  • Or set it temporarily:");
        println!("    export GEMINI_API_KEY='your-api-key-here'");
        println!("\n{}", "After setting the key, restart your terminal or run: source ~/.bashrc".cyan());
        
        let configure_now = Confirm::new("Would you like to set the API key now (for this session only)?")
            .with_default(false)
            .prompt()?;
        
        if configure_now {
            let api_key = Text::new("Enter your Gemini API key:")
                .with_help_message("The key will only be set for this terminal session")
                .prompt()?;
            
            if !api_key.trim().is_empty() {
                env::set_var("GEMINI_API_KEY", api_key.trim());
                println!("{}", "✅ API key set for this session!".green());
                println!("{}", "Note: Add it to your shell profile for permanent use.".yellow());
            }
        }
        println!();
    } else {
        println!("{}", "✅ Gemini is available (CLI or API configured)".green());
        println!();
    }
    
    loop {
        let options = vec![
            "General Settings",
            "System Settings",
            "Project Settings",
            "Search Settings",
            "Save & Exit",
            "Discard & Exit",
        ];

        let choice = Select::new("Main Menu:", options).prompt()?;

        match choice {
            "General Settings" => edit_general(config_manager)?,
            "System Settings" => edit_system(config_manager)?,
            "Project Settings" => edit_project(config_manager)?,
            "Search Settings" => edit_search(config_manager)?,
            "Save & Exit" => {
                config_manager.save()?;
                println!("{}", "Configuration saved.".green());
                break;
            }
            "Discard & Exit" => {
                println!("{}", "Changes discarded.".yellow());
                break;
            }
            _ => break,
        }
        println!(); // Spacer
    }

    Ok(())
}

fn edit_general(cm: &mut ConfigManager) -> Result<()> {
    println!("\n{}", "--- General Settings ---".bold());
    
    let lang = Text::new("Language Code:")
        .with_default(&cm.config.general.language)
        .with_help_message("e.g. en, de, fr")
        .prompt()?;
    
    cm.config.general.language = lang;
    Ok(())
}

fn edit_system(cm: &mut ConfigManager) -> Result<()> {
    println!("\n{}", "--- System Settings ---".bold());

    loop {
        let choices = vec![
            "Edit Package Manager Priority", 
            "Toggle Default Install Confirm",
            "Toggle Update Mirrors",
            "Toggle Create Timeshift",
            "Back"
        ];
        
        // Show current states
        let conf = &mut cm.config.system;
        println!("Current Priority: {}", conf.package_manager_priority.join(", ").cyan());
        println!("Install Confirm: {}", format!("{}", conf.default_install_confirm).cyan());
        println!("Update Mirrors:  {}", format!("{}", conf.update_mirrors).cyan());
        println!("Timeshift:       {}", format!("{}", conf.create_timeshift).cyan());

        let selection = Select::new("Edit System Option:", choices).prompt()?;

        match selection {
            "Edit Package Manager Priority" => {
                let current = conf.package_manager_priority.join(", ");
                let new = Text::new("Priorities (comma separated):")
                    .with_default(&current)
                    .prompt()?;
                conf.package_manager_priority = new.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "Toggle Default Install Confirm" => {
                conf.default_install_confirm = !conf.default_install_confirm;
            }
            "Toggle Update Mirrors" => {
                conf.update_mirrors = !conf.update_mirrors;
            }
            "Toggle Create Timeshift" => {
                conf.create_timeshift = !conf.create_timeshift;
            }
            "Back" => break,
            _ => {}
        }
    }
    Ok(())
}

fn edit_project(cm: &mut ConfigManager) -> Result<()> {
    println!("\n{}", "--- Project Settings ---".bold());
    
    // Simple direct prompts for these
    cm.config.project.default_author = Text::new("Default Author:")
        .with_default(&cm.config.project.default_author)
        .prompt()?;

    cm.config.project.default_email = Text::new("Default Email:")
        .with_default(&cm.config.project.default_email)
        .prompt()?;

    cm.config.project.default_license = Text::new("Default License:")
        .with_default(&cm.config.project.default_license)
        .prompt()?;

    cm.config.project.use_git_init = Confirm::new("Initialize Git by default?")
        .with_default(cm.config.project.use_git_init)
        .prompt()?;

    Ok(())
}

fn edit_search(cm: &mut ConfigManager) -> Result<()> {
    println!("\n{}", "--- Search Settings ---".bold());
    
    loop {
        let choices = vec![
            "Edit Default Paths",
            "Edit Ignore Patterns",
            "Set Max Depth",
            "Set Max Results",
            "Set Fuzzy Threshold",
            "Toggle Show Details",
            "Toggle Verbose Mode",
            "Toggle Exclude Hidden",
            "Toggle Lightspeed Mode",
            "Back"
        ];
        
        // Show current states
        let conf = &mut cm.config.search;
        println!("Default Paths: {}", conf.default_paths.join(", ").cyan());
        println!("Ignore Patterns: {}", conf.ignore_patterns.join(", ").cyan());
        println!("Max Depth: {}", format!("{}", conf.max_depth).cyan());
        println!("Max Results: {}", format!("{}", conf.max_results).cyan());
        println!("Fuzzy Threshold: {}", format!("{}", conf.fuzzy_threshold).cyan());
        println!("Show Details: {}", format!("{}", conf.show_details).cyan());
        println!("Verbose: {}", format!("{}", conf.verbose).cyan());
        println!("Exclude Hidden: {}", format!("{}", conf.exclude_hidden).cyan());
        println!("Lightspeed Mode: {} {}", 
            format!("{}", conf.lightspeed_mode).cyan(),
            if conf.lightspeed_mode { "⚡" } else { "" }
        );

        let selection = Select::new("Edit Search Option:", choices).prompt()?;

        match selection {
            "Edit Default Paths" => {
                let current = conf.default_paths.join(", ");
                let new = Text::new("Default paths (comma separated):")
                    .with_default(&current)
                    .with_help_message("Paths to index when no path is specified")
                    .prompt()?;
                conf.default_paths = new.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "Edit Ignore Patterns" => {
                let current = conf.ignore_patterns.join(", ");
                let new = Text::new("Ignore patterns (comma separated):")
                    .with_default(&current)
                    .with_help_message("Patterns to exclude from indexing")
                    .prompt()?;
                conf.ignore_patterns = new.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "Set Max Depth" => {
                let new = Text::new("Max directory depth:")
                    .with_default(&conf.max_depth.to_string())
                    .with_help_message("Maximum depth to traverse directories")
                    .prompt()?;
                if let Ok(depth) = new.parse::<usize>() {
                    conf.max_depth = depth;
                } else {
                    println!("{}", "Invalid number, keeping current value.".yellow());
                }
            }
            "Set Max Results" => {
                let new = Text::new("Max search results:")
                    .with_default(&conf.max_results.to_string())
                    .with_help_message("Maximum number of results to display")
                    .prompt()?;
                if let Ok(results) = new.parse::<usize>() {
                    conf.max_results = results;
                } else {
                    println!("{}", "Invalid number, keeping current value.".yellow());
                }
            }
            "Set Fuzzy Threshold" => {
                let new = Text::new("Fuzzy search edit distance:")
                    .with_default(&conf.fuzzy_threshold.to_string())
                    .with_help_message("Maximum edit distance for fuzzy matching (0-3)")
                    .prompt()?;
                if let Ok(threshold) = new.parse::<usize>() {
                    if threshold <= 3 {
                        conf.fuzzy_threshold = threshold;
                    } else {
                        println!("{}", "Value must be 0-3, keeping current value.".yellow());
                    }
                } else {
                    println!("{}", "Invalid number, keeping current value.".yellow());
                }
            }
            "Toggle Show Details" => {
                conf.show_details = !conf.show_details;
            }
            "Toggle Verbose Mode" => {
                conf.verbose = !conf.verbose;
            }
            "Toggle Exclude Hidden" => {
                conf.exclude_hidden = !conf.exclude_hidden;
            }
            "Toggle Lightspeed Mode" => {
                conf.lightspeed_mode = !conf.lightspeed_mode;
                if conf.lightspeed_mode {
                    println!("{}", "⚡ Lightspeed mode enabled! Rebuild index to activate.".green());
                } else {
                    println!("{}", "Standard search mode enabled.".yellow());
                }
            }
            "Back" => break,
            _ => {}
        }
    }
    Ok(())
}
