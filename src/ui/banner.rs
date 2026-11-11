use aegis::{AppContext, AppState};
use anyhow::Result;
use colored::Colorize;
use std::time::Duration;

const CYAN: (u8, u8, u8) = (34, 211, 238);
const BLUE: (u8, u8, u8) = (56, 189, 248);
const PURPLE: (u8, u8, u8) = (167, 139, 250);
const GREEN: (u8, u8, u8) = (74, 222, 128);
const MAGENTA: (u8, u8, u8) = (232, 121, 249);
const AMBER: (u8, u8, u8) = (251, 146, 60);
const MUTED: (u8, u8, u8) = (148, 163, 184);
const BORDER: (u8, u8, u8) = (51, 65, 85);

const AEGIS_FONT: [[&str; 5]; 5] = [
    [" ███ ", "█   █", "█████", "█   █", "█   █"],
    ["█████", "█    ", "████ ", "█    ", "█████"],
    [" ████", "█    ", "█ ███", "█   █", " ████"],
    ["█████", "  █  ", "  █  ", "  █  ", "█████"],
    [" ████", "█    ", " ███ ", "    █", "████ "],
];

pub fn print_banner(app: &AppContext, interactive: bool, timeout: Duration) -> Result<()> {
    if interactive {
        print!("\x1B[2J\x1B[H");
    }

    let state = app.state();
    let saved_maps = app.saved_maps()?.len();
    let hosts = count_type(state, "host");
    let cameras = count_type(state, "camera");
    let certificates = count_type(state, "certificate");
    let web_properties = count_type(state, "web_property");
    let groups = state
        .scopes
        .iter()
        .map(|scope| scope.group.as_str())
        .chain(state.assets.iter().map(|asset| asset.group.as_str()))
        .collect::<std::collections::BTreeSet<_>>()
        .len();

    println!();
    println!(
        "  {}",
        "╭────────────────────────────────────────────────────────────────────╮"
            .truecolor(BORDER.0, BORDER.1, BORDER.2)
    );
    for row_index in 0..5 {
        let row = AEGIS_FONT
            .iter()
            .map(|glyph| glyph[row_index])
            .collect::<Vec<_>>()
            .join(" ");
        println!("  │   {}", gradient(&row));
    }
    println!(
        "  │   {}",
        "CYBERSECURITY OPERATIONS  •  DEFENSE  •  RESPONSE"
            .truecolor(CYAN.0, CYAN.1, CYAN.2)
            .bold()
    );
    println!(
        "  │   {}  {}  {}  {}",
        format!("{} scopes", state.scopes.len()).truecolor(CYAN.0, CYAN.1, CYAN.2),
        format!("{} assets", state.assets.len()).truecolor(GREEN.0, GREEN.1, GREEN.2),
        format!("{hosts} hosts").truecolor(BLUE.0, BLUE.1, BLUE.2),
        format!("{cameras} IoT").truecolor(PURPLE.0, PURPLE.1, PURPLE.2)
    );
    println!(
        "  │   {}",
        format!(
            "ATTACK SURFACE  •  {groups} groups  •  {certificates} certs  •  {web_properties} web"
        )
        .truecolor(MUTED.0, MUTED.1, MUTED.2)
    );
    println!(
        "  │   {}",
        format!(
            "{saved_maps} saved maps  •  HTTPS/TLS  •  timeout: {}s",
            timeout.as_secs()
        )
        .truecolor(MUTED.0, MUTED.1, MUTED.2)
    );
    println!(
        "  │   {}",
        "ROADMAP: APPSEC  •  CLOUD  •  IR  •  FORENSICS  •  DETECTION"
            .truecolor(MAGENTA.0, MAGENTA.1, MAGENTA.2)
            .bold()
    );
    println!(
        "  │   {}",
        "OS routing  •  VPN route unverified  •  Tor disabled".truecolor(MUTED.0, MUTED.1, MUTED.2)
    );
    println!(
        "  {}",
        "╰────────────────────────────────────────────────────────────────────╯"
            .truecolor(BORDER.0, BORDER.1, BORDER.2)
    );
    if interactive {
        println!(
            "  {}",
            "Use ↑/↓ and Enter to navigate • Ctrl+C to cancel".truecolor(MUTED.0, MUTED.1, MUTED.2)
        );
    }
    println!(
        "  {}",
        "Scoped operations  •  Local evidence  •  No unverified routing claims"
            .truecolor(AMBER.0, AMBER.1, AMBER.2)
    );
    println!();
    Ok(())
}

fn count_type(state: &AppState, asset_type: &str) -> usize {
    state
        .assets
        .iter()
        .filter(|asset| asset.asset_type == asset_type)
        .count()
}

fn gradient(text: &str) -> String {
    let colors = [CYAN, BLUE, PURPLE, MAGENTA, GREEN];
    let chars = text.chars().collect::<Vec<_>>();
    let denom = chars.len().saturating_sub(1).max(1) as f32;
    chars
        .iter()
        .enumerate()
        .map(|(index, ch)| {
            let ratio = index as f32 / denom;
            let scaled = ratio * (colors.len() - 1) as f32;
            let segment = (scaled.floor() as usize).min(colors.len() - 2);
            let amount = scaled - segment as f32;
            let (left, right) = (colors[segment], colors[segment + 1]);
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
