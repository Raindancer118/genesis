// src/analytics.rs
use crate::config::ConfigManager;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use directories::ProjectDirs;

const ANALYTICS_BASE_URL: &str = "https://analytics.volantic.de/v1";
const PING_INTERVAL_SECS: u64 = 86400; // 24 hours

#[derive(Serialize)]
struct PingPayload {
    client_id: String,
    version: String,
    os: String,
    arch: String,
    timestamp: String,
}

#[derive(Serialize)]
struct EventPayload {
    client_id: String,
    event: String,
    command: String,
    version: String,
    timestamp: String,
}

fn get_state_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("", "volantic", "genesis") {
        proj_dirs.data_dir().join("analytics_state.json")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".local").join("share").join("volantic-genesis").join("analytics_state.json")
    }
}

#[derive(Serialize, Deserialize, Default)]
struct AnalyticsState {
    last_ping: Option<String>,
}

fn load_state() -> AnalyticsState {
    let path = get_state_path();
    if let Ok(content) = std::fs::read_to_string(&path) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AnalyticsState::default()
    }
}

fn save_state(state: &AnalyticsState) {
    let path = get_state_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(state) {
        let _ = std::fs::write(path, json);
    }
}

fn should_ping() -> bool {
    let state = load_state();
    if let Some(last_ping_str) = state.last_ping {
        if let Ok(last_ping) = chrono::DateTime::parse_from_rfc3339(&last_ping_str) {
            let elapsed = Utc::now().signed_duration_since(last_ping.with_timezone(&Utc));
            return elapsed.num_seconds() as u64 >= PING_INTERVAL_SECS;
        }
    }
    true
}

fn get_client_id(config: &ConfigManager) -> String {
    config.config.analytics.client_id.clone()
}

/// Send daily ping in background (non-blocking, daily max)
pub fn maybe_ping(config: &ConfigManager) {
    if !config.config.analytics.enabled { return; }
    if !should_ping() { return; }

    let client_id = get_client_id(config);
    let version = env!("CARGO_PKG_VERSION").to_string();
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();

    // Spawn background thread — doesn't block CLI
    std::thread::spawn(move || {
        let payload = PingPayload {
            client_id,
            version,
            os,
            arch,
            timestamp: Utc::now().to_rfc3339(),
        };

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build();

        if let Ok(client) = client {
            let url = format!("{}/ping", ANALYTICS_BASE_URL);
            let _ = client.post(&url).json(&payload).send();
        }

        // Update last ping timestamp
        let state = AnalyticsState {
            last_ping: Some(Utc::now().to_rfc3339()),
        };
        save_state(&state);
    });
}

/// Send command event (only if track_commands enabled)
pub fn track_command(config: &ConfigManager, command: &str) {
    if !config.config.analytics.enabled { return; }
    if !config.config.analytics.track_commands { return; }

    let client_id = get_client_id(config);
    let version = env!("CARGO_PKG_VERSION").to_string();
    let command = command.to_string();

    std::thread::spawn(move || {
        let payload = EventPayload {
            client_id,
            event: "command".to_string(),
            command,
            version,
            timestamp: Utc::now().to_rfc3339(),
        };

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build();

        if let Ok(client) = client {
            let url = format!("{}/event", ANALYTICS_BASE_URL);
            let _ = client.post(&url).json(&payload).send();
        }
    });
}
