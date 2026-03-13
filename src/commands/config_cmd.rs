// src/commands/config_cmd.rs
use crate::ui;
use crate::config::ConfigManager;
use anyhow::Result;
use inquire::{Select, Text, Confirm};
use colored::Colorize;

pub fn run(action: Option<String>, key: Option<String>, value: Option<String>, config: &mut ConfigManager) -> Result<()> {
    match action.as_deref() {
        // No action or "edit" → launch TUI; "list" → plain text output for scripting
        None | Some("edit") => super::config_tui::run(config)?,
        Some("list") => list(config),
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

    ui::section("Search — Index");
    ui::info_line("search.default_paths",       &config.config.search.default_paths.join(", "));
    ui::info_line("search.full_system_index",   &config.config.search.full_system_index.to_string());
    ui::info_line("search.system_index_roots",  &config.config.search.system_index_roots.join(", "));
    ui::info_line("search.system_exclude_paths",&config.config.search.system_exclude_paths.join(", "));
    ui::info_line("search.max_depth",           &config.config.search.max_depth.to_string());
    ui::info_line("search.exclude_hidden",      &config.config.search.exclude_hidden.to_string());

    ui::section("Search — Results");
    ui::info_line("search.max_results",         &config.config.search.max_results.to_string());
    ui::info_line("search.fuzzy_threshold",     &config.config.search.fuzzy_threshold.to_string());

    ui::section("System");
    ui::info_line("system.auto_confirm_update", &config.config.system.auto_confirm_update.to_string());

    ui::section("Analytics");
    ui::info_line("analytics.enabled",          &config.config.analytics.enabled.to_string());
    ui::info_line("analytics.track_commands",   &config.config.analytics.track_commands.to_string());
    ui::info_line("analytics.client_id",        &format!("{}...", &config.config.analytics.client_id.chars().take(8).collect::<String>()));

    println!();
    println!("  {} {}", "Config file:".truecolor(71, 85, 105), config.config_path().display());
    println!("  {} {}", "Tip:".truecolor(71, 85, 105), "vg config set search.full_system_index true  →  index entire filesystem".truecolor(100, 116, 139));
}

fn get_key(key: &str, config: &ConfigManager) {
    let value: Option<String> = match key {
        "search.default_paths"        => Some(config.config.search.default_paths.join(", ")),
        "search.full_system_index"    => Some(config.config.search.full_system_index.to_string()),
        "search.system_index_roots"   => Some(config.config.search.system_index_roots.join(", ")),
        "search.system_exclude_paths" => Some(config.config.search.system_exclude_paths.join(", ")),
        "search.max_results"          => Some(config.config.search.max_results.to_string()),
        "search.max_depth"            => Some(config.config.search.max_depth.to_string()),
        "search.exclude_hidden"       => Some(config.config.search.exclude_hidden.to_string()),
        "search.fuzzy_threshold"      => Some(config.config.search.fuzzy_threshold.to_string()),
        "system.auto_confirm_update"  => Some(config.config.system.auto_confirm_update.to_string()),
        "analytics.enabled"           => Some(config.config.analytics.enabled.to_string()),
        "analytics.track_commands"    => Some(config.config.analytics.track_commands.to_string()),
        "analytics.client_id"         => Some(config.config.analytics.client_id.clone()),
        _ => None,
    };
    match value {
        Some(v) => println!("{} = {}", key.truecolor(96, 165, 250), v.truecolor(224, 242, 254)),
        None => ui::fail(&format!("Unknown config key: {}", key)),
    }
}

fn set_key(key: &str, value: &str, config: &mut ConfigManager) -> Result<()> {
    match key {
        "search.full_system_index"    => config.config.search.full_system_index    = value.parse()?,
        "search.max_results"          => config.config.search.max_results          = value.parse()?,
        "search.max_depth"            => config.config.search.max_depth            = value.parse()?,
        "search.exclude_hidden"       => config.config.search.exclude_hidden       = value.parse()?,
        "search.fuzzy_threshold"      => config.config.search.fuzzy_threshold      = value.parse()?,
        "system.auto_confirm_update"  => config.config.system.auto_confirm_update  = value.parse()?,
        "analytics.enabled"           => config.config.analytics.enabled           = value.parse()?,
        "analytics.track_commands"    => config.config.analytics.track_commands    = value.parse()?,
        // Vec fields: comma-separated
        "search.default_paths" => {
            config.config.search.default_paths = value.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }
        "search.system_index_roots" => {
            config.config.search.system_index_roots = value.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }
        "search.system_exclude_paths" => {
            config.config.search.system_exclude_paths = value.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        }
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
        "search.full_system_index",
        "search.default_paths",
        "search.system_index_roots",
        "search.system_exclude_paths",
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
            "search.full_system_index" => {
                let val = Confirm::new("Enable full system index? (indexes entire filesystem)")
                    .with_default(config.config.search.full_system_index)
                    .prompt()?;
                config.config.search.full_system_index = val;
            }
            "search.default_paths" => {
                let current = config.config.search.default_paths.join(", ");
                let val = Text::new("default_paths (comma-separated):").with_default(&current).prompt()?;
                config.config.search.default_paths = val.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
            "search.system_index_roots" => {
                let current = config.config.search.system_index_roots.join(", ");
                let val = Text::new("system_index_roots (comma-separated):").with_default(&current).prompt()?;
                config.config.search.system_index_roots = val.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
            "search.system_exclude_paths" => {
                let current = config.config.search.system_exclude_paths.join(", ");
                let val = Text::new("system_exclude_paths (comma-separated):").with_default(&current).prompt()?;
                config.config.search.system_exclude_paths = val.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
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
