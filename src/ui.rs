use colored::*;

// Volantic color palette (R, G, B)
const BLUE_DEEP: (u8, u8, u8) = (37, 99, 235);
const BLUE_MID: (u8, u8, u8) = (59, 130, 246);
const BLUE_LIGHT: (u8, u8, u8) = (96, 165, 250);
const BLUE_PALE: (u8, u8, u8) = (147, 197, 253);
const TEXT_MAIN: (u8, u8, u8) = (224, 242, 254);
const TEXT_DIM: (u8, u8, u8) = (71, 85, 105);

fn rgb(r: u8, g: u8, b: u8, text: &str) -> ColoredString {
    text.truecolor(r, g, b)
}

pub fn print_header(subtitle: &str) {
    println!();
    // Bird ASCII art (blue gradient lines)
    let lines = [
        ("  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", BLUE_DEEP),
        ("  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ ◂", BLUE_MID),
        ("  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", BLUE_LIGHT),
        ("  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━", BLUE_MID),
        ("  ━━━━━━━━━━━━━━━━━━━━━━━━━━━", BLUE_PALE),
        ("    ·  ·  ·", BLUE_PALE),
    ];
    for (line, (r, g, b)) in &lines {
        println!("{}", rgb(*r, *g, *b, line));
    }
    println!();
    // Title
    println!("  {}", gradient_text("V O L A N T I C   G E N E S I S"));
    println!("  {}", rgb(BLUE_MID.0, BLUE_MID.1, BLUE_MID.2, "─────────────────────────────────"));
    println!("  {}", rgb(TEXT_MAIN.0, TEXT_MAIN.1, TEXT_MAIN.2, subtitle).bold());
    println!();
}

pub fn gradient_text(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len().max(1);
    let mut result = String::new();
    for (i, ch) in chars.iter().enumerate() {
        let t = i as f32 / len as f32;
        let r = lerp(BLUE_DEEP.0, BLUE_PALE.0, t);
        let g = lerp(BLUE_DEEP.1, BLUE_PALE.1, t);
        let b = lerp(BLUE_DEEP.2, BLUE_PALE.2, t);
        result.push_str(&format!("{}", ch.to_string().truecolor(r, g, b).bold()));
    }
    result
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t) as u8
}

pub fn section(title: &str) {
    let line = "─".repeat(48);
    println!(
        "\n  {} {} {}",
        rgb(BLUE_DEEP.0, BLUE_DEEP.1, BLUE_DEEP.2, "──"),
        rgb(BLUE_LIGHT.0, BLUE_LIGHT.1, BLUE_LIGHT.2, title).bold(),
        rgb(BLUE_DEEP.0, BLUE_DEEP.1, BLUE_DEEP.2, &line[..line.len().min(44 - title.len().min(44))])
    );
}

pub fn divider() {
    println!(
        "  {}",
        rgb(BLUE_DEEP.0, BLUE_DEEP.1, BLUE_DEEP.2, &"─".repeat(50))
    );
}

pub fn success(msg: &str) {
    println!(
        "  {} {}",
        rgb(96, 165, 250, "✓").bold(),
        rgb(TEXT_MAIN.0, TEXT_MAIN.1, TEXT_MAIN.2, msg)
    );
}

pub fn fail(msg: &str) {
    println!(
        "  {} {}",
        "✗".truecolor(239, 68, 68).bold(),
        msg.truecolor(239, 68, 68)
    );
}

pub fn skip(msg: &str) {
    println!(
        "  {} {}",
        rgb(TEXT_DIM.0, TEXT_DIM.1, TEXT_DIM.2, "·"),
        rgb(TEXT_DIM.0, TEXT_DIM.1, TEXT_DIM.2, msg)
    );
}

pub fn info_line(label: &str, value: &str) {
    println!(
        "  {} {}",
        rgb(BLUE_LIGHT.0, BLUE_LIGHT.1, BLUE_LIGHT.2, &format!("{:<16}", label)),
        rgb(TEXT_MAIN.0, TEXT_MAIN.1, TEXT_MAIN.2, value)
    );
}
