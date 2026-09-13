use anyhow::{bail, Context, Result};
use serde_json::{json, Value};

use super::sanitize;

pub(super) fn call(
    base_url: &str,
    model: &str,
    system: &str,
    max_tokens: u32,
    user_prompt: &str,
) -> Result<String> {
    let url = format!("{base_url}/api/chat");
    // Thinking models otherwise spend the whole num_predict budget on reasoning and answer empty.
    let mut body = json!({
        "model": model,
        "stream": false,
        "think": false,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_prompt},
        ],
        "options": {"num_predict": max_tokens},
    });
    let mut retried_without_think = false;
    let resp = loop {
        match ureq::post(&url)
            .set("content-type", "application/json")
            .send_json(&body)
        {
            Ok(resp) => break resp,
            Err(ureq::Error::Status(code, resp)) => {
                let detail = error_detail(resp, code);
                if code == 400 && !retried_without_think && detail.contains("think") {
                    retried_without_think = true;
                    if let Some(fields) = body.as_object_mut() {
                        fields.remove("think");
                    }
                    continue;
                }
                // A wrong model name in the config is the common failure; ollama explains it here.
                bail!("ollama returned {code}: {detail}");
            }
            Err(e) => bail!("cannot reach ollama at {base_url} ({e}); is `ollama serve` running?"),
        }
    };
    let v: Value = resp.into_json().context("ollama response is not json")?;
    let content = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .context("ollama response missing message.content")?;
    let text = sanitize(content);
    if text.is_empty() {
        let reason = v
            .get("done_reason")
            .and_then(|r| r.as_str())
            .unwrap_or("unknown");
        bail!("ollama returned an empty answer (done_reason: {reason})");
    }
    Ok(text)
}

fn error_detail(resp: ureq::Response, code: u16) -> String {
    resp.into_json::<Value>()
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_string))
        .unwrap_or_else(|| format!("http {code}"))
}
