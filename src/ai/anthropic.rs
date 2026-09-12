use anyhow::{Context, Result};
use serde_json::json;

use super::sanitize;

pub(super) fn call(
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
