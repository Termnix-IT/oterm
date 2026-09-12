use anyhow::{bail, Context, Result};
use serde_json::json;

use super::sanitize;

pub(super) fn call(
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
