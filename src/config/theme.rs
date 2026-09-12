use ratatui::style::Color;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Theme {
    #[allow(dead_code)]
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub status_fg: Option<String>,
    #[serde(default)]
    pub status_bg: Option<String>,
    #[serde(default)]
    pub tab_active_fg: Option<String>,
    #[serde(default)]
    pub tab_active_bg: Option<String>,
    #[serde(default)]
    pub tab_inactive_fg: Option<String>,
    #[serde(default)]
    pub tab_inactive_bg: Option<String>,
    #[serde(default)]
    pub focus_border: Option<String>,
    #[serde(default)]
    pub inactive_border: Option<String>,
}

pub fn parse_color(value: &str) -> Option<Color> {
    let s = value.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
    }
    match s.to_ascii_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        _ => None,
    }
}

pub fn color_or(value: &Option<String>, fallback: Color) -> Color {
    value.as_deref().and_then(parse_color).unwrap_or(fallback)
}
