use crate::ui;
use crate::package_managers::get_available_managers;
use anyhow::Result;
use colored::Colorize;

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

        // Show pending updates if the PM supports it
        let pending = manager.list_updates();
        if !pending.is_empty() {
            println!(
                "  {}",
                format!("{} package{} to update:", pending.len(), if pending.len() == 1 { "" } else { "s" })
                    .truecolor(147, 197, 253)
            );
            println!();
            for (name, old_ver, new_ver) in &pending {
                println!(
                    "    {:<28} {}  →  {}",
                    name.truecolor(224, 242, 254),
                    old_ver.truecolor(71, 85, 105),
                    new_ver.truecolor(74, 222, 128),
                );
            }
            println!();
        }

        match manager.update(yes) {
            Ok(()) => {
                if pending.is_empty() {
                    ui::success(&format!("{} — up to date", manager.display_name()));
                } else {
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
