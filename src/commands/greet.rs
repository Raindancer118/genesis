use colored::Colorize;
use chrono::Local;

pub fn run() {
    let now = Local::now();
    let hour = now.hour();
    let user = whoami::username(); // or realname

    let greeting = if hour < 12 {
        "Good Morning"
    } else if hour < 18 {
        "Good Afternoon"
    } else {
        "Good Evening"
    };

    println!("{}, {}!", greeting.bold().cyan(), user.bold().yellow());
}
