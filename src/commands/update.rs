use crate::ui;
use crate::package_managers::get_available_managers;
use anyhow::Result;

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

    for manager in &managers {
        ui::section(&format!("Updating via {}", manager.display_name()));
        match manager.update(yes) {
            Ok(()) => ui::success(&format!("{} updated successfully", manager.display_name())),
            Err(e) => ui::fail(&format!("{} failed: {}", manager.display_name(), e)),
        }
    }

    println!();
    ui::success("Update complete.");
    Ok(())
}
