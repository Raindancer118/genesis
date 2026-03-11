use crate::ui;
use sysinfo::System;

pub fn run() {
    ui::print_header("SYSTEM INFO");

    let mut sys = System::new_all();
    sys.refresh_all();

    ui::section("Hardware");
    ui::info_line("OS", &System::name().unwrap_or_default());
    ui::info_line("OS Version", &System::os_version().unwrap_or_default());
    ui::info_line("Kernel", &System::kernel_version().unwrap_or_default());
    ui::info_line("Hostname", &System::host_name().unwrap_or_default());
    ui::info_line("Architecture", std::env::consts::ARCH);

    ui::section("CPU");
    let cpus = sys.cpus();
    if let Some(cpu) = cpus.first() {
        ui::info_line("Model", cpu.brand());
        ui::info_line("Cores", &cpus.len().to_string());
        ui::info_line("Freq", &format!("{} MHz", cpu.frequency()));
    }

    ui::section("Memory");
    let total = sys.total_memory() / 1024 / 1024;
    let used = sys.used_memory() / 1024 / 1024;
    ui::info_line("RAM", &format!("{} / {} MB", used, total));
    let swap_total = sys.total_swap() / 1024 / 1024;
    ui::info_line("Swap", &format!("{} MB total", swap_total));

    ui::section("User");
    ui::info_line("Username", &whoami::username());
    ui::info_line("Home", &dirs::home_dir().unwrap_or_default().to_string_lossy());

    println!();
}
