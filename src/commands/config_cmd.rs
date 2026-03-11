// src/commands/config_cmd.rs
use crate::ui;
use crate::config::ConfigManager;
use anyhow::Result;
use inquire::{Select, Text, Confirm};
use colored::Colorize;

pub fn run(action: Option<String>, key: Option<String>, value: Option<String>, config: &mut ConfigManager) -> Result<()> {
    match action.as_deref() {
        Some("list") | None => list(config),
        Some("get") => {
            if let Some(k) = key {
                get_key(&k, config);
            } else {
                ui::fail("Usage: vg config get <key>");
            }
        }
        Some("set") => {
            if let (Some(k), Some(v)) = (key, value) {
                set_key(&k, &v, config)?;
            } else {
                ui::fail("Usage: vg config set <key> <value>");
            }
        }
        Some("edit") => interactive_edit(config)?,
        Some(unknown) => ui::fail(&format!("Unknown config action: {}", unknown)),
    }
    Ok(())
}

fn list(config: &ConfigManager) {
    ui::print_header("SETTINGS");

    ui::section("Search");
    ui::info_line("search.max_results", &config.config.search.max_results.to_string());
    ui::info_line("search.max_depth", &config.config.search.max_depth.to_string());
    ui::info_line("search.exclude_hidden", &config.config.search.exclude_hidden.to_string());
    ui::info_line("search.fuzzy_threshold", &config.config.search.fuzzy_threshold.to_string());

    ui::section("System");
    ui::info_line("system.auto_confirm_update", &config.config.system.auto_confirm_update.to_string());

    ui::section("Analytics");
    ui::info_line("analytics.enabled", &config.config.analytics.enabled.to_string());
    ui::info_line("analytics.track_commands", &config.config.analytics.track_commands.to_string());
    ui::info_line("analytics.client_id", &format!("{}...", &config.config.analytics.client_id.chars().take(8).collect::<String>()));

    println!();
    println!("  {} {}", "Config file:".truecolor(71, 85, 105), config.config_path().display());
}

fn get_key(key: &str, config: &ConfigManager) {
    let value = match key {
        "search.max_results" => config.config.search.max_results.to_string(),
        "search.max_depth" => config.config.search.max_depth.to_string(),
        "search.exclude_hidden" => config.config.search.exclude_hidden.to_string(),
        "search.fuzzy_threshold" => config.config.search.fuzzy_threshold.to_string(),
        "system.auto_confirm_update" => config.config.system.auto_confirm_update.to_string(),
        "analytics.enabled" => config.config.analytics.enabled.to_string(),
        "analytics.track_commands" => config.config.analytics.track_commands.to_string(),
        "analytics.client_id" => config.config.analytics.client_id.clone(),
        _ => {
            ui::fail(&format!("Unknown config key: {}", key));
            return;
        }
    };
    println!("{} = {}", key.truecolor(96, 165, 250), value.truecolor(224, 242, 254));
}

fn set_key(key: &str, value: &str, config: &mut ConfigManager) -> Result<()> {
    match key {
        "search.max_results" => config.config.search.max_results = value.parse()?,
        "search.max_depth" => config.config.search.max_depth = value.parse()?,
        "search.exclude_hidden" => config.config.search.exclude_hidden = value.parse()?,
        "search.fuzzy_threshold" => config.config.search.fuzzy_threshold = value.parse()?,
        "system.auto_confirm_update" => config.config.system.auto_confirm_update = value.parse()?,
        "analytics.enabled" => config.config.analytics.enabled = value.parse()?,
        "analytics.track_commands" => config.config.analytics.track_commands = value.parse()?,
        _ => {
            ui::fail(&format!("Unknown or read-only config key: {}", key));
            return Ok(());
        }
    }
    config.save()?;
    ui::success(&format!("Set {} = {}", key, value));
    Ok(())
}

fn interactive_edit(config: &mut ConfigManager) -> Result<()> {
    ui::print_header("EDIT SETTINGS");

    let options = vec![
        "search.max_results",
        "search.max_depth",
        "search.exclude_hidden",
        "search.fuzzy_threshold",
        "system.auto_confirm_update",
        "analytics.enabled",
        "analytics.track_commands",
        "[ Save & Exit ]",
    ];

    loop {
        let choice = Select::new("Select setting to edit:", options.clone()).prompt()?;
        if choice == "[ Save & Exit ]" {
            config.save()?;
            ui::success("Settings saved.");
            break;
        }

        match choice {
            "search.max_results" => {
                let val = Text::new("max_results:").with_default(&config.config.search.max_results.to_string()).prompt()?;
                if let Ok(n) = val.parse() { config.config.search.max_results = n; }
            }
            "search.max_depth" => {
                let val = Text::new("max_depth:").with_default(&config.config.search.max_depth.to_string()).prompt()?;
                if let Ok(n) = val.parse() { config.config.search.max_depth = n; }
            }
            "search.exclude_hidden" => {
                let val = Confirm::new("exclude_hidden?").with_default(config.config.search.exclude_hidden).prompt()?;
                config.config.search.exclude_hidden = val;
            }
            "search.fuzzy_threshold" => {
                let val = Text::new("fuzzy_threshold:").with_default(&config.config.search.fuzzy_threshold.to_string()).prompt()?;
                if let Ok(n) = val.parse() { config.config.search.fuzzy_threshold = n; }
            }
            "system.auto_confirm_update" => {
                let val = Confirm::new("auto_confirm_update?").with_default(config.config.system.auto_confirm_update).prompt()?;
                config.config.system.auto_confirm_update = val;
            }
            "analytics.enabled" => {
                let val = Confirm::new("Enable analytics ping?").with_default(config.config.analytics.enabled).prompt()?;
                config.config.analytics.enabled = val;
            }
            "analytics.track_commands" => {
                let val = Confirm::new("Track command usage?").with_default(config.config.analytics.track_commands).prompt()?;
                config.config.analytics.track_commands = val;
            }
            _ => {}
        }
    }

    Ok(())
}
