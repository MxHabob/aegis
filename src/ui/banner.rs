use assetctl::AppState;
use colored::Colorize;

const CYAN: (u8, u8, u8) = (34, 211, 238);
const BLUE: (u8, u8, u8) = (59, 130, 246);
const PURPLE: (u8, u8, u8) = (129, 140, 248);
const GREEN: (u8, u8, u8) = (52, 211, 153);
const AMBER: (u8, u8, u8) = (251, 191, 36);
const MUTED: (u8, u8, u8) = (148, 163, 184);
const BORDER: (u8, u8, u8) = (51, 65, 85);

pub fn print_banner(state: &AppState, interactive: bool) {
    if interactive {
        print!("\x1B[2J\x1B[H");
    }
    let rule = "  ───────────────────────────────────────────────────────────────";
    println!();
    println!(
        "  {}",
        "╭───────────────────────────────────────────────────────────────╮"
            .truecolor(BORDER.0, BORDER.1, BORDER.2)
    );
    println!(
        "  {}",
        "│                                                               │"
            .truecolor(BORDER.0, BORDER.1, BORDER.2)
    );
    println!(
        "  │   {}  {}",
        gradient("ASSETCTL"),
        "DIGITAL ASSET INVENTORY"
            .truecolor(MUTED.0, MUTED.1, MUTED.2)
            .bold()
    );
    println!(
        "  │   {}",
        "Authorized scope • Local inventory • Read-only checks"
            .truecolor(MUTED.0, MUTED.1, MUTED.2)
    );
    println!(
        "  │   {}  {}  {}",
        "● LOCAL".truecolor(GREEN.0, GREEN.1, GREEN.2).bold(),
        format!("{} scopes", state.scopes.len()).truecolor(CYAN.0, CYAN.1, CYAN.2),
        format!("{} assets", state.assets.len()).truecolor(PURPLE.0, PURPLE.1, PURPLE.2)
    );
    println!(
        "  {}",
        "│                                                               │"
            .truecolor(BORDER.0, BORDER.1, BORDER.2)
    );
    println!(
        "  {}",
        "╰───────────────────────────────────────────────────────────────╯"
            .truecolor(BORDER.0, BORDER.1, BORDER.2)
    );
    println!("{}", rule.truecolor(BORDER.0, BORDER.1, BORDER.2));
    if interactive {
        println!(
            "  {}",
            "↑/↓ navigate  •  Enter select  •  Ctrl+C cancel".truecolor(MUTED.0, MUTED.1, MUTED.2)
        );
    }
    println!(
        "  {}",
        "Only assets explicitly added to an authorized scope are managed."
            .truecolor(AMBER.0, AMBER.1, AMBER.2)
    );
    println!();
}

fn gradient(text: &str) -> String {
    let colors = [CYAN, BLUE, PURPLE];
    let chars = text.chars().collect::<Vec<_>>();
    let denom = chars.len().saturating_sub(1).max(1) as f32;
    chars
        .iter()
        .enumerate()
        .map(|(index, ch)| {
            let ratio = index as f32 / denom;
            let (left, right, amount) = if ratio < 0.5 {
                (colors[0], colors[1], ratio * 2.0)
            } else {
                (colors[1], colors[2], (ratio - 0.5) * 2.0)
            };
            let blend = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * amount) as u8;
            ch.to_string()
                .truecolor(
                    blend(left.0, right.0),
                    blend(left.1, right.1),
                    blend(left.2, right.2),
                )
                .bold()
                .to_string()
        })
        .collect()
}
