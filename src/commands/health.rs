use crate::ui;
use anyhow::Result;
use sysinfo::System;
use std::process::Command;
use which::which;

pub fn run() -> Result<()> {
    ui::print_header("SYSTEM HEALTH");

    let mut sys = System::new_all();
    sys.refresh_all();

    // System Info
    ui::section("System");
    ui::info_line("OS", &System::name().unwrap_or_default());
    ui::info_line("Kernel", &System::kernel_version().unwrap_or_default());
    ui::info_line("Hostname", &System::host_name().unwrap_or_default());
    let uptime = System::uptime();
    ui::info_line("Uptime", &format!("{}d {}h {}m", uptime/86400, (uptime%86400)/3600, uptime%3600/60));

    // Resources
    ui::section("Resources");
    let total_mem = sys.total_memory() / 1024 / 1024;
    let used_mem = sys.used_memory() / 1024 / 1024;
    let mem_pct = (used_mem as f64 / total_mem as f64) * 100.0;
    let mem_bar = bar(mem_pct);
    ui::info_line("Memory", &format!("{} / {} MB  {} {:.1}%", used_mem, total_mem, mem_bar, mem_pct));

    let total_swap = sys.total_swap() / 1024 / 1024;
    let used_swap = sys.used_swap() / 1024 / 1024;
    ui::info_line("Swap", &format!("{} / {} MB", used_swap, total_swap));

    let load = System::load_average();
    ui::info_line("Load Avg", &format!("{:.2}  {:.2}  {:.2}", load.one, load.five, load.fifteen));

    // Storage
    ui::section("Storage");
    let disks = sysinfo::Disks::new_with_refreshed_list();
    for disk in &disks {
        let total = disk.total_space();
        let avail = disk.available_space();
        let used = total - avail;
        let pct = (used as f64 / total as f64) * 100.0;
        ui::info_line(
            &disk.mount_point().to_string_lossy(),
            &format!("{} / {}  {:.1}%", fmt_bytes(used), fmt_bytes(total), pct)
        );
    }

    // Integrity
    ui::section("Integrity");

    if cfg!(target_os = "linux") {
        if let Ok(output) = Command::new("systemctl").args(["--failed", "--no-legend"]).output() {
            let out = String::from_utf8_lossy(&output.stdout);
            let count = out.lines().filter(|l| !l.trim().is_empty()).count();
            if count == 0 {
                ui::success("No failed systemd units");
            } else {
                ui::fail(&format!("{} failed systemd unit(s)", count));
            }
        }
    }

    // Pending updates
    if which("checkupdates").is_ok() {
        if let Ok(output) = Command::new("checkupdates").output() {
            let count = String::from_utf8_lossy(&output.stdout).lines().count();
            if count == 0 {
                ui::success("System is up to date");
            } else {
                ui::info_line("Updates", &format!("{} pending", count));
            }
        }
    } else if which("apt").is_ok() {
        if let Ok(output) = Command::new("apt").args(["list", "--upgradable"]).output() {
            let out = String::from_utf8_lossy(&output.stdout);
            let count = out.lines().filter(|l| !l.starts_with("Listing")).count();
            ui::info_line("Updates", &format!("{} pending", count));
        }
    }

    // Volantic service
    if cfg!(target_os = "linux") {
        let status = Command::new("systemctl")
            .args(["--user", "is-active", "genesis-greet.service"])
            .output();
        match status {
            Ok(o) => {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s == "active" {
                    ui::success("vg-greet service: active");
                } else {
                    ui::skip(&format!("vg-greet service: {}", s));
                }
            }
            Err(_) => ui::skip("vg-greet service: unavailable"),
        }
    }

    println!();
    ui::success("Health check complete.");
    Ok(())
}

fn bar(pct: f64) -> String {
    let filled = (pct / 10.0) as usize;
    let empty = 10usize.saturating_sub(filled);
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

fn fmt_bytes(bytes: u64) -> String {
    const UNIT: u64 = 1024;
    if bytes < UNIT { return format!("{} B", bytes); }
    let div = UNIT as f64;
    let exp = (bytes as f64).log(div).floor() as i32;
    let pre = "KMGTPE".chars().nth((exp - 1) as usize).unwrap_or('?');
    format!("{:.1} {}B", (bytes as f64) / div.powi(exp), pre)
}
