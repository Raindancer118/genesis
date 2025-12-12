use crate::config::ConfigManager;
use anyhow::{Result, anyhow};
use colored::Colorize;
use inquire::Confirm;
use std::process::Command;
use sysinfo::System;
use which::which;

// --- INSTALL ---
pub fn install(packages: Vec<String>, config: &ConfigManager) -> Result<()> {
    if packages.is_empty() {
        println!("No packages specified.");
        return Ok(());
    }

    println!("{}", "📦 Package Installation".bold().cyan());

    // Strategy: Try system PMs first, then universal/3rd party.
    
    // Arch
    if which("pacman").is_ok() {
        return handle_arch_install(packages, config);
    }
    // Debian
    if which("apt-get").is_ok() || which("apt").is_ok() {
        return handle_debian_install(packages, config);
    }
    // Fedora/RHEL
    if which("dnf").is_ok() {
        return run_install("dnf", "install", &packages, true, config);
    }
    // OpenSUSE
    if which("zypper").is_ok() {
        return run_install("zypper", "install", &packages, true, config);
    }
    // Alpine
    if which("apk").is_ok() {
        return run_install("apk", "add", &packages, true, config);
    }
    // Void
    if which("xbps-install").is_ok() {
        return run_install("xbps-install", "-S", &packages, true, config);
    }
    // Gentoo
    if which("emerge").is_ok() {
        return run_install("emerge", "", &packages, true, config); // emerge pkg
    }
    // Nix
    if which("nix-env").is_ok() {
        return run_install("nix-env", "-iA", &packages, false, config); // Nix often user-level
    }
    // Homebrew
    if which("brew").is_ok() {
        return run_install("brew", "install", &packages, false, config);
    }

    // Windows
    if cfg!(windows) {
        if which("choco").is_ok() {
            handle_windows_install(packages, config)?;
            return Ok(());
        }
        if which("winget").is_ok() {
            handle_windows_install(packages, config)?;
            return Ok(());
        }
        if which("scoop").is_ok() {
            println!("Using Scoop.");
             for pkg in packages {
                println!("Installing {}...", pkg);
                Command::new("scoop").arg("install").arg(&pkg).status()?;
            }
            return Ok(());
        }
    }

    Err(anyhow!("No supported package manager found."))
}

fn run_install(cmd: &str, action: &str, packages: &[String], sudo: bool, config: &ConfigManager) -> Result<()> {
    println!("Using {}", cmd);
    if config.config.system.default_install_confirm {
        if !Confirm::new(&format!("Proceed with {}?", cmd)).with_default(true).prompt()? {
            println!("Cancelled.");
            return Ok(());
        }
    }
    
    let mut command = if sudo { Command::new("sudo") } else { Command::new(cmd) };
    if sudo { command.arg(cmd); }
    
    if !action.is_empty() {
        command.arg(action);
    }

    // Manager specific flags could go here, but keeping it simple for now
    if cmd == "dnf" || cmd == "zypper" || cmd == "apt" || cmd == "apt-get" {
        command.arg("-y");
    }
    
    command.args(packages).status()?;
    Ok(())
}

// ... (Arch/Debian/Windows handlers remain similar, refactored slightly) ...
fn handle_arch_install(packages: Vec<String>, config: &ConfigManager) -> Result<()> {
    // Simplified for brevity in this large replacement, but keeping core logic
    if config.config.system.default_install_confirm {
        if !Confirm::new("Proceed with Pacman/Yay?").with_default(true).prompt()? { return Ok(()); }
    }
    // Try yay/paru first if execution
    if which("yay").is_ok() {
        Command::new("yay").arg("-S").args(&packages).status()?;
    } else if which("paru").is_ok() {
        Command::new("paru").arg("-S").args(&packages).status()?;
    } else {
        Command::new("sudo").arg("pacman").arg("-S").args(&packages).status()?;
    }
    Ok(())
}

fn handle_debian_install(packages: Vec<String>, config: &ConfigManager) -> Result<()> {
    run_install("apt", "install", &packages, true, config)
}

fn handle_windows_install(packages: Vec<String>, config: &ConfigManager) -> Result<()> {
    if which("choco").is_ok() {
        if config.config.system.default_install_confirm {
             if !Confirm::new("Proceed with Chocolatey?").with_default(true).prompt()? { return Ok(()); }
        }
        for pkg in packages {
            Command::new("choco").arg("install").arg(&pkg).arg("-y").status()?;
        }
    } else if which("winget").is_ok() {
         if config.config.system.default_install_confirm {
             if !Confirm::new("Proceed with Winget?").with_default(true).prompt()? { return Ok(()); }
        }
        for pkg in packages {
            Command::new("winget").args(["install", "-e", "--id", &pkg]).status()?;
        }
    }
    Ok(())
}

// --- UPDATE ---
pub fn update(yes: bool, _config: &ConfigManager) -> Result<()> {
    println!("{}", "🔄 System Update Initiated".bold().blue());

    // Helper macro to run commands
    macro_rules! run {
        ($name:expr, $cmd:expr, $args:expr) => {
             if which($cmd).is_ok() {
                 println!("{}", format!("--- {} ---", $name).bold().magenta());
                 let mut c = Command::new($cmd);
                 c.args($args);
                 if let Err(e) = c.status() {
                     println!("{} update failed: {}", $name, e);
                 }
             }
        };
        (sudo $name:expr, $cmd:expr, $args:expr) => {
             if which($cmd).is_ok() {
                 println!("{}", format!("--- {} ---", $name).bold().magenta());
                 let mut c = Command::new("sudo");
                 c.arg($cmd).args($args);
                 if let Err(e) = c.status() {
                     println!("{} update failed: {}", $name, e);
                 }
             }
        };
    }

    // 1. Arch
    if which("pacman").is_ok() {
        println!("{}", "--- Arch Linux ---".bold().blue());
        let mut args = vec!["-Syu"];
        if yes { args.push("--noconfirm"); }
        if which("yay").is_ok() {
            Command::new("yay").args(&args).status()?;
        } else if which("paru").is_ok() {
            Command::new("paru").args(&args).status()?;
        } else if which("pamac").is_ok() {
            let mut p_args = vec!["upgrade"];
            if yes { p_args.push("--no-confirm"); }
             Command::new("pamac").args(&p_args).status()?;
        } else {
             Command::new("sudo").arg("pacman").args(&args).status()?;
        }
    }

    // 2. Debian
    if which("apt").is_ok() || which("apt-get").is_ok() {
        if which("nala").is_ok() {
            run!(sudo "Debian (Nala)", "nala", ["upgrade", "-y"]);
        } else {
            run!(sudo "Debian (Apt)", "apt-get", ["update"]);
            let mut args = vec!["upgrade"];
            if yes { args.push("-y"); }
            run!(sudo "Debian (Apt)", "apt-get", args);
        }
    }

    // 3. Fedora
    let mut dnf_args = vec!["upgrade", "--refresh"];
    if yes { dnf_args.push("-y"); }
    run!(sudo "Fedora (DNF)", "dnf", dnf_args);

    // 4. OpenSUSE
    let mut zyp_args = vec!["update"];
    if yes { zyp_args.push("-y"); }
    run!(sudo "OpenSUSE (Zypper)", "zypper", zyp_args);

    // 5. Alpine
    run!(sudo "Alpine (APK)", "apk", ["upgrade"]);

    // 6. Void
    run!(sudo "Void (XBPS)", "xbps-install", ["-Su"]);

    // 7. Gentoo
    // emerge --sync usually separate, but let's do update world
    // emerge -auUDN @world
    run!(sudo "Gentoo (Emerge)", "emerge", ["-uUDN", "@world"]);

    // 8. Nix
    // nix-channel --update && nix-env -u
    // Handling just nix-env -u for now as user-level
    run!("Nix", "nix-env", ["-u"]);

    // 9. Homebrew
    run!("Homebrew", "brew", ["upgrade"]);

    // --- Universal ---
    let mut flat_args = vec!["update"];
    if yes { flat_args.push("-y"); }
    run!("Flatpak", "flatpak", flat_args);

    run!(sudo "Snap", "snap", ["refresh"]);

    // --- Language ---
    if which("cargo").is_ok() {
         // Try cargo install-update
         let _ = Command::new("cargo").args(["install-update", "-a"]).status();
    }
    run!(sudo "NPM Global", "npm", ["update", "-g"]);
    run!("Ruby Gems", "gem", ["update"]);
    run!("Pipx", "pipx", ["upgrade-all"]);

    // --- Windows ---
    if cfg!(windows) {
        let mut choco_args = vec!["upgrade", "all"];
        if yes { choco_args.push("-y"); }
        run!("Chocolatey", "choco", choco_args);

        run!("Winget", "winget", ["upgrade", "--all"]);
        
        run!("Scoop", "scoop", ["update", "*"]);
    }

    println!("{}", "\n✅ Universal system update complete.".bold().green());
    Ok(())
}

// --- SEARCH ---
pub fn search(query: String, _config: &ConfigManager) -> Result<()> {
    println!("{}", format!("🔍 Searching for '{}'...", query).bold().magenta());
    let mut found = false;

    macro_rules! s {
        ($name:expr, $cmd:expr, $args:expr) => {
            if which($cmd).is_ok() {
                println!("{}", format!("--- {} ---", $name).bold().cyan());
                if Command::new($cmd).args($args).status().is_ok() { found = true; }
            }
        };
    }

    s!("Arch (Pacman)", "pacman", ["-Ss", &query]);
    s!("Arch (Yay)", "yay", ["-Ss", &query]);
    s!("Debian (Apt)", "apt", ["search", &query]);
    s!("Fedora (DNF)", "dnf", ["search", &query]);
    s!("OpenSUSE (Zypper)", "zypper", ["search", &query]);
    s!("Alpine (APK)", "apk", ["search", &query]);
    s!("Void (XBPS)", "xbps-query", ["-Js", &query]); // -Rs for remote? -Js for json? -Rs is search
    s!("Gentoo (Emerge)", "emerge", ["--search", &query]);
    s!("Nix", "nix-env", ["-qa", &query]);
    s!("Homebrew", "brew", ["search", &query]);
    s!("Flatpak", "flatpak", ["search", &query]);
    s!("Snap", "snap", ["find", &query]);
    s!("Cargo", "cargo", ["search", &query]);
    s!("NPM", "npm", ["search", &query]);
    
    if cfg!(windows) {
        s!("Chocolatey", "choco", ["search", &query]);
        s!("Winget", "winget", ["search", &query]);
        s!("Scoop", "scoop", ["search", &query]);
    }

    if !found { println!("No results."); }
    Ok(())
}

// --- REMOVE ---
pub fn remove(packages: Vec<String>, config: &ConfigManager) -> Result<()> {
    if packages.is_empty() { return Ok(()); }
    println!("{}", format!("🗑️  Removing packages: {:?}", packages).bold().red());

    // Try all managers that are present
    macro_rules! rem {
        ($cmd:expr, $args:expr, $sudo:expr) => {
            if which($cmd).is_ok() {
                if config.config.system.default_install_confirm {
                     if !Confirm::new(&format!("Try removing via {}?", $cmd)).with_default(true).prompt()? {
                         // skip
                     } else {
                         let mut c = if $sudo { Command::new("sudo") } else { Command::new($cmd) };
                         if $sudo { c.arg($cmd); }
                         c.args($args).args(&packages).status()?;
                     }
                } else {
                     let mut c = if $sudo { Command::new("sudo") } else { Command::new($cmd) };
                     if $sudo { c.arg($cmd); }
                     c.args($args).args(&packages).status()?;
                }
            }
        };
    }
    
    // Arch
    rem!("pacman", ["-Rns"], true);
    // Debian
    rem!("apt", ["remove", "-y"], true);
    // Fedora
    rem!("dnf", ["remove", "-y"], true);
    // OpenSUSE
    rem!("zypper", ["remove", "-y"], true);
    // Alpine
    rem!("apk", ["del"], true);
    // Void
    rem!("xbps-remove", ["-R"], true);
    // Gentoo
    rem!("emerge", ["-C"], true); // unmerge
    // Nix
    rem!("nix-env", ["-e"], false);
    // Brew
    rem!("brew", ["uninstall"], false);
    // Flatpak
    rem!("flatpak", ["uninstall", "-y"], false);
    // Snap
    rem!("snap", ["remove"], true);
    // Cargo -- cannot multi remove easily unless looped.
    // Simplifying:
    if which("cargo").is_ok() {
        for p in &packages { Command::new("cargo").args(["uninstall", p]).status()?; }
    }
    // Pipx
    if which("pipx").is_ok() {
        for p in &packages { Command::new("pipx").args(["uninstall", p]).status()?; }
    }

    // Windows
    if cfg!(windows) {
        if which("choco").is_ok() {
             for p in &packages { Command::new("choco").args(["uninstall", p, "-y"]).status()?; }
        }
        if which("winget").is_ok() {
             for p in &packages { Command::new("winget").args(["uninstall", "--id", p]).status()?; }
        }
         if which("scoop").is_ok() {
             for p in &packages { Command::new("scoop").args(["uninstall", p]).status()?; }
        }
    }

    Ok(())
}

pub fn info() {
    let mut sys = System::new_all();
    sys.refresh_all(); 

    println!("{}", "System Information".bold().green());
    println!("{}: {}", "OS".bold(), System::name().unwrap_or("Unknown".into()));
    println!("{}: {}", "Kernel".bold(), System::kernel_version().unwrap_or("Unknown".into()));
    println!("{}: {}", "Host Name".bold(), System::host_name().unwrap_or("Unknown".into()));
    println!("{}: {} cores", "CPU".bold(), sys.cpus().len());
    println!("{}: {} MB / {} MB", 
        "Memory".bold(), 
        sys.used_memory() / 1024 / 1024, 
        sys.total_memory() / 1024 / 1024
    );

    println!("\n{}", "Disks".bold());
    let disks = sysinfo::Disks::new_with_refreshed_list();
    for disk in &disks {
         println!("{}: {} / {} ({} free)", 
            disk.name().to_string_lossy(),
            format_bytes(disk.total_space() - disk.available_space()),
            format_bytes(disk.total_space()),
            format_bytes(disk.available_space())
         );
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNIT: u64 = 1024;
    if bytes < UNIT {
        return format!("{} B", bytes);
    }
    let div = UNIT as f64;
    let exp = (bytes as f64).log(div).floor() as i32;
    let pre = "KMGTPE".chars().nth((exp - 1) as usize).unwrap_or('?');
    format!("{:.1} {}B", (bytes as f64) / div.powi(exp), pre)
}
