use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    #[default]
    Anthropic,
    Ollama,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub provider: AiProvider,
    #[serde(default = "default_ai_model")]
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_ai_system_prompt")]
    pub system_prompt: String,
    #[serde(default = "default_ai_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub ollama: OllamaConfig,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: AiProvider::default(),
            model: default_ai_model(),
            api_key: None,
            system_prompt: default_ai_system_prompt(),
            max_tokens: default_ai_max_tokens(),
            ollama: OllamaConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OllamaConfig {
    #[serde(default = "default_ollama_base_url")]
    pub base_url: String,
    #[serde(default = "default_ollama_models")]
    pub models: Vec<String>,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: default_ollama_base_url(),
            models: default_ollama_models(),
        }
    }
}

fn default_ai_model() -> String {
    "claude-haiku-4-5-20251001".into()
}

fn default_ollama_base_url() -> String {
    "http://localhost:11434".into()
}

fn default_ollama_models() -> Vec<String> {
    vec!["qwen2.5-coder".into()]
}

fn default_ai_system_prompt() -> String {
    "You translate the user's natural-language request into ONE shell command for the user's current OS. Output ONLY the command on a single line: no markdown, no code fences, no quotes around the whole thing, no explanation, no leading prompt or shell name. Prefer the most idiomatic, safest command. If multiple commands are required, join them with `&&` (POSIX shells) or `;` (PowerShell).".into()
}

fn default_ai_max_tokens() -> u32 {
    256
}
