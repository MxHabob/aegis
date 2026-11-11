use inquire::{
    Confirm, Select, Text,
    ui::{Color, RenderConfig, StyleSheet, Styled},
};

pub fn set_theme() {
    let mut config = RenderConfig::default_colored();
    config.prompt_prefix = Styled::new("› ").with_fg(Color::LightCyan);
    config.answered_prompt_prefix = Styled::new("✔ ").with_fg(Color::LightGreen);
    config.highlighted_option_prefix = Styled::new("  ▸ ").with_fg(Color::LightMagenta);
    config.scroll_up_prefix = Styled::new("▲ ");
    config.scroll_down_prefix = Styled::new("▼ ");
    config.help_message = StyleSheet::new().with_fg(Color::DarkGrey);
    config.placeholder = StyleSheet::new().with_fg(Color::DarkGrey);
    config.default_value = StyleSheet::new().with_fg(Color::DarkGrey);
    inquire::set_global_render_config(config);
}

pub fn prompt_select(
    message: &str,
    options: Vec<String>,
) -> std::result::Result<String, inquire::InquireError> {
    set_theme();
    Select::new(message, options)
        .with_help_message("Use ↑/↓ to navigate • Enter to select • Ctrl+C to cancel")
        .prompt()
}

pub fn prompt_text(
    message: &str,
    default: Option<&str>,
) -> std::result::Result<String, inquire::InquireError> {
    set_theme();
    let prompt = Text::new(message);
    match default {
        Some(value) => prompt.with_default(value).prompt(),
        None => prompt.prompt(),
    }
}

pub fn prompt_confirm(
    message: &str,
    default: bool,
) -> std::result::Result<bool, inquire::InquireError> {
    set_theme();
    Confirm::new(message).with_default(default).prompt()
}
