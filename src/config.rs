use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use directories::ProjectDirs;
use anyhow::{Result, Context};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub system: SystemConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SearchConfig {
    pub default_paths: Vec<String>,
    pub ignore_patterns: Vec<String>,
    pub max_depth: usize,
    pub max_results: usize,
    pub show_details: bool,
    pub verbose: bool,
    pub exclude_hidden: bool,
    pub lightspeed_mode: bool,
    pub fuzzy_threshold: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_paths: vec![dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .to_string_lossy().to_string()],
            ignore_patterns: vec![
                "node_modules".into(), ".git".into(), "target".into(),
                ".cache".into(), "__pycache__".into(), ".npm".into(),
                ".cargo".into(), "venv".into(), ".venv".into(),
            ],
            max_depth: 10,
            max_results: 50,
            show_details: false,
            verbose: false,
            exclude_hidden: true,
            lightspeed_mode: true,
            fuzzy_threshold: 2,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SystemConfig {
    pub package_manager_priority: Vec<String>,
    pub auto_confirm_update: bool,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            package_manager_priority: vec!["pamac".into(), "yay".into(), "paru".into(), "pacman".into()],
            auto_confirm_update: false,
        }
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
    pub config: Config,
}

impl ConfigManager {
    pub fn new() -> Self {
        let (config_path, config) = Self::load_or_default();
        Self { config_path, config }
    }

    fn load_or_default() -> (PathBuf, Config) {
        let config_dir = if let Some(proj_dirs) = ProjectDirs::from("", "volantic", "genesis") {
            proj_dirs.config_dir().to_path_buf()
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".config").join("volantic-genesis")
        };
        let config_path = config_dir.join("config.toml");
        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str(&content) {
                    return (config_path, config);
                }
            }
        }
        (config_path, Config::default())
    }

    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).context("Failed to create config directory")?;
        }
        let content = toml::to_string_pretty(&self.config).context("Failed to serialize config")?;
        fs::write(&self.config_path, content).context("Failed to write config file")?;
        Ok(())
    }
}
