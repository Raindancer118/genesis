use crate::ui;
use colored::Colorize;
use chrono::{Local, Timelike};

pub fn run() {
    ui::print_header("WELCOME");

    let now = Local::now();
    let hour = now.hour();
    let user = whoami::username();

    let greeting = if hour < 12 {
        "Good Morning"
    } else if hour < 18 {
        "Good Afternoon"
    } else {
        "Good Evening"
    };

    println!("  {}, {}!",
        greeting.truecolor(96, 165, 250).bold(),
        user.truecolor(224, 242, 254).bold()
    );
    println!("  {}", now.format("%A, %B %-d · %H:%M").to_string().truecolor(71, 85, 105));
    println!();
    ui::divider();
    println!();
}
