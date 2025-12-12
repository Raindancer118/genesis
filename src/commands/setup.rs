use crate::config::ConfigManager;
use anyhow::Result;
use inquire::{Text, Confirm, Select};
use colored::Colorize;

pub fn run(config_manager: &mut ConfigManager) -> Result<()> {
    println!("{}", "🛠️  Genesis Configuration".bold().cyan());
    
    loop {
        let options = vec![
            "General Settings",
            "System Settings",
            "Project Settings",
            "Save & Exit",
            "Discard & Exit",
        ];

        let choice = Select::new("Main Menu:", options).prompt()?;

        match choice {
            "General Settings" => edit_general(config_manager)?,
            "System Settings" => edit_system(config_manager)?,
            "Project Settings" => edit_project(config_manager)?,
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
