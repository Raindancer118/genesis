use anyhow::Result;
use which::which;

pub mod arch;
pub mod debian;
pub mod universal;
pub mod language;
pub mod homebrew;

#[derive(Debug, Clone)]
pub struct PmPackage {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub source: String,
}

/// A pending package update: (name, old_version, new_version).
pub type PmUpdate = (String, String, String);

pub trait PackageManager: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn is_available(&self) -> bool;
    fn update(&self, yes: bool) -> Result<()>;
    fn search(&self, query: &str) -> Result<Vec<PmPackage>>;
    fn install(&self, pkg: &str, yes: bool) -> Result<()>;
    fn uninstall(&self, pkg: &str) -> Result<()>;
    fn needs_sudo(&self) -> bool { false }
    /// Return pending updates without applying them. Empty = unsupported or none.
    fn list_updates(&self) -> Vec<PmUpdate> { vec![] }
}

pub fn get_all_managers() -> Vec<Box<dyn PackageManager>> {
    vec![
        Box::new(arch::Pamac),
        Box::new(arch::Yay),
        Box::new(arch::Paru),
        Box::new(arch::Pacman),
        Box::new(debian::Apt),
        Box::new(universal::Flatpak),
        Box::new(universal::Snap),
        Box::new(language::Cargo),
        Box::new(language::Npm),
        Box::new(language::Pipx),
        Box::new(homebrew::Brew),
    ]
}

pub fn get_available_managers() -> Vec<Box<dyn PackageManager>> {
    get_all_managers().into_iter().filter(|m| m.is_available()).collect()
}

pub fn is_available(cmd: &str) -> bool {
    which(cmd).is_ok()
}

/// Run a command with inherited I/O (interactive — shows all output).
pub fn run_cmd(args: &[&str], sudo: bool) -> Result<()> {
    run_cmd_impl(args, sudo, false)
}

/// Run a command silently (stdout+stderr discarded).
/// stdin is inherited so TTY-based prompts (e.g. polkit password) still work.
pub fn run_cmd_quiet(args: &[&str], sudo: bool) -> Result<()> {
    run_cmd_impl(args, sudo, false)
}

/// Spawn `args` silently, show a spinner with `label` until it exits, then clear the line.
pub fn run_with_spinner(args: &[&str], sudo: bool, label: &str) -> Result<()> {
    use std::process::{Command, Stdio};
    use std::io::Write;

    let (prog, rest) = if sudo { ("sudo", args) } else { (args[0], &args[1..]) };
    let mut cmd = Command::new(prog);
    if sudo { cmd.args(args); } else { cmd.args(rest); }
    cmd.stdout(Stdio::null()).stderr(Stdio::null());

    let mut child = cmd.spawn()?;
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let mut i = 0usize;

    loop {
        match child.try_wait()? {
            Some(status) => {
                // Clear the spinner line
                print!("\r\x1b[2K");
                std::io::stdout().flush().ok();
                if !status.success() {
                    anyhow::bail!("Command failed: {:?}", args);
                }
                return Ok(());
            }
            None => {
                print!(
                    "\r  \x1b[38;2;96;165;250m{}\x1b[0m  \x1b[38;2;71;85;105m{}\x1b[0m",
                    frames[i % frames.len()],
                    label,
                );
                std::io::stdout().flush().ok();
                i += 1;
                std::thread::sleep(std::time::Duration::from_millis(80));
            }
        }
    }
}

fn run_cmd_impl(args: &[&str], sudo: bool, quiet: bool) -> Result<()> {
    use std::process::{Command, Stdio};
    let (prog, rest) = if sudo { ("sudo", args) } else { (args[0], &args[1..]) };
    let mut cmd = Command::new(prog);
    if sudo { cmd.args(args); } else { cmd.args(rest); }
    if quiet {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }
    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("Command failed: {:?}", args);
    }
    Ok(())
}
