use super::{PackageManager, PmPackage, PmUpdate, is_available, run_cmd, run_cmd_quiet};
use anyhow::Result;
use std::process::Command;

pub struct Brew;

impl PackageManager for Brew {
    fn id(&self) -> &str { "brew" }
    fn display_name(&self) -> &str { "Homebrew" }
    fn is_available(&self) -> bool { is_available("brew") }

    fn update(&self, _yes: bool) -> Result<()> {
        run_cmd_quiet(&["brew", "update"], false)?;
        run_cmd_quiet(&["brew", "upgrade"], false)
    }

    fn list_updates(&self) -> Vec<PmUpdate> {
        // brew fetch index first, then list outdated as JSON
        let _ = Command::new("brew").args(["update", "--quiet"]).output();
        let Ok(out) = Command::new("brew").args(["outdated", "--json=v2"]).output() else { return vec![] };
        let text = String::from_utf8_lossy(&out.stdout);
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { return vec![] };
        let mut updates = Vec::new();
        for category in &["formulae", "casks"] {
            if let Some(arr) = json[category].as_array() {
                for item in arr {
                    let name = item["name"].as_str().unwrap_or("?").to_string();
                    let installed = item["installed_versions"]
                        .as_array()
                        .and_then(|v| v.last())
                        .and_then(|v| v.as_str())
                        .unwrap_or("?").to_string();
                    let latest = item["current_version"].as_str().unwrap_or("?").to_string();
                    updates.push((name, installed, latest));
                }
            }
        }
        updates
    }

    fn search(&self, query: &str) -> Result<Vec<PmPackage>> {
        let output = Command::new("brew").args(["search", query]).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines()
            .filter(|l| !l.trim().is_empty() && !l.contains("==>"))
            .map(|l| PmPackage {
                name: l.trim().to_string(),
                version: None,
                description: None,
                source: "brew".to_string(),
            })
            .collect())
    }

    fn install(&self, pkg: &str, _yes: bool) -> Result<()> {
        run_cmd(&["brew", "install", pkg], false)
    }

    fn uninstall(&self, pkg: &str) -> Result<()> {
        run_cmd(&["brew", "uninstall", pkg], false)
    }
}
