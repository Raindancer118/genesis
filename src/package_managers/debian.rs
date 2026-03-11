use super::{PackageManager, PmPackage, is_available, run_cmd};
use anyhow::Result;
use std::process::Command;

pub struct Apt;

impl PackageManager for Apt {
    fn id(&self) -> &str { "apt" }
    fn display_name(&self) -> &str { "APT (Debian/Ubuntu)" }
    fn is_available(&self) -> bool { is_available("apt") }
    fn needs_sudo(&self) -> bool { true }

    fn update(&self, yes: bool) -> Result<()> {
        run_cmd(&["apt", "update"], true)?;
        let mut args = vec!["apt", "upgrade"];
        if yes { args.push("-y"); }
        run_cmd(&args, true)
    }

    fn search(&self, query: &str) -> Result<Vec<PmPackage>> {
        let output = Command::new("apt").args(["search", query]).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();
        for line in stdout.lines() {
            if line.starts_with("Sorting") || line.starts_with("Full") || line.is_empty() { continue; }
            // "name/release version arch"
            let parts: Vec<&str> = line.splitn(2, '/').collect();
            if let Some(name) = parts.first() {
                if !name.is_empty() && !name.contains(' ') {
                    results.push(PmPackage {
                        name: name.trim().to_string(),
                        version: None,
                        description: None,
                        source: "apt".to_string(),
                    });
                }
            }
        }
        Ok(results)
    }

    fn install(&self, pkg: &str, yes: bool) -> Result<()> {
        let mut args = vec!["apt", "install", pkg];
        if yes { args.push("-y"); }
        run_cmd(&args, true)
    }

    fn uninstall(&self, pkg: &str) -> Result<()> {
        run_cmd(&["apt", "remove", "-y", pkg], true)
    }
}
