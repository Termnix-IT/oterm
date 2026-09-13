mod anthropic;
mod ollama;

use std::sync::mpsc;
use std::thread;

use anyhow::anyhow;
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
        self.request_async(self.system_prompt.clone(), prompt)
    }

    /// Sends only the query; the history itself is matched locally and never leaves the machine.
    pub fn search_terms_async(&self, query: String, shell: &str) -> mpsc::Receiver<AiOutcome> {
        self.request_async(search_terms_prompt(shell), query)
    }

    fn request_async(&self, system: String, prompt: String) -> mpsc::Receiver<AiOutcome> {
        let (tx, rx) = mpsc::channel();
        let provider = self.provider;
        let model = self.model().to_string();
        let api_key = self.api_key.clone();
        let base_url = self.ollama_base_url.clone();
        let max_tokens = self.max_tokens;
        thread::spawn(move || {
            let result = match provider {
                AiProvider::Anthropic => match api_key {
                    Some(key) => anthropic::call(&key, &model, &system, max_tokens, &prompt),
                    None => Err(anyhow!(
                        "no Anthropic API key: set anthropic_api_key in secrets.toml, \
                         the ANTHROPIC_API_KEY env var, or switch provider with Tab"
                    )),
                },
                AiProvider::Ollama => {
                    if model.is_empty() {
                        Err(anyhow!("no model listed in [ai.ollama].models"))
                    } else {
                        ollama::call(&base_url, &model, &system, max_tokens, &prompt)
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

/// Both providers are told to answer with a bare command, but models still
/// wrap it in a code fence often enough to be worth stripping here.
fn sanitize(text: &str) -> String {
    let trimmed = text.trim();
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

fn search_terms_prompt(shell: &str) -> String {
    format!(
        "You help search a user's {shell} command history. The user describes a command \
         they ran before. Reply with 3 to 8 short search terms that would appear literally \
         in that command line: command names, subcommands, flags, or file names, written \
         in {shell} syntax. Output one term per line with no numbering, no quotes, and no \
         explanation."
    )
}
