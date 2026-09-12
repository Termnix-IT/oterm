pub mod ai;
pub mod profile;
pub mod secrets;
pub mod theme;

use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::Deserialize;

pub use ai::{AiConfig, AiProvider};
pub use profile::{Profile, ProfileType};
pub use theme::{color_or, Theme};

use secrets::Secrets;

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
    #[serde(skip)]
    pub secrets: Secrets,
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut config = Self::load_toml()?;
        config.secrets = Secrets::load();
        Ok(config)
    }

    pub fn defaults() -> Result<Self> {
        let mut config: Self = toml::from_str(DEFAULT_CONFIG).context("invalid default config")?;
        config.secrets = Secrets::load();
        Ok(config)
    }

    /// Anthropic key lookup: dedicated secrets file, then env var, then the
    /// legacy plaintext `[ai].api_key` in the shared config.
    pub fn anthropic_api_key(&self) -> Option<String> {
        self.secrets
            .anthropic_api_key
            .clone()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                std::env::var("ANTHROPIC_API_KEY")
                    .ok()
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| self.ai.api_key.clone().filter(|s| !s.is_empty()))
    }

    fn load_toml() -> Result<Self> {
        if let Some(path) = config_path() {
            if path.exists() {
                let text = std::fs::read_to_string(&path)
                    .with_context(|| format!("failed to read {}", path.display()))?;
                return toml::from_str(&text)
                    .with_context(|| format!("invalid TOML in {}", path.display()));
            }
        }
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

pub fn secrets_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "oterm").map(|d| d.config_dir().join("secrets.toml"))
}
