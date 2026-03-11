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

pub trait PackageManager: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn is_available(&self) -> bool;
    fn update(&self, yes: bool) -> Result<()>;
    fn search(&self, query: &str) -> Result<Vec<PmPackage>>;
    fn install(&self, pkg: &str, yes: bool) -> Result<()>;
    fn uninstall(&self, pkg: &str) -> Result<()>;
    fn needs_sudo(&self) -> bool { false }
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

/// Run a command, returning Ok if exit code 0
pub fn run_cmd(args: &[&str], sudo: bool) -> Result<()> {
    use std::process::Command;
    let (prog, rest) = if sudo {
        ("sudo", args)
    } else {
        (args[0], &args[1..])
    };
    let mut cmd = Command::new(prog);
    if sudo {
        cmd.args(args);
    } else {
        cmd.args(rest);
    }
    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("Command failed: {:?}", args);
    }
    Ok(())
}
