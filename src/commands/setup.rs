use crate::config::ConfigManager;
use anyhow::{Result, Context};
use inquire::{Text, Confirm, MultiSelect, Select};
use colored::Colorize;

pub fn run(config_manager: &mut ConfigManager) -> Result<()> {
    println!("{}", "Interactive Configuration Wizard".bold().green());
    println!("Type 'exit' at any prompt to cancel.\n");

    // Work on a copy/reference of config struct, then save it back via manager
    // Manager owns it, so we modify config_manager.config directly.

    // 1. General Settings
    println!("{}", "--- General Settings ---".bold());
    let lang = Text::new("Language Code:")
        .with_default(&config_manager.config.general.language)
        .prompt()?;
    config_manager.config.general.language = lang;

    // 2. System Settings
    println!("\n{}", "--- System Settings ---".bold());
    
    let priorities_str = config_manager.config.system.package_manager_priority.join(", ");
    let new_priorities = Text::new("Package Manager Priority (comma separated):")
        .with_default(&priorities_str)
        .prompt()?;
    
    config_manager.config.system.package_manager_priority = new_priorities
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    config_manager.config.system.default_install_confirm = Confirm::new("Default Install Confirmation?")
        .with_default(config_manager.config.system.default_install_confirm)
        .prompt()?;

    config_manager.config.system.update_mirrors = Confirm::new("Update Mirrors during System Update?")
        .with_default(config_manager.config.system.update_mirrors)
        .prompt()?;

    config_manager.config.system.create_timeshift = Confirm::new("Create Timeshift Snapshot during System Update?")
        .with_default(config_manager.config.system.create_timeshift)
        .prompt()?;

    // 3. Project Settings
    println!("\n{}", "--- Project Settings ---".bold());
    config_manager.config.project.default_author = Text::new("Default Author:")
        .with_default(&config_manager.config.project.default_author)
        .prompt()?;
    
    config_manager.config.project.default_email = Text::new("Default Email:")
        .with_default(&config_manager.config.project.default_email)
        .prompt()?;

    config_manager.config.project.default_license = Text::new("Default License:")
        .with_default(&config_manager.config.project.default_license)
        .prompt()?;

    config_manager.config.project.use_git_init = Confirm::new("Initialize Git by default?")
        .with_default(config_manager.config.project.use_git_init)
        .prompt()?;

    // Save
    if Confirm::new("Save configuration?").with_default(true).prompt()? {
        config_manager.save()?;
        println!("{}", "Configuration saved successfully.".green());
    } else {
        println!("Configuration discarded.");
    }

    Ok(())
}
