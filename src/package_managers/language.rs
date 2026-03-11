use super::{PackageManager, PmPackage, PmUpdate, is_available, run_cmd, run_with_spinner};
use anyhow::Result;
use std::process::Command;

pub struct Cargo;
pub struct Npm;
pub struct Pipx;

impl PackageManager for Cargo {
    fn id(&self) -> &str { "cargo" }
    fn display_name(&self) -> &str { "Cargo (Rust)" }
    fn is_available(&self) -> bool { is_available("cargo") }

    fn update(&self, _yes: bool) -> Result<()> {
        if is_available("cargo-install-update") {
            run_with_spinner(&["cargo", "install-update", "-a"], false, "Updating crates…")
        } else {
            Ok(())
        }
    }

    fn list_updates(&self) -> Vec<PmUpdate> {
        if !is_available("cargo-install-update") { return vec![]; }
        // cargo install-update -l: "Package  Installed  Latest  Needs update"
        let Ok(out) = Command::new("cargo").args(["install-update", "-l"]).output() else { return vec![] };
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .skip(2) // two header lines
            .filter_map(|line| {
                let cols: Vec<&str> = line.split_whitespace().collect();
                // cols: name, installed, latest, "Yes"/"No"
                if cols.len() >= 4 && cols[3].eq_ignore_ascii_case("yes") {
                    Some((cols[0].to_string(), cols[1].to_string(), cols[2].to_string()))
                } else {
                    None
                }
            })
            .collect()
    }

    fn search(&self, query: &str) -> Result<Vec<PmPackage>> {
        let output = Command::new("cargo").args(["search", "--limit", "10", query]).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();
        for line in stdout.lines() {
            if line.trim_start().starts_with('#') || line.is_empty() { continue; }
            // "name = \"version\"    # description"
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if let Some(name) = parts.first() {
                let rest = parts.get(1).unwrap_or(&"");
                let version = rest.split('"').nth(1).map(|v| v.to_string());
                let desc = rest.split('#').nth(1).map(|d| d.trim().to_string());
                results.push(PmPackage {
                    name: name.trim().to_string(),
                    version,
                    description: desc,
                    source: "cargo".to_string(),
                });
            }
        }
        Ok(results)
    }

    fn install(&self, pkg: &str, _yes: bool) -> Result<()> {
        run_cmd(&["cargo", "install", pkg], false)
    }

    fn uninstall(&self, pkg: &str) -> Result<()> {
        run_cmd(&["cargo", "uninstall", pkg], false)
    }
}

impl PackageManager for Npm {
    fn id(&self) -> &str { "npm" }
    fn display_name(&self) -> &str { "npm (Node.js)" }
    fn is_available(&self) -> bool { is_available("npm") }

    fn update(&self, _yes: bool) -> Result<()> {
        run_with_spinner(&["npm", "update", "-g"], false, "Updating global packages…")
    }

    fn list_updates(&self) -> Vec<PmUpdate> {
        // npm outdated -g --json: {"pkg": {"current": "x", "latest": "y"}}
        let Ok(out) = Command::new("npm").args(["outdated", "-g", "--json"]).output() else { return vec![] };
        let text = String::from_utf8_lossy(&out.stdout);
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else { return vec![] };
        json.as_object()
            .map(|map| {
                map.iter().filter_map(|(name, info)| {
                    let current = info["current"].as_str().unwrap_or("?").to_string();
                    let latest  = info["latest"].as_str().unwrap_or("?").to_string();
                    if current != latest {
                        Some((name.clone(), current, latest))
                    } else {
                        None
                    }
                }).collect()
            })
            .unwrap_or_default()
    }

    fn search(&self, query: &str) -> Result<Vec<PmPackage>> {
        let output = Command::new("npm").args(["search", "--json", query]).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(arr) = json.as_array() {
                return Ok(arr.iter().take(10).filter_map(|item| {
                    Some(PmPackage {
                        name: item["name"].as_str()?.to_string(),
                        version: item["version"].as_str().map(String::from),
                        description: item["description"].as_str().map(String::from),
                        source: "npm".to_string(),
                    })
                }).collect());
            }
        }
        Ok(vec![])
    }

    fn install(&self, pkg: &str, _yes: bool) -> Result<()> {
        run_cmd(&["npm", "install", "-g", pkg], false)
    }

    fn uninstall(&self, pkg: &str) -> Result<()> {
        run_cmd(&["npm", "uninstall", "-g", pkg], false)
    }
}

impl PackageManager for Pipx {
    fn id(&self) -> &str { "pipx" }
    fn display_name(&self) -> &str { "pipx (Python)" }
    fn is_available(&self) -> bool { is_available("pipx") }

    fn update(&self, _yes: bool) -> Result<()> {
        run_with_spinner(&["pipx", "upgrade-all"], false, "Upgrading tools…")
    }

    fn search(&self, query: &str) -> Result<Vec<PmPackage>> {
        // pipx has no search; use pip index
        let output = Command::new("pip").args(["index", "versions", query]).output();
        if let Ok(o) = output {
            let stdout = String::from_utf8_lossy(&o.stdout);
            for line in stdout.lines() {
                if line.contains(query) {
                    return Ok(vec![PmPackage {
                        name: query.to_string(),
                        version: None,
                        description: None,
                        source: "pipx".to_string(),
                    }]);
                }
            }
        }
        Ok(vec![])
    }

    fn install(&self, pkg: &str, _yes: bool) -> Result<()> {
        run_cmd(&["pipx", "install", pkg], false)
    }

    fn uninstall(&self, pkg: &str) -> Result<()> {
        run_cmd(&["pipx", "uninstall", pkg], false)
    }
}
