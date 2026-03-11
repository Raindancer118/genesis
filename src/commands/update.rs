use crate::ui;
use crate::package_managers::get_available_managers;
use anyhow::Result;
use colored::Colorize;

fn print_pkg_row(name: &str, old_ver: &str, new_ver: &str, done: bool) {
    let bullet = if done {
        "✓".truecolor(74, 222, 128).to_string()
    } else {
        "·".truecolor(71, 85, 105).to_string()
    };
    let name_col = if done {
        name.truecolor(74, 222, 128).to_string()
    } else {
        name.truecolor(224, 242, 254).to_string()
    };
    println!(
        "    {} {:<30} {}  →  {}",
        bullet,
        name_col,
        old_ver.truecolor(71, 85, 105),
        new_ver.truecolor(96, 165, 250),
    );
}

pub fn run(yes: bool) -> Result<()> {
    ui::print_header("SYSTEM UPDATE");

    let managers = get_available_managers();

    if managers.is_empty() {
        ui::fail("No package managers found.");
        return Ok(());
    }

    ui::section("Available Package Managers");
    for m in &managers {
        ui::skip(&format!("{}", m.display_name()));
    }
    println!();

    let mut any_updated = false;

    for manager in &managers {
        ui::section(&format!("Updating via {}", manager.display_name()));

        let pending = manager.list_updates();

        if !pending.is_empty() {
            println!(
                "  {}\n",
                format!("{} package{} queued:", pending.len(), if pending.len() == 1 { "" } else { "s" })
                    .truecolor(147, 197, 253)
            );
            for (name, old_ver, new_ver) in &pending {
                print_pkg_row(name, old_ver, new_ver, false);
            }
            println!();
        }

        match manager.update(yes) {
            Ok(()) => {
                if pending.is_empty() {
                    ui::success(&format!("{} — up to date", manager.display_name()));
                } else {
                    // Reprint package list with checkmarks
                    for (name, old_ver, new_ver) in &pending {
                        print_pkg_row(name, old_ver, new_ver, true);
                    }
                    println!();
                    ui::success(&format!(
                        "{} — {} package{} updated",
                        manager.display_name(),
                        pending.len(),
                        if pending.len() == 1 { "" } else { "s" }
                    ));
                    any_updated = true;
                }
            }
            Err(e) => ui::fail(&format!("{} failed: {}", manager.display_name(), e)),
        }
        println!();
    }

    if any_updated {
        ui::success("All updates applied.");
    } else {
        ui::success("Everything is up to date.");
    }
    Ok(())
}
