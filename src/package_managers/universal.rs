use super::{PackageManager, PmPackage, is_available, run_cmd, run_cmd_quiet};
use anyhow::Result;
use std::process::Command;

pub struct Flatpak;
pub struct Snap;

impl PackageManager for Flatpak {
    fn id(&self) -> &str { "flatpak" }
    fn display_name(&self) -> &str { "Flatpak" }
    fn is_available(&self) -> bool { is_available("flatpak") }

    fn update(&self, _yes: bool) -> Result<()> {
        run_cmd_quiet(&["flatpak", "update", "-y"], false)
    }

    fn search(&self, query: &str) -> Result<Vec<PmPackage>> {
        let output = Command::new("flatpak").args(["search", "--columns=application,name,version,description", query]).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();
        for line in stdout.lines().skip(1) {
            let cols: Vec<&str> = line.splitn(4, '\t').collect();
            if cols.len() >= 2 {
                results.push(PmPackage {
                    name: cols[0].trim().to_string(),
                    version: cols.get(2).map(|v| v.trim().to_string()),
                    description: cols.get(3).map(|d| d.trim().to_string()),
                    source: "flatpak".to_string(),
                });
            }
        }
        Ok(results)
    }

    fn install(&self, pkg: &str, yes: bool) -> Result<()> {
        let mut args = vec!["flatpak", "install", pkg];
        if yes { args.push("-y"); }
        run_cmd(&args, false)
    }

    fn uninstall(&self, pkg: &str) -> Result<()> {
        run_cmd(&["flatpak", "uninstall", pkg, "-y"], false)
    }
}

impl PackageManager for Snap {
    fn id(&self) -> &str { "snap" }
    fn display_name(&self) -> &str { "Snap" }
    fn is_available(&self) -> bool { is_available("snap") }
    fn needs_sudo(&self) -> bool { true }

    fn update(&self, _yes: bool) -> Result<()> {
        run_cmd_quiet(&["snap", "refresh"], true)
    }

    fn search(&self, query: &str) -> Result<Vec<PmPackage>> {
        let output = Command::new("snap").args(["find", query]).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();
        for line in stdout.lines().skip(1) {
            let cols: Vec<&str> = line.splitn(4, ' ').filter(|s| !s.is_empty()).collect();
            if let Some(name) = cols.first() {
                results.push(PmPackage {
                    name: name.trim().to_string(),
                    version: cols.get(1).map(|v| v.trim().to_string()),
                    description: cols.get(3).map(|d| d.trim().to_string()),
                    source: "snap".to_string(),
                });
            }
        }
        Ok(results)
    }

    fn install(&self, pkg: &str, _yes: bool) -> Result<()> {
        run_cmd(&["snap", "install", pkg], true)
    }

    fn uninstall(&self, pkg: &str) -> Result<()> {
        run_cmd(&["snap", "remove", pkg], true)
    }
}
