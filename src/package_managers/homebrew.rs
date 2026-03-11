use super::{PackageManager, PmPackage, is_available, run_cmd};
use anyhow::Result;
use std::process::Command;

pub struct Brew;

impl PackageManager for Brew {
    fn id(&self) -> &str { "brew" }
    fn display_name(&self) -> &str { "Homebrew" }
    fn is_available(&self) -> bool { is_available("brew") }

    fn update(&self, _yes: bool) -> Result<()> {
        run_cmd(&["brew", "update"], false)?;
        run_cmd(&["brew", "upgrade"], false)
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
