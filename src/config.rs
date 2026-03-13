use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use directories::ProjectDirs;
use anyhow::{Result, Context};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub search: SearchConfig,
    #[serde(default)]
    pub system: SystemConfig,
    #[serde(default)]
    pub analytics: AnalyticsConfig,
    #[serde(default)]
    pub auto_index: AutoIndexConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AutoIndexConfig {
    /// Run a background re-index job automatically
    pub enabled: bool,
    /// How often to re-index (minutes). Default: 120 (2 hours)
    pub interval_minutes: u64,
    /// Paths to index. Empty = use search.default_paths
    pub paths: Vec<String>,
}

impl Default for AutoIndexConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: 120,
            paths: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct SearchConfig {
    /// Paths indexed as "user" scope (searched by default)
    pub default_paths: Vec<String>,
    /// When true, index the entire system in addition to default_paths
    pub full_system_index: bool,
    /// Root paths to walk when full_system_index is enabled (default: ["/"])
    pub system_index_roots: Vec<String>,
    /// Paths that are NEVER indexed (even when full_system_index = true)
    pub system_exclude_paths: Vec<String>,
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
            full_system_index: false,
            system_index_roots: vec!["/".into()],
            system_exclude_paths: vec![
                "/proc".into(), "/sys".into(), "/dev".into(),
                "/run".into(), "/tmp".into(), "/var/tmp".into(),
                "/var/run".into(), "/var/lock".into(),
            ],
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AnalyticsConfig {
    /// Opt-in: send daily anonymous ping to analytics.volantic.de
    pub enabled: bool,
    /// Also track which commands are used (still anonymous)
    pub track_commands: bool,
    /// Anonymous client identifier (auto-generated SHA256 hash)
    pub client_id: String,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            track_commands: false,
            client_id: String::new(), // generated on first run
        }
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
    pub config: Config,
}

impl ConfigManager {
    pub fn new() -> Self {
        let (config_path, mut config) = Self::load_or_default();
        // Auto-generate client_id if missing
        if config.analytics.client_id.is_empty() {
            config.analytics.client_id = Self::generate_client_id();
        }
        // Always save after loading: existing values are preserved by serde,
        // and any new fields added in a version upgrade get written with their
        // defaults — so the on-disk config stays complete after every update.
        let mgr = ConfigManager { config_path: config_path.clone(), config: config.clone() };
        let _ = mgr.save();
        Self { config_path, config }
    }

    fn generate_client_id() -> String {
        use sha2::{Sha256, Digest};
        let hostname = sysinfo::System::host_name().unwrap_or_else(|| "unknown".to_string());
        let username = whoami::username();
        let input = format!("{}:{}", hostname, username);
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        hex::encode(&result[..8])
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

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Path to the auto-index timestamp file.
    pub fn auto_index_stamp_path() -> PathBuf {
        let base = if let Some(proj) = ProjectDirs::from("", "volantic", "genesis") {
            proj.data_local_dir().to_path_buf()
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local").join("share").join("volantic-genesis")
        };
        base.join("last_auto_index")
    }

    /// Seconds since the last auto-index completed. Returns `u64::MAX` if never run.
    pub fn seconds_since_last_auto_index() -> u64 {
        let stamp = Self::auto_index_stamp_path();
        let Ok(content) = fs::read_to_string(&stamp) else { return u64::MAX };
        let Ok(ts) = content.trim().parse::<u64>() else { return u64::MAX };
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now.saturating_sub(ts)
    }

    /// Write the current time as the last auto-index stamp.
    pub fn touch_auto_index_stamp() {
        let stamp = Self::auto_index_stamp_path();
        if let Some(parent) = stamp.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = fs::write(&stamp, now.to_string());
    }
}
