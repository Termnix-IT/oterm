use anyhow::{Context, Result};
use serde::Deserialize;
use tracing::warn;

use super::secrets_path;

/// Credentials kept out of `config.toml` so the main config stays shareable.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Secrets {
    #[serde(default)]
    pub anthropic_api_key: Option<String>,
}

impl Secrets {
    /// A broken secrets file must not discard the rest of the config, so a
    /// failure here degrades to "no stored credentials" instead of an error.
    pub fn load() -> Self {
        match Self::try_load() {
            Ok(secrets) => secrets,
            Err(err) => {
                warn!(error = %err, "secrets load failed; continuing without stored credentials");
                Self::default()
            }
        }
    }

    fn try_load() -> Result<Self> {
        let Some(path) = secrets_path() else {
            return Ok(Self::default());
        };
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("invalid TOML in {}", path.display()))
    }
}
