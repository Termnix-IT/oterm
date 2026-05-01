use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use ratatui::style::Color;
use serde::Deserialize;

const DEFAULT_CONFIG: &str = include_str!("default.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub default_profile: String,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub ai: AiConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_ai_model")]
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_ai_system_prompt")]
    pub system_prompt: String,
    #[serde(default = "default_ai_max_tokens")]
    pub max_tokens: u32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model: default_ai_model(),
            api_key: None,
            system_prompt: default_ai_system_prompt(),
            max_tokens: default_ai_max_tokens(),
        }
    }
}

impl AiConfig {
    pub fn effective_api_key(&self) -> Option<String> {
        if let Some(key) = self.api_key.as_ref().filter(|s| !s.is_empty()) {
            return Some(key.clone());
        }
        std::env::var("ANTHROPIC_API_KEY").ok().filter(|s| !s.is_empty())
    }
}

fn default_ai_model() -> String {
    "claude-haiku-4-5-20251001".into()
}

fn default_ai_system_prompt() -> String {
    "You translate the user's natural-language request into ONE shell command for the user's current OS. Output ONLY the command on a single line: no markdown, no code fences, no quotes around the whole thing, no explanation, no leading prompt or shell name. Prefer the most idiomatic, safest command. If multiple commands are required, join them with `&&` (POSIX shells) or `;` (PowerShell).".into()
}

fn default_ai_max_tokens() -> u32 {
    256
}

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProfileType {
    #[default]
    Local,
    Ssh,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(default, rename = "type")]
    pub kind: ProfileType,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

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

impl Config {
    pub fn load() -> Result<Self> {
        if let Some(path) = config_path() {
            if path.exists() {
                let text = std::fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {}", path.display()))?;
                return toml::from_str(&text)
                    .with_context(|| format!("invalid TOML in {}", path.display()));
            }
        }
        Self::defaults()
    }

    pub fn defaults() -> Result<Self> {
        toml::from_str(DEFAULT_CONFIG).context("invalid default config")
    }

    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    pub fn pick_default_profile(&self) -> Option<&Profile> {
        if let Some(p) = self.profile(&self.default_profile) {
            return Some(p);
        }
        let platform = if cfg!(windows) { "powershell" } else { "bash" };
        self.profile(platform).or_else(|| self.profiles.first())
    }
}

pub fn config_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "oterm").map(|d| d.config_dir().join("config.toml"))
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
    value
        .as_deref()
        .and_then(parse_color)
        .unwrap_or(fallback)
}
