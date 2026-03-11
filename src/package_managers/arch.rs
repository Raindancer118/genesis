use super::{PackageManager, PmPackage, PmUpdate, is_available, run_cmd, run_with_spinner};
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
        run_with_spinner(&["pamac", "upgrade", "--no-confirm"], false, "Updating packages…")
    }

    fn update_streaming(&self, _yes: bool, on_pkg_done: &mut dyn FnMut(&str)) -> Result<()> {
        use std::process::{Command, Stdio};
        use std::io::{BufRead, BufReader};

        // stdbuf -oL forces line-buffered stdout so we get each line as pamac writes it
        let use_stdbuf = is_available("stdbuf");
        let mut cmd = if use_stdbuf {
            let mut c = Command::new("stdbuf");
            c.args(["-oL", "pamac", "upgrade", "--no-confirm"]);
            c
        } else {
            let mut c = Command::new("pamac");
            c.args(["upgrade", "--no-confirm"]);
            c
        };
        cmd.stdout(Stdio::piped()).stderr(Stdio::null());

        let mut child = cmd.spawn()?;
        if let Some(stdout) = child.stdout.take() {
            for line in BufReader::new(stdout).lines().flatten() {
                if let Some(pkg) = parse_pamac_progress_line(&line) {
                    on_pkg_done(&pkg);
                }
            }
        }
        let status = child.wait()?;
        if !status.success() {
            anyhow::bail!("pamac upgrade failed");
        }
        Ok(())
    }

    fn list_updates(&self) -> Vec<PmUpdate> {
        // pamac checkupdates: "name old_ver -> new_ver [repo]"
        parse_qu_output(Command::new("pamac").args(["checkupdates"]).output().ok())
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
        run_with_spinner(&["yay", "-Syu", "--noconfirm"], false, "Updating packages…")
    }

    fn update_streaming(&self, _yes: bool, on_pkg_done: &mut dyn FnMut(&str)) -> Result<()> {
        streaming_pacman_update(&["yay", "-Syu", "--noconfirm"], false, on_pkg_done)
    }

    fn list_updates(&self) -> Vec<PmUpdate> {
        parse_qu_output(Command::new("yay").args(["-Qu"]).output().ok())
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
        run_with_spinner(&["paru", "-Syu", "--noconfirm"], false, "Updating packages…")
    }

    fn update_streaming(&self, _yes: bool, on_pkg_done: &mut dyn FnMut(&str)) -> Result<()> {
        streaming_pacman_update(&["paru", "-Syu", "--noconfirm"], false, on_pkg_done)
    }

    fn list_updates(&self) -> Vec<PmUpdate> {
        parse_qu_output(Command::new("paru").args(["-Qu"]).output().ok())
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
        run_with_spinner(&["pacman", "-Syu", "--noconfirm"], true, "Updating packages…")
    }

    fn update_streaming(&self, _yes: bool, on_pkg_done: &mut dyn FnMut(&str)) -> Result<()> {
        streaming_pacman_update(&["pacman", "-Syu", "--noconfirm"], true, on_pkg_done)
    }

    fn list_updates(&self) -> Vec<PmUpdate> {
        parse_qu_output(Command::new("pacman").args(["-Qu"]).output().ok())
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

fn streaming_pacman_update(args: &[&str], sudo: bool, on_pkg_done: &mut dyn FnMut(&str)) -> Result<()> {
    use std::process::{Command, Stdio};
    use std::io::{BufRead, BufReader};

    let (prog, rest) = if sudo { ("sudo", args) } else { (args[0], &args[1..]) };
    let mut cmd = Command::new(prog);
    if sudo { cmd.args(args); } else { cmd.args(rest); }

    let mut child = if is_available("stdbuf") {
        let mut c = Command::new("stdbuf");
        c.arg("-oL");
        if sudo { c.arg("sudo").args(args); } else { c.arg(args[0]).args(rest); }
        c.stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?
    } else {
        cmd.stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?
    };

    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().flatten() {
            if let Some(pkg) = parse_pacman_progress_line(&line) {
                on_pkg_done(&pkg);
            }
        }
    }
    let status = child.wait()?;
    // Exit code 1 from yay/paru typically means "nothing to do" — not a real error
    if !status.success() && status.code() != Some(1) {
        anyhow::bail!("Command failed: {:?}", args);
    }
    Ok(())
}

/// Extract a package name from a pamac progress line.
/// Handles German ("Erneuere foo", "Installiere foo") and English ("Upgrading foo", "Installing foo").
fn parse_pamac_progress_line(line: &str) -> Option<String> {
    let line = line.trim();
    for prefix in &["Erneuere ", "Installiere ", "Upgrading ", "Installing ", "Reinstalling "] {
        if let Some(rest) = line.strip_prefix(prefix) {
            let name = rest.split(|c: char| c == ' ' || c == '(').next()?;
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Extract a package name from pacman/yay/paru transaction lines.
/// Matches "(N/M) upgrading foo" / "(N/M) installing foo" patterns.
fn parse_pacman_progress_line(line: &str) -> Option<String> {
    let line = line.trim();
    // Strip optional "(N/M) " prefix
    let rest = if line.starts_with('(') {
        line.splitn(2, ") ").nth(1).unwrap_or(line)
    } else {
        line
    };
    for prefix in &["upgrading ", "installing ", "reinstalling "] {
        if let Some(name) = rest.strip_prefix(prefix) {
            let pkg = name.split_whitespace().next()?;
            if !pkg.is_empty() { return Some(pkg.to_string()); }
        }
    }
    None
}

/// Parse `name old_ver -> new_ver [extras]` lines from pacman/yay/paru/pamac -Qu output.
pub fn parse_qu_output(out: Option<std::process::Output>) -> Vec<PmUpdate> {
    let Some(out) = out else { return vec![] };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() >= 4 && p[2] == "->" {
                Some((p[0].to_string(), p[1].to_string(), p[3].to_string()))
            } else {
                None
            }
        })
        .collect()
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
