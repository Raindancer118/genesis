use crate::ui;
use anyhow::{Result, Context, anyhow};
use std::process::Command;
use std::env;
use std::path::Path;

pub fn run() -> Result<()> {
    ui::print_header("SELF UPDATE");

    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().context("Failed to get executable directory")?;

    let project_root = if exe_dir.ends_with("release") && exe_dir.parent().map(|p| p.ends_with("target")).unwrap_or(false) {
        exe_dir.parent().unwrap().parent().unwrap().to_path_buf()
    } else {
        Path::new("/opt/volantic-genesis").to_path_buf()
    };

    if !project_root.exists() || !project_root.join(".git").exists() {
        return Err(anyhow!("Cannot locate Volantic Genesis repository at {:?}", project_root));
    }

    ui::info_line("Repository", &project_root.display().to_string());

    ui::section("Pulling latest changes");
    let status = Command::new("git").arg("pull").current_dir(&project_root).status().context("Failed to run git pull")?;
    if !status.success() { return Err(anyhow!("git pull failed")); }
    ui::success("Repository updated");

    // Show changelog
    let changelog_path = project_root.join("CHANGELOG.md");
    if changelog_path.exists() {
        ui::section("Changelog");
        if let Ok(content) = std::fs::read_to_string(changelog_path) {
            let mut printing = false;
            let mut count = 0;
            for line in content.lines() {
                if line.starts_with("## [") {
                    if printing { break; }
                    printing = true;
                    println!("  {}", line);
                    continue;
                }
                if printing {
                    println!("  {}", line);
                    count += 1;
                    if count > 15 { println!("  ..."); break; }
                }
            }
        }
    }

    ui::section("Rebuilding");
    let cargo = if which::which("cargo").is_ok() { "cargo".to_string() }
    else { format!("{}/.cargo/bin/cargo", env::var("HOME").unwrap_or_default()) };

    let status = Command::new(&cargo)
        .args(["build", "--release"])
        .current_dir(&project_root)
        .status()
        .context("Failed to run cargo build")?;

    if !status.success() { return Err(anyhow!("Build failed")); }

    println!();
    ui::success("Volantic Genesis updated. Please restart vg.");
    Ok(())
}
