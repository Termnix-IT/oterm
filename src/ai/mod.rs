use std::sync::mpsc;
use std::thread;

use anyhow::{anyhow, bail, Context, Result};
use serde_json::json;
use tracing::{info, warn};

use crate::config::{AiConfig, AiProvider};

#[derive(Debug)]
pub enum AiOutcome {
    Ok(String),
    Err(String),
}

pub struct AiClient {
    provider: AiProvider,
    api_key: Option<String>,
    anthropic_model: String,
    ollama_base_url: String,
    ollama_models: Vec<String>,
    ollama_model_index: usize,
    system_prompt: String,
    max_tokens: u32,
}

impl AiClient {
    pub fn from_config(cfg: &AiConfig, api_key: Option<String>) -> Option<Self> {
        if !cfg.enabled {
            info!("ai disabled in config");
            return None;
        }
        info!(
            provider = ?cfg.provider,
            anthropic_key = api_key.is_some(),
            ollama_models = cfg.ollama.models.len(),
            "ai client ready"
        );
        Some(Self {
            provider: cfg.provider,
            api_key,
            anthropic_model: cfg.model.clone(),
            ollama_base_url: cfg.ollama.base_url.trim_end_matches('/').to_string(),
            ollama_models: cfg.ollama.models.clone(),
            ollama_model_index: 0,
            system_prompt: cfg.system_prompt.clone(),
            max_tokens: cfg.max_tokens,
        })
    }

    pub fn model(&self) -> &str {
        match self.provider {
            AiProvider::Anthropic => &self.anthropic_model,
            AiProvider::Ollama => self
                .ollama_models
                .get(self.ollama_model_index)
                .map(String::as_str)
                .unwrap_or(""),
        }
    }

    /// Label for the modal header, e.g. `ollama · qwen2.5-coder`.
    pub fn status_label(&self) -> String {
        let (name, missing) = match self.provider {
            AiProvider::Anthropic => ("anthropic", self.api_key.is_none()),
            AiProvider::Ollama => ("ollama", false),
        };
        let model = match self.model() {
            "" => "(no model configured)",
            m => m,
        };
        if missing {
            format!("{name} · {model} · no api key")
        } else {
            format!("{name} · {model}")
        }
    }

    pub fn cycle_provider(&mut self) {
        self.provider = match self.provider {
            AiProvider::Anthropic => AiProvider::Ollama,
            AiProvider::Ollama => AiProvider::Anthropic,
        };
    }

    pub fn cycle_model(&mut self) {
        if self.provider != AiProvider::Ollama || self.ollama_models.is_empty() {
            return;
        }
        self.ollama_model_index = (self.ollama_model_index + 1) % self.ollama_models.len();
    }

    pub fn suggest_async(&self, prompt: String) -> mpsc::Receiver<AiOutcome> {
        let (tx, rx) = mpsc::channel();
        let provider = self.provider;
        let model = self.model().to_string();
        let api_key = self.api_key.clone();
        let base_url = self.ollama_base_url.clone();
        let system = self.system_prompt.clone();
        let max_tokens = self.max_tokens;
        thread::spawn(move || {
            let result = match provider {
                AiProvider::Anthropic => match api_key {
                    Some(key) => call_anthropic(&key, &model, &system, max_tokens, &prompt),
                    None => Err(anyhow!(
                        "no Anthropic API key: set anthropic_api_key in secrets.toml, \
                         the ANTHROPIC_API_KEY env var, or switch provider with Tab"
                    )),
                },
                AiProvider::Ollama => {
                    if model.is_empty() {
                        Err(anyhow!("no model listed in [ai.ollama].models"))
                    } else {
                        call_ollama(&base_url, &model, &system, max_tokens, &prompt)
                    }
                }
            };
            let outcome = match result {
                Ok(text) => AiOutcome::Ok(text),
                Err(e) => {
                    warn!(error = %e, ?provider, "ai call failed");
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

fn call_ollama(
    base_url: &str,
    model: &str,
    system: &str,
    max_tokens: u32,
    user_prompt: &str,
) -> Result<String> {
    let body = json!({
        "model": model,
        "stream": false,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_prompt},
        ],
        "options": {"num_predict": max_tokens},
    });
    let url = format!("{base_url}/api/chat");
    let resp = match ureq::post(&url)
        .set("content-type", "application/json")
        .send_json(body)
    {
        Ok(resp) => resp,
        // A wrong model name in the config is the common failure here, and
        // ollama reports it in the body, so surface that instead of the status.
        Err(ureq::Error::Status(code, resp)) => {
            let detail = resp
                .into_json::<serde_json::Value>()
                .ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_string))
                .unwrap_or_else(|| format!("http {code}"));
            bail!("ollama returned {code}: {detail}");
        }
        Err(e) => bail!("cannot reach ollama at {base_url} ({e}); is `ollama serve` running?"),
    };
    let v: serde_json::Value = resp.into_json().context("ollama response is not json")?;
    let text = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .context("ollama response missing message.content")?;
    Ok(sanitize(text))
}

fn sanitize(text: &str) -> String {
    let trimmed = text.trim();
    // Strip a single ``` code-fence wrapper if present.
    if let Some(rest) = trimmed.strip_prefix("```") {
        let body = rest
            .split_once('\n')
            .map(|(_, body)| body)
            .unwrap_or(rest)
            .trim_end_matches("```")
            .trim();
        return body.to_string();
    }
    trimmed.to_string()
}
