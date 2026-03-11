use super::{PackageManager, PmPackage, PmUpdate, is_available, run_cmd, run_cmd_quiet};
use anyhow::Result;
use std::process::Command;

pub struct Pamac;
pub struct Yay;
pub struct Paru;
pub struct Pacman;

impl PackageManager for Pamac {
    fn id(&self) -> &str { "pamac" }
    fn display_name(&self) -> &str { "Pamac (Arch/Manjaro)" }
    fn is_available(&self) -> bool { is_available("pamac") }
    fn needs_sudo(&self) -> bool { false }

    fn update(&self, _yes: bool) -> Result<()> {
        // Always non-interactive: we show the package list ourselves before running.
        run_cmd_quiet(&["pamac", "upgrade", "--no-confirm"], false)
    }

    fn list_updates(&self) -> Vec<PmUpdate> {
        let Ok(out) = Command::new("pamac").args(["checkupdates"]).output() else { return vec![] };
        let text = String::from_utf8_lossy(&out.stdout);
        let mut updates = Vec::new();
        for line in text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            // Format: "name old_ver -> new_ver [repo]"
            if parts.len() >= 4 && parts[2] == "->" {
                updates.push((parts[0].to_string(), parts[1].to_string(), parts[3].to_string()));
            }
        }
        updates
    }

    fn search(&self, query: &str) -> Result<Vec<PmPackage>> {
        let output = Command::new("pamac").args(["search", query]).output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut results = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(' ') { continue; }
            // pamac output: "name version\n    description"
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            if let Some(name) = parts.first() {
                results.push(PmPackage {
                    name: name.to_string(),
                    version: parts.get(1).map(|v| v.trim().to_string()),
                    description: None,
                    source: "pamac".to_string(),
                });
            }
        }
        Ok(results)
    }

    fn install(&self, pkg: &str, yes: bool) -> Result<()> {
        let mut args = vec!["pamac", "install", pkg];
        if yes { args.push("--no-confirm"); }
        run_cmd(&args, false)
    }

    fn uninstall(&self, pkg: &str) -> Result<()> {
        run_cmd(&["pamac", "remove", pkg, "--no-confirm"], false)
    }
}

impl PackageManager for Yay {
    fn id(&self) -> &str { "yay" }
    fn display_name(&self) -> &str { "Yay (AUR)" }
    fn is_available(&self) -> bool { is_available("yay") }

    fn update(&self, _yes: bool) -> Result<()> {
        run_cmd_quiet(&["yay", "-Syu", "--noconfirm"], false)
    }

    fn search(&self, query: &str) -> Result<Vec<PmPackage>> {
        let output = Command::new("yay").args(["-Ss", query]).output()?;
        parse_pacman_search(&String::from_utf8_lossy(&output.stdout), "yay")
    }

    fn install(&self, pkg: &str, yes: bool) -> Result<()> {
        let mut args = vec!["yay", "-S", pkg];
        if yes { args.push("--noconfirm"); }
        run_cmd(&args, false)
    }

    fn uninstall(&self, pkg: &str) -> Result<()> {
        run_cmd(&["yay", "-Rns", pkg, "--noconfirm"], false)
    }
}

impl PackageManager for Paru {
    fn id(&self) -> &str { "paru" }
    fn display_name(&self) -> &str { "Paru (AUR)" }
    fn is_available(&self) -> bool { is_available("paru") }

    fn update(&self, _yes: bool) -> Result<()> {
        run_cmd_quiet(&["paru", "-Syu", "--noconfirm"], false)
    }

    fn search(&self, query: &str) -> Result<Vec<PmPackage>> {
        let output = Command::new("paru").args(["-Ss", query]).output()?;
        parse_pacman_search(&String::from_utf8_lossy(&output.stdout), "paru")
    }

    fn install(&self, pkg: &str, yes: bool) -> Result<()> {
        let mut args = vec!["paru", "-S", pkg];
        if yes { args.push("--noconfirm"); }
        run_cmd(&args, false)
    }

    fn uninstall(&self, pkg: &str) -> Result<()> {
        run_cmd(&["paru", "-Rns", pkg, "--noconfirm"], false)
    }
}

impl PackageManager for Pacman {
    fn id(&self) -> &str { "pacman" }
    fn display_name(&self) -> &str { "Pacman (Arch)" }
    fn is_available(&self) -> bool { is_available("pacman") }
    fn needs_sudo(&self) -> bool { true }

    fn update(&self, _yes: bool) -> Result<()> {
        run_cmd_quiet(&["pacman", "-Syu", "--noconfirm"], true)
    }

    fn search(&self, query: &str) -> Result<Vec<PmPackage>> {
        let output = Command::new("pacman").args(["-Ss", query]).output()?;
        parse_pacman_search(&String::from_utf8_lossy(&output.stdout), "pacman")
    }

    fn install(&self, pkg: &str, yes: bool) -> Result<()> {
        let mut args = vec!["pacman", "-S", pkg];
        if yes { args.push("--noconfirm"); }
        run_cmd(&args, true)
    }

    fn uninstall(&self, pkg: &str) -> Result<()> {
        run_cmd(&["pacman", "-Rns", pkg, "--noconfirm"], true)
    }
}

pub fn parse_pacman_search(output: &str, source: &str) -> Result<Vec<PmPackage>> {
    let mut results = Vec::new();
    let mut lines = output.lines().peekable();
    while let Some(line) = lines.next() {
        if line.starts_with("    ") || line.is_empty() { continue; }
        // Format: "repo/name version [installed]"
        // or: "aur/name version"
        let desc = lines.peek().map(|l| l.trim().to_string());
        let parts: Vec<&str> = line.splitn(2, '/').collect();
        let name_ver = if parts.len() == 2 { parts[1] } else { line };
        let nv: Vec<&str> = name_ver.splitn(2, ' ').collect();
        if let Some(name) = nv.first() {
            let version = nv.get(1).map(|v| {
                // Remove "[installed]" etc
                v.split('[').next().unwrap_or(v).trim().to_string()
            });
            results.push(PmPackage {
                name: name.trim().to_string(),
                version,
                description: desc.clone(),
                source: source.to_string(),
            });
        }
        if desc.is_some() { lines.next(); }
    }
    Ok(results)
}
