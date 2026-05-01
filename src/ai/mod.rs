use std::sync::mpsc;
use std::thread;

use anyhow::{Context, Result};
use serde_json::json;
use tracing::{info, warn};

use crate::config::AiConfig;

#[derive(Debug)]
pub enum AiOutcome {
    Ok(String),
    Err(String),
}

pub struct AiClient {
    api_key: String,
    model: String,
    system_prompt: String,
    max_tokens: u32,
}

impl AiClient {
    pub fn from_config(cfg: &AiConfig) -> Option<Self> {
        if !cfg.enabled {
            info!("ai disabled in config");
            return None;
        }
        let api_key = cfg.effective_api_key()?;
        Some(Self {
            api_key,
            model: cfg.model.clone(),
            system_prompt: cfg.system_prompt.clone(),
            max_tokens: cfg.max_tokens,
        })
    }

    pub fn suggest_async(&self, prompt: String) -> mpsc::Receiver<AiOutcome> {
        let (tx, rx) = mpsc::channel();
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let system = self.system_prompt.clone();
        let max_tokens = self.max_tokens;
        thread::spawn(move || {
            let outcome = match call_anthropic(&api_key, &model, &system, max_tokens, &prompt) {
                Ok(text) => AiOutcome::Ok(text),
                Err(e) => {
                    warn!(error = %e, "anthropic call failed");
                    AiOutcome::Err(format!("{e:#}"))
                }
            };
            let _ = tx.send(outcome);
        });
        rx
    }
}

fn call_anthropic(
    api_key: &str,
    model: &str,
    system: &str,
    max_tokens: u32,
    user_prompt: &str,
) -> Result<String> {
    let body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": system,
        "messages": [{"role": "user", "content": user_prompt}],
    });
    let resp = ureq::post("https://api.anthropic.com/v1/messages")
        .set("x-api-key", api_key)
        .set("anthropic-version", "2023-06-01")
        .set("content-type", "application/json")
        .send_json(body)
        .context("anthropic api request failed")?;
    let v: serde_json::Value = resp.into_json().context("anthropic response is not json")?;
    let text = v
        .get("content")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .context("anthropic response missing content[0].text")?;
    Ok(sanitize(text))
}

fn sanitize(text: &str) -> String {
    let trimmed = text.trim();
    // Strip a single ``` code-fence wrapper if present.
    if let Some(rest) = trimmed.strip_prefix("```") {
        let body = rest
            .splitn(2, '\n')
            .nth(1)
            .unwrap_or(rest)
            .trim_end_matches("```")
            .trim();
        return body.to_string();
    }
    trimmed.to_string()
}
