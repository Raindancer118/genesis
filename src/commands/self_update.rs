use crate::ui;
use anyhow::{Result, Context, anyhow};
use serde::Deserialize;
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const API_URL: &str = "https://api.github.com/repos/Raindancer118/genesis/releases/latest";
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
    body: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct GithubAsset {
    pub name: String,
    pub browser_download_url: String,
}

/// Metadata about an available update. Returned by `check()`.
pub struct UpdateInfo {
    pub latest_version: String,
    pub asset: GithubAsset,
    pub release_notes: Option<String>,
}

fn detect_artifact() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    return "vg-x86_64-linux.tar.gz";
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    return "vg-aarch64-linux.tar.gz";
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    return "vg-x86_64-windows.zip";
    #[allow(unreachable_code)]
    "vg-x86_64-linux.tar.gz"
}

fn fetch_latest_release() -> Result<GithubRelease> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("vg-self-update")
        .build()?;

    let release: GithubRelease = client
        .get(API_URL)
        .send()
        .context("Failed to reach GitHub API")?
        .json()
        .context("Failed to parse release JSON")?;

    Ok(release)
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    let latest  = latest.trim_start_matches('v');
    let current = current.trim_start_matches('v');

    let parse = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<u32> = v.split('.').filter_map(|p| p.parse().ok()).collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };

    parse(latest) > parse(current)
}

/// Try to atomically replace `dst` with `src` using rename().
fn replace_binary(src: &std::path::Path, dst: &std::path::Path) -> bool {
    if fs::rename(src, dst).is_ok() {
        return true;
    }
    let Some(parent) = dst.parent() else { return false };
    let staged = parent.join(".vg-update-staged");
    if fs::copy(src, &staged).is_err() {
        return false;
    }
    #[cfg(unix)]
    let _ = fs::set_permissions(&staged, fs::Permissions::from_mode(0o755));
    if fs::rename(&staged, dst).is_ok() {
        return true;
    }
    let _ = fs::remove_file(&staged);
    false
}

/// Check GitHub for a newer release. Returns `None` if already up to date or unreachable.
pub fn check() -> Option<UpdateInfo> {
    let release = fetch_latest_release().ok()?;
    if !version_is_newer(&release.tag_name, CURRENT_VERSION) {
        return None;
    }
    let artifact_name = detect_artifact();
    let asset = release.assets.iter().find(|a| a.name == artifact_name)?.clone();
    Some(UpdateInfo {
        latest_version: release.tag_name,
        asset,
        release_notes: release.body,
    })
}

/// Download and install the update described by `info`. Shows progress via `ui`.
pub fn apply(info: &UpdateInfo) -> Result<()> {
    let artifact_name = &info.asset.name;

    let tmp_dir = tempfile::tempdir().context("Failed to create temp dir")?;
    let archive_path = tmp_dir.path().join(artifact_name.as_str());

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("vg-self-update")
        .build()?;

    let bytes = client
        .get(&info.asset.browser_download_url)
        .send()
        .context("Download failed")?
        .bytes()
        .context("Failed to read download")?;

    fs::write(&archive_path, &bytes).context("Failed to write archive")?;

    // Extract
    if artifact_name.ends_with(".tar.gz") {
        let status = std::process::Command::new("tar")
            .args(["-xzf", archive_path.to_str().unwrap(), "-C", tmp_dir.path().to_str().unwrap()])
            .status()
            .context("Failed to run tar")?;
        if !status.success() { return Err(anyhow!("tar extraction failed")); }
    } else if artifact_name.ends_with(".zip") {
        let status = std::process::Command::new("unzip")
            .args(["-o", archive_path.to_str().unwrap(), "-d", tmp_dir.path().to_str().unwrap()])
            .status()
            .context("Failed to run unzip")?;
        if !status.success() { return Err(anyhow!("unzip extraction failed")); }
    }

    let new_bin = if artifact_name.ends_with(".zip") {
        tmp_dir.path().join("vg.exe")
    } else {
        tmp_dir.path().join("vg")
    };

    #[cfg(unix)]
    fs::set_permissions(&new_bin, fs::Permissions::from_mode(0o755))
        .context("Failed to set permissions")?;

    let exe_path = env::current_exe().context("Cannot determine current executable path")?;
    let exe_str  = exe_path.to_str().unwrap();
    let new_str  = new_bin.to_str().unwrap();

    if !replace_binary(&new_bin, &exe_path) {
        ui::skip("Needs elevated privileges to replace binary...");
        let status = std::process::Command::new("sudo")
            .args(["install", "-m", "755", new_str, exe_str])
            .status()
            .context("Failed to run sudo install")?;
        if !status.success() {
            return Err(anyhow!("Failed to install new binary"));
        }
    }

    Ok(())
}

/// Entry point for `vg self-update` — interactive, shows header + release notes.
pub fn run() -> Result<()> {
    ui::print_header("SELF UPDATE");
    ui::info_line("Current version", &format!("v{}", CURRENT_VERSION));
    ui::section("Checking for updates");

    let Some(info) = check() else {
        println!();
        ui::success("Already up to date.");
        return Ok(());
    };

    ui::info_line("Latest version", &info.latest_version);
    ui::success(&format!("New version available: {}", info.latest_version));

    if let Some(body) = &info.release_notes {
        let notes: String = body.lines().take(12).collect::<Vec<_>>().join("\n");
        if !notes.trim().is_empty() {
            ui::section("Release Notes");
            for line in notes.lines() {
                println!("  {}", line);
            }
        }
    }

    ui::section(&format!("Downloading {}", info.asset.name));
    apply(&info)?;

    println!();
    ui::success(&format!("Updated to {} — restart vg to use the new version.", info.latest_version));
    Ok(())
}
