use crate::ui;
use anyhow::{anyhow, Context, Result};
use inquire::{Confirm, Select, Text};
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;
use which::which;

// ── ISO discovery ─────────────────────────────────────────────────────────────

/// Returns (filename, download_url) for the latest Manjaro KDE ISO.
/// Strategy:
///   1. Parse the Manjaro KDE download page for a direct download.manjaro.org link
///   2. Fallback: scrape the download.manjaro.org/kde/ directory listing
fn fetch_latest_iso_info() -> Result<(String, String)> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .user_agent("Mozilla/5.0 (compatible; vg-cli)")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?;

    // ── Strategy 1: official download page ──────────────────────────────────
    if let Ok(resp) = client
        .get("https://manjaro.org/downloads/official/kde/")
        .send()
    {
        if let Ok(body) = resp.text() {
            if let Some(result) = extract_iso_from_html(&body) {
                return Ok(result);
            }
        }
    }

    // ── Strategy 2: download.manjaro.org/kde/ directory listing ─────────────
    if let Ok(resp) = client
        .get("https://download.manjaro.org/kde/")
        .send()
    {
        if let Ok(dir_body) = resp.text() {
            // Find version directories like "24.0/", "24.2.1/"
            let mut versions: Vec<String> = Vec::new();
            for chunk in dir_body.split("href=\"") {
                if let Some(end) = chunk.find('"') {
                    let href = &chunk[..end];
                    // Version directories match digit.digit pattern
                    if href.ends_with('/')
                        && href.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                    {
                        versions.push(href.trim_end_matches('/').to_string());
                    }
                }
            }
            versions.sort_by(|a, b| compare_versions(a, b));

            if let Some(latest) = versions.last() {
                let ver_url = format!("https://download.manjaro.org/kde/{}/", latest);
                if let Ok(ver_resp) = client.get(&ver_url).send() {
                    if let Ok(ver_body) = ver_resp.text() {
                        if let Some(result) = extract_iso_from_html(&ver_body) {
                            return Ok(result);
                        }
                        // Build URL manually from href ending in .iso
                        for chunk in ver_body.split("href=\"") {
                            if let Some(end) = chunk.find('"') {
                                let href = &chunk[..end];
                                if href.ends_with(".iso") && href.contains("kde") {
                                    let url = if href.starts_with("http") {
                                        href.to_string()
                                    } else {
                                        format!("https://download.manjaro.org/kde/{}/{}", latest, href)
                                    };
                                    let filename = url
                                        .split('/')
                                        .last()
                                        .unwrap_or("manjaro-kde.iso")
                                        .to_string();
                                    return Ok((filename, url));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Err(anyhow!(
        "Could not find a Manjaro KDE ISO URL.\n\
         Please check https://manjaro.org/downloads/official/kde/ manually."
    ))
}

/// Scan HTML body for href links pointing to a Manjaro KDE ISO.
fn extract_iso_from_html(html: &str) -> Option<(String, String)> {
    for chunk in html.split("href=\"") {
        if let Some(end) = chunk.find('"') {
            let url = &chunk[..end];
            if url.contains("kde") && url.ends_with(".iso")
                && (url.starts_with("https://download.manjaro.org")
                    || url.starts_with("https://mirrors."))
            {
                let filename = url.split('/').last().unwrap_or("manjaro-kde.iso").to_string();
                return Some((filename, url.to_string()));
            }
        }
    }
    None
}

/// Simple version comparison for "24.0" vs "24.2.1" style strings.
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u32> {
        s.split('.').filter_map(|p| p.parse().ok()).collect()
    };
    parse(a).cmp(&parse(b))
}

// ── USB device detection ──────────────────────────────────────────────────────

#[derive(Deserialize)]
struct LsblkOutput {
    blockdevices: Vec<BlockDevice>,
}

#[derive(Deserialize)]
struct BlockDevice {
    name: String,
    size: Option<String>,
    tran: Option<String>,
    vendor: Option<String>,
    model: Option<String>,
    #[serde(rename = "type")]
    dev_type: Option<String>,
}

struct UsbDrive {
    device: String,
    size: String,
    label: String,
}

impl std::fmt::Display for UsbDrive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "/dev/{}  {:>8}  {}", self.device, self.size, self.label)
    }
}

fn list_usb_drives() -> Result<Vec<UsbDrive>> {
    let output = Command::new("lsblk")
        .args(["-J", "-d", "-o", "NAME,SIZE,TRAN,VENDOR,MODEL,TYPE"])
        .output()
        .context("Failed to run lsblk. Is util-linux installed?")?;

    let text = String::from_utf8_lossy(&output.stdout);
    let parsed: LsblkOutput =
        serde_json::from_str(&text).context("Failed to parse lsblk JSON output")?;

    let drives: Vec<UsbDrive> = parsed
        .blockdevices
        .into_iter()
        .filter(|d| {
            d.tran.as_deref() == Some("usb") && d.dev_type.as_deref() == Some("disk")
        })
        .map(|d| {
            let vendor = d.vendor.unwrap_or_default().trim().to_string();
            let model = d.model.unwrap_or_default().trim().to_string();
            let label = match (vendor.is_empty(), model.is_empty()) {
                (true, true) => "Unknown USB device".to_string(),
                (true, false) => model,
                (false, true) => vendor,
                (false, false) => format!("{} {}", vendor, model),
            };
            UsbDrive {
                device: d.name,
                size: d.size.unwrap_or_else(|| "?".to_string()),
                label,
            }
        })
        .collect();

    Ok(drives)
}

// ── ISO download ──────────────────────────────────────────────────────────────

fn download_iso(url: &str, dest: &PathBuf) -> Result<()> {
    // Use wget for streaming download with built-in progress display
    let status = Command::new("wget")
        .args([
            "--progress=bar:force:noscroll",
            "--output-document",
            dest.to_str().unwrap(),
            url,
        ])
        .status()
        .context("Failed to run wget. Please install wget.")?;

    if !status.success() {
        return Err(anyhow!("wget download failed"));
    }
    Ok(())
}

// ── Ventoy ────────────────────────────────────────────────────────────────────

fn ensure_ventoy() -> Result<()> {
    if which("ventoy").is_ok() {
        ui::success("ventoy found.");
        return Ok(());
    }

    ui::skip("ventoy not found — trying to install via pacman/pamac...");

    // Try official repos first (Arch/Manjaro may have it)
    let via_pacman = Command::new("sudo")
        .args(["pacman", "-S", "--noconfirm", "ventoy"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if via_pacman {
        ui::success("ventoy installed via pacman.");
        return Ok(());
    }

    // Try AUR via pamac (Manjaro default)
    let via_pamac = Command::new("pamac")
        .args(["install", "--no-confirm", "ventoy-bin"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if via_pamac {
        ui::success("ventoy installed via pamac.");
        return Ok(());
    }

    Err(anyhow!(
        "Could not install ventoy automatically.\n\
         Please install it manually: pamac install ventoy-bin"
    ))
}

fn install_ventoy(device: &str) -> Result<()> {
    let dev_path = format!("/dev/{}", device);
    // -I  install Ventoy on the disk
    // -g  use GPT partition table (recommended for UEFI)
    let status = Command::new("sudo")
        .args(["ventoy", "-I", "-g", &dev_path])
        .status()
        .context("Failed to run ventoy")?;

    if !status.success() {
        return Err(anyhow!("ventoy installation failed on {}", dev_path));
    }

    // Tell the kernel about new partitions
    let _ = Command::new("sudo")
        .args(["partprobe", &dev_path])
        .status();
    std::thread::sleep(std::time::Duration::from_secs(2));

    Ok(())
}

// ── Copy ISO onto the Ventoy data partition ───────────────────────────────────

fn copy_iso_to_ventoy(iso_path: &PathBuf, device: &str) -> Result<()> {
    let mount_point = "/tmp/vg-ventoy-mount";
    // Ventoy always creates the first partition as the large data partition
    let partition = format!("/dev/{}1", device);

    // Ensure mount point exists
    Command::new("sudo")
        .args(["mkdir", "-p", mount_point])
        .status()
        .context("Failed to create mount point")?;

    // Mount the Ventoy data partition
    let mount_status = Command::new("sudo")
        .args(["mount", &partition, mount_point])
        .status()
        .context("Failed to mount Ventoy data partition")?;

    if !mount_status.success() {
        return Err(anyhow!(
            "Failed to mount {}. \
             The partition may not yet be visible — try unplugging and reinserting the USB stick.",
            partition
        ));
    }

    ui::info_line("Mounted", &format!("{} → {}", partition, mount_point));
    println!("  Copying ISO — this may take a few minutes...");

    // Prefer rsync (shows transfer progress); fall back to cp
    let copy_status = if which("rsync").is_ok() {
        Command::new("sudo")
            .args([
                "rsync",
                "--progress",
                "-h",
                iso_path.to_str().unwrap(),
                &format!("{}/", mount_point),
            ])
            .status()
            .context("rsync failed")?
    } else {
        Command::new("sudo")
            .args(["cp", iso_path.to_str().unwrap(), mount_point])
            .status()
            .context("cp failed")?
    };

    // Sync and unmount regardless of copy result
    let _ = Command::new("sudo").args(["sync"]).status();
    let _ = Command::new("sudo")
        .args(["umount", mount_point])
        .status();

    if !copy_status.success() {
        return Err(anyhow!("Failed to copy ISO to the USB stick."));
    }

    Ok(())
}

// ── ISO source (auto-resolved or user-supplied) ───────────────────────────────

enum IsoSource {
    /// User provided a URL — we need to download the file.
    Url { name: String, url: String },
    /// User pointed to a file already on disk — skip download.
    LocalFile(PathBuf),
}

/// Called when automatic discovery fails. Asks the user to either paste a
/// download URL or pick / type a local .iso file path.
fn ask_iso_manually() -> Result<IsoSource> {
    println!();
    ui::skip("Automatic ISO discovery failed.");
    println!("  You can provide the ISO yourself:");

    const OPT_URL: &str = "Paste a download URL";
    const OPT_FILE: &str = "Select / enter a local .iso file";

    let choice = Select::new("How do you want to supply the ISO?", vec![OPT_URL, OPT_FILE])
        .prompt()?;

    if choice == OPT_URL {
        let url = Text::new("Paste the Manjaro KDE ISO download URL:")
            .prompt()?;
        let url = url.trim().to_string();
        if url.is_empty() {
            return Err(anyhow!("No URL entered."));
        }
        let name = url
            .split('/')
            .last()
            .unwrap_or("manjaro-kde.iso")
            .to_string();
        return Ok(IsoSource::Url { name, url });
    }

    // ── Local file ───────────────────────────────────────────────────────────
    // Scan common directories for .iso files and offer them as quick picks.
    let search_dirs = [
        dirs::download_dir().unwrap_or_else(|| PathBuf::from("~/Downloads")),
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~")),
        PathBuf::from("/tmp"),
    ];

    let mut found: Vec<String> = Vec::new();
    for dir in &search_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("iso") {
                    found.push(path.to_string_lossy().into_owned());
                }
            }
        }
    }
    found.sort();

    const MANUAL_ENTRY: &str = "Enter path manually…";
    let mut options = found.clone();
    options.push(MANUAL_ENTRY.to_string());

    let pick = Select::new("Select an ISO file:", options).prompt()?;

    let path_str = if pick == MANUAL_ENTRY {
        Text::new("Enter the full path to the .iso file:").prompt()?
    } else {
        pick
    };

    let path = PathBuf::from(path_str.trim());
    if !path.exists() {
        return Err(anyhow!("File not found: {}", path.display()));
    }
    Ok(IsoSource::LocalFile(path))
}

// ── Public entry point ────────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    ui::print_header("MANJARO LIVE USB");
    println!("  Creates a bootable Manjaro KDE USB stick with Ventoy.");

    // ── 1. Resolve latest ISO ────────────────────────────────────────────────
    ui::section("Resolving latest Manjaro KDE ISO");
    ui::skip("Querying manjaro.org...");

    let iso_source = match fetch_latest_iso_info() {
        Ok((name, url)) => {
            ui::success(&format!("Latest ISO: {}", name));
            ui::info_line("Download", &url);
            IsoSource::Url { name, url }
        }
        Err(_) => ask_iso_manually()?,
    };

    // ── 2. Detect USB drives ─────────────────────────────────────────────────
    ui::section("Detecting USB drives");

    let drives = list_usb_drives().context("Failed to enumerate USB drives")?;
    if drives.is_empty() {
        ui::fail("No USB drives detected.");
        println!("  Please connect a USB stick (≥ 4 GB) and run the command again.");
        return Ok(());
    }

    for d in &drives {
        ui::info_line("USB", &d.to_string());
    }

    // ── 3. User selects target drive ─────────────────────────────────────────
    println!();
    let options: Vec<String> = drives.iter().map(|d| d.to_string()).collect();
    let selection = Select::new("Which USB device should become the Manjaro stick?", options.clone())
        .prompt()?;
    let idx = options.iter().position(|o| o == &selection).unwrap_or(0);
    let target = &drives[idx];

    // ── 4. Final confirmation ────────────────────────────────────────────────
    println!();
    ui::fail(&format!(
        "ALL data on /dev/{} ({}) will be permanently erased!",
        target.device, target.size
    ));
    let confirmed = Confirm::new(&format!(
        "Continue and format /dev/{} with Ventoy + Manjaro KDE?",
        target.device
    ))
    .with_default(false)
    .prompt()?;

    if !confirmed {
        ui::skip("Aborted. No changes made.");
        return Ok(());
    }

    // ── 5. Obtain ISO path (download or use local) ───────────────────────────
    let iso_path = match iso_source {
        IsoSource::LocalFile(path) => {
            ui::section("Using local ISO");
            ui::info_line("File", path.to_str().unwrap_or("?"));
            path
        }
        IsoSource::Url { name, url } => {
            ui::section("Downloading Manjaro KDE ISO");
            let download_dir = dirs::download_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
            let dest = download_dir.join(&name);

            if dest.exists() {
                    ui::info_line("Found", dest.to_str().unwrap_or("?"));
                    let reuse = Confirm::new("ISO already exists in ~/Downloads. Skip download?")
                        .with_default(true)
                        .prompt()?;
                    if !reuse {
                        std::fs::remove_file(&dest).context("Failed to remove existing ISO")?;
                        download_iso(&url, &dest)?;
                    } else {
                        ui::success("Using existing ISO file.");
                    }
                } else {
                    ui::info_line("Saving to", dest.to_str().unwrap_or("?"));
                    download_iso(&url, &dest)?;
                    ui::success("Download complete.");
                }
            dest
        }
    };

    // ── 6. Install Ventoy ────────────────────────────────────────────────────
    ui::section("Installing Ventoy");
    ensure_ventoy()?;

    ui::info_line("Target", &format!("/dev/{}", target.device));
    println!("  (Ventoy will ask for confirmation before erasing the drive.)");
    install_ventoy(&target.device)?;
    ui::success("Ventoy installed successfully.");

    // ── 7. Copy ISO to stick ─────────────────────────────────────────────────
    ui::section("Writing Manjaro ISO to USB stick");
    copy_iso_to_ventoy(&iso_path, &target.device)?;

    // ── Done ─────────────────────────────────────────────────────────────────
    println!();
    ui::success("Your Manjaro KDE live USB stick is ready!");
    ui::info_line("Device", &format!("/dev/{}", target.device));
    println!();
    println!("  Boot from this stick and select the Manjaro KDE ISO in the Ventoy menu.");

    Ok(())
}
