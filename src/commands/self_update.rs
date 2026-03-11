use crate::ui;
use anyhow::{Result, Context, anyhow};
use serde::Deserialize;
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const REPO: &str = "Raindancer118/genesis";
const API_URL: &str = "https://api.github.com/repos/Raindancer118/genesis/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
    body: Option<String>,
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
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
    // Strip leading 'v'
    let latest = latest.trim_start_matches('v');
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

pub fn run() -> Result<()> {
    ui::print_header("SELF UPDATE");

    ui::info_line("Current version", &format!("v{}", CURRENT_VERSION));
    ui::section("Checking for updates");

    let release = fetch_latest_release().context("Could not fetch release info from GitHub")?;
    let latest_version = &release.tag_name;

    ui::info_line("Latest version", latest_version);

    if !version_is_newer(latest_version, CURRENT_VERSION) {
        println!();
        ui::success("Already up to date.");
        return Ok(());
    }

    ui::success(&format!("New version available: {}", latest_version));

    // Show release notes
    if let Some(body) = &release.body {
        let notes: String = body.lines().take(12).collect::<Vec<_>>().join("\n");
        if !notes.trim().is_empty() {
            ui::section("Release Notes");
            for line in notes.lines() {
                println!("  {}", line);
            }
        }
    }

    // Find the right artifact
    let artifact_name = detect_artifact();
    let asset = release.assets.iter()
        .find(|a| a.name == artifact_name)
        .ok_or_else(|| anyhow!("No artifact '{}' found in release {}", artifact_name, latest_version))?;

    ui::section(&format!("Downloading {}", artifact_name));

    // Download to temp file
    let tmp_dir = tempfile::tempdir().context("Failed to create temp dir")?;
    let archive_path = tmp_dir.path().join(artifact_name);

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .user_agent("vg-self-update")
        .build()?;

    let bytes = client
        .get(&asset.browser_download_url)
        .send()
        .context("Download failed")?
        .bytes()
        .context("Failed to read download")?;

    fs::write(&archive_path, &bytes).context("Failed to write archive")?;
    ui::success(&format!("{:.1} MB downloaded", bytes.len() as f64 / 1_048_576.0));

    // Extract
    ui::section("Installing");
    let bin_path = tmp_dir.path().join("vg");

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

    // Replace current binary
    let exe_path = env::current_exe().context("Cannot determine current executable path")?;

    // Try direct copy first, fall back to sudo
    let new_bin = if artifact_name.ends_with(".zip") {
        tmp_dir.path().join("vg.exe")
    } else {
        bin_path
    };

    // Make executable
    #[cfg(unix)]
    fs::set_permissions(&new_bin, fs::Permissions::from_mode(0o755))
        .context("Failed to set permissions")?;

    // On Linux a running binary is locked (ETXTBSY) — remove it first, then copy.
    // Try without sudo first, fall back to sudo for both steps.
    let exe_str = exe_path.to_str().unwrap();
    let new_str = new_bin.to_str().unwrap();

    // Try without sudo first; if permission denied, fall back to sudo.
    // (permissions().readonly() doesn't reflect actual user access — just try it.)
    if fs::remove_file(&exe_path).is_ok() {
        fs::copy(&new_bin, &exe_path).context("Failed to copy new binary")?;
    } else {
        ui::skip("Needs elevated privileges to replace binary...");
        let rm = std::process::Command::new("sudo").args(["rm", "-f", exe_str]).status()?;
        if !rm.success() { return Err(anyhow!("Failed to remove old binary")); }
        let cp = std::process::Command::new("sudo").args(["cp", new_str, exe_str]).status()?;
        if !cp.success() { return Err(anyhow!("Failed to copy new binary")); }
        std::process::Command::new("sudo").args(["chmod", "+x", exe_str]).status()?;
    }

    println!();
    ui::success(&format!("Updated to {} — restart vg to use the new version.", latest_version));
    Ok(())
}
