use super::{PackageManager, PmPackage, PmUpdate, is_available, run_cmd, run_with_spinner};
use anyhow::Result;
use std::process::Command;

pub struct Apt;

impl PackageManager for Apt {
    fn id(&self) -> &str { "apt" }
    fn display_name(&self) -> &str { "APT (Debian/Ubuntu)" }
    fn is_available(&self) -> bool { is_available("apt") }
    fn needs_sudo(&self) -> bool { true }

    fn update(&self, _yes: bool) -> Result<()> {
        run_with_spinner(&["apt", "update"], true, "Syncing package index…")?;
        run_with_spinner(&["apt", "upgrade", "-y"], true, "Upgrading packages…")
    }

    fn list_updates(&self) -> Vec<PmUpdate> {
        // Just query the already-cached index; the actual `apt update` runs during update()
        let Ok(out) = Command::new("apt").args(["list", "--upgradable"]).output() else { return vec![] };
        // Format: "name/release new_ver arch [upgradable from: old_ver]"
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| {
                if !line.contains("[upgradable from:") { return None; }
                let name = line.split('/').next()?.trim().to_string();
                let new_ver = line.split_whitespace().nth(1)?.to_string();
                let old_ver = line.split("upgradable from: ")
                    .nth(1)?.trim_end_matches(']').trim().to_string();
                Some((name, old_ver, new_ver))
            })
            .collect()
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
