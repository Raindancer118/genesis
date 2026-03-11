use super::{PackageManager, PmPackage, PmUpdate, is_available, run_cmd, run_with_spinner};
use anyhow::Result;
use std::process::Command;

pub struct Flatpak;
pub struct Snap;

impl PackageManager for Flatpak {
    fn id(&self) -> &str { "flatpak" }
    fn display_name(&self) -> &str { "Flatpak" }
    fn is_available(&self) -> bool { is_available("flatpak") }

    fn update(&self, _yes: bool) -> Result<()> {
        run_with_spinner(&["flatpak", "update", "-y"], false, "Updating runtimes…")
    }

    fn list_updates(&self) -> Vec<PmUpdate> {
        // flatpak remote-ls --updates: tab-separated application, installed-version, latest-version
        let Ok(out) = Command::new("flatpak")
            .args(["remote-ls", "--updates", "--columns=application,installed-version,version"])
            .output() else { return vec![] };
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| {
                let cols: Vec<&str> = line.splitn(3, '\t').collect();
                if cols.len() >= 3 {
                    let name = cols[0].trim().to_string();
                    let old  = cols[1].trim().to_string();
                    let new  = cols[2].trim().to_string();
                    if !name.is_empty() { Some((name, old, new)) } else { None }
                } else {
                    None
                }
            })
            .collect()
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
        run_with_spinner(&["snap", "refresh"], true, "Refreshing snaps…")
    }

    fn list_updates(&self) -> Vec<PmUpdate> {
        // snap refresh --list: "Name  Version  Rev  Size  Publisher  Notes"
        let Ok(out) = Command::new("snap").args(["refresh", "--list"]).output() else { return vec![] };
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .skip(1) // header row
            .filter_map(|line| {
                let cols: Vec<&str> = line.split_whitespace().collect();
                if cols.len() >= 2 {
                    // snap doesn't expose old version here; show installed as old
                    let name = cols[0].to_string();
                    let new  = cols[1].to_string();
                    Some((name, String::from("installed"), new))
                } else {
                    None
                }
            })
            .collect()
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
