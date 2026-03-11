use crate::ui;
use crate::package_managers::{get_available_managers, PmPackage};
use anyhow::Result;
use rayon::prelude::*;
use comfy_table::{Table, Cell, Color, Attribute};
use inquire::Select;

pub fn install(pkg: &str, yes: bool) -> Result<()> {
    ui::print_header(&format!("INSTALL  {}", pkg));

    let managers = get_available_managers();
    if managers.is_empty() {
        ui::fail("No package managers available.");
        return Ok(());
    }

    ui::section("Searching all package managers");

    // Parallel search across all PMs
    let results: Vec<(String, Vec<PmPackage>)> = managers
        .par_iter()
        .filter_map(|m| {
            match m.search(pkg) {
                Ok(pkgs) if !pkgs.is_empty() => Some((m.id().to_string(), pkgs)),
                _ => None,
            }
        })
        .collect();

    if results.is_empty() {
        ui::fail(&format!("No results found for '{}'", pkg));
        return Ok(());
    }

    // Flatten and display results
    let mut all: Vec<(String, PmPackage)> = Vec::new();
    for (pm_id, pkgs) in &results {
        for p in pkgs.iter().take(5) {
            all.push((pm_id.clone(), p.clone()));
        }
    }

    // Show table
    let mut table = Table::new();
    table.set_header(vec![
        Cell::new("#").add_attribute(Attribute::Bold),
        Cell::new("Package").add_attribute(Attribute::Bold),
        Cell::new("Version").add_attribute(Attribute::Bold),
        Cell::new("Source").add_attribute(Attribute::Bold),
        Cell::new("Description").add_attribute(Attribute::Bold),
    ]);

    for (i, (pm_id, p)) in all.iter().enumerate() {
        table.add_row(vec![
            Cell::new(i + 1),
            Cell::new(&p.name).fg(Color::Blue),
            Cell::new(p.version.as_deref().unwrap_or("-")),
            Cell::new(pm_id).fg(Color::Cyan),
            Cell::new(p.description.as_deref().unwrap_or("").chars().take(40).collect::<String>()),
        ]);
    }
    println!("{}", table);
    println!();

    // Interactive selection
    let options: Vec<String> = all.iter().enumerate()
        .map(|(_, (pm_id, p))| format!("[{}] {} ({})", pm_id, p.name, p.version.as_deref().unwrap_or("?")))
        .collect();

    if options.is_empty() {
        ui::fail("No packages to select.");
        return Ok(());
    }

    let selection = Select::new("Select package to install:", options.clone()).prompt()?;
    let idx = options.iter().position(|o| o == &selection).unwrap_or(0);

    let (pm_id, selected_pkg) = &all[idx];

    // Find the right manager
    let managers2 = get_available_managers();
    let manager = managers2.iter().find(|m| m.id() == pm_id.as_str());

    if let Some(m) = manager {
        ui::section(&format!("Installing via {}", m.display_name()));
        m.install(&selected_pkg.name, yes)?;
        ui::success(&format!("'{}' installed successfully.", selected_pkg.name));
    } else {
        ui::fail("Package manager not found.");
    }

    Ok(())
}

pub fn uninstall(pkg: &str) -> Result<()> {
    ui::print_header(&format!("UNINSTALL  {}", pkg));

    let managers = get_available_managers();

    ui::section("Removing package");

    let mut removed = false;
    for m in &managers {
        match m.uninstall(pkg) {
            Ok(()) => {
                ui::success(&format!("Removed '{}' via {}", pkg, m.display_name()));
                removed = true;
                break;
            }
            Err(_) => {
                ui::skip(&format!("{}: not found or failed", m.display_name()));
            }
        }
    }

    if !removed {
        ui::fail(&format!("Could not uninstall '{}' from any package manager.", pkg));
    }

    Ok(())
}
