use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use directories::ProjectDirs;
use anyhow::{Result, Context};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub system: SystemConfig,
    #[serde(default)]
    pub hero: HeroConfig,
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub search: SearchConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GeneralConfig {
    pub language: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SystemConfig {
    pub package_manager_priority: Vec<String>,
    pub default_install_confirm: bool,
    pub update_mirrors: bool,
    pub create_timeshift: bool,
    pub auto_confirm_update: bool,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            package_manager_priority: vec!["pamac".into(), "paru".into(), "yay".into(), "pacman".into()],
            default_install_confirm: true,
            update_mirrors: true,
            create_timeshift: true,
            auto_confirm_update: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct HeroConfig {
    pub cpu_threshold: f64,
    pub mem_threshold_mb: f64,
    pub default_scope: String,
}

impl Default for HeroConfig {
    fn default() -> Self {
        Self {
            cpu_threshold: 50.0,
            mem_threshold_mb: 400.0,
            default_scope: "user".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct ProjectConfig {
    pub default_author: String,
    pub default_email: String,
    pub default_license: String,
    pub use_git_init: bool,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            default_author: "".to_string(),
            default_email: "".to_string(),
            default_license: "MIT".to_string(),
            use_git_init: true,
        }
    }
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
            default_paths: vec![std::env::current_dir()
                .unwrap_or_else(|_| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")))
                .to_string_lossy()
                .to_string()],
            ignore_patterns: vec![
                "node_modules".to_string(),
                ".git".to_string(),
                "target".to_string(),
                ".cache".to_string(),
                "__pycache__".to_string(),
                ".npm".to_string(),
                ".cargo".to_string(),
                "venv".to_string(),
                ".venv".to_string(),
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

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            system: SystemConfig::default(),
            hero: HeroConfig::default(),
            project: ProjectConfig::default(),
            search: SearchConfig::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_partial_parse() {
        // Test that config with only [general] section parses correctly
        let toml_content = r#"
[general]
language = "en"
"#;
        
        let config: Result<Config, _> = toml::from_str(toml_content);
        assert!(config.is_ok(), "Failed to parse partial config");
        
        let config = config.unwrap();
        assert_eq!(config.general.language, "en");
        assert_eq!(config.search.max_results, 50); // Should use default
        assert_eq!(config.hero.cpu_threshold, 50.0); // Should use default
    }

    #[test]
    fn test_config_empty_parse() {
        // Test that empty config uses all defaults
        let toml_content = "";
        
        let config: Result<Config, _> = toml::from_str(toml_content);
        assert!(config.is_ok(), "Failed to parse empty config");
        
        let config = config.unwrap();
        assert_eq!(config.general.language, "en"); // Should use default
        assert_eq!(config.search.max_results, 50); // Should use default
    }

    #[test]
    fn test_config_override_defaults() {
        // Test that specified values override defaults
        let toml_content = r#"
[general]
language = "de"

[search]
max_results = 100
"#;
        
        let config: Result<Config, _> = toml::from_str(toml_content);
        if let Err(ref e) = config {
            eprintln!("Parse error: {}", e);
        }
        assert!(config.is_ok(), "Failed to parse config with overrides");
        
        let config = config.unwrap();
        assert_eq!(config.general.language, "de");
        assert_eq!(config.search.max_results, 100);
        assert_eq!(config.hero.cpu_threshold, 50.0); // Should still use default
    }
}
