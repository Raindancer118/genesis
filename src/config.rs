use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use directories::ProjectDirs;
use anyhow::{Result, Context};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    pub system: SystemConfig,
    pub hero: HeroConfig,
    pub project: ProjectConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeneralConfig {
    pub language: String,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemConfig {
    pub package_manager_priority: Vec<String>,
    pub default_install_confirm: bool,
    pub update_mirrors: bool,
    pub create_timeshift: bool,
    pub auto_confirm_update: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HeroConfig {
    pub cpu_threshold: f64,
    pub mem_threshold_mb: f64,
    pub default_scope: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectConfig {
    pub default_author: String,
    pub default_email: String,
    pub default_license: String,
    pub use_git_init: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                language: "en".to_string(),
            },
            system: SystemConfig {
                package_manager_priority: vec!["pamac".into(), "paru".into(), "yay".into(), "pacman".into()],
                default_install_confirm: true,
                update_mirrors: true,
                create_timeshift: true,
                auto_confirm_update: false,
            },
            hero: HeroConfig {
                cpu_threshold: 50.0,
                mem_threshold_mb: 400.0,
                default_scope: "user".to_string(),
            },
            project: ProjectConfig {
                default_author: "".to_string(),
                default_email: "".to_string(),
                default_license: "MIT".to_string(),
                use_git_init: true,
            },
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
        let config_dir = if let Some(proj_dirs) = ProjectDirs::from("", "", "genesis") {
            proj_dirs.config_dir().to_path_buf()
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".config").join("genesis")
        };

        let config_path = config_dir.join("config.toml");

        if config_path.exists() {
            match fs::read_to_string(&config_path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(mut loaded_config) => {
                         // Fallback mechanism can be complex with serde. 
                         // For now, if we fail to parse, we log error and return default?
                         // Or if fields are missing, serde default usage?
                         // Simplest is standard serde loading.
                         // To enable partial loading + defaults, we'd need Option<T> in struct and merge logic.
                         // But let's assume valid config or overwrite.
                         // Actually, let's keep it simple: Load full config, if fail, warn and use default.
                         return (config_path, loaded_config);
                    },
                    Err(e) => {
                        eprintln!("Warning: Failed to parse config file: {}. Using defaults.", e);
                    }
                },
                Err(e) => {
                     eprintln!("Warning: Failed to read config file: {}. Using defaults.", e);
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

    pub fn get(&self) -> &Config {
        &self.config
    }
}
