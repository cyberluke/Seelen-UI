//! V271 + Mastodon social fabric adapters (ADR/08).
//!
//! Typed REST adapters over the public Mastodon API and the v271.cz agentic
//! endpoints. All results are normalized into the same typed objects consumed
//! by the capability router (UI, CLI, MCP, REST share these functions).

use std::sync::LazyLock;

use serde_json::{Value, json};

static HTTP: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

const MASTODON_BASE: &str = "https://mastodon.social/api/v1";
const V271_BASE: &str = "https://v271.cz/api";

async fn get_json(url: &str) -> Result<Value, String> {
    HTTP.get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

/// Mastodon objects projected into the NAI graph shape.
fn project_status(status: &Value) -> Value {
    json!({
        "id": status.get("id").cloned().unwrap_or(Value::Null),
        "kind": "MastodonStatus",
        "title": status.get("content").and_then(Value::as_str).unwrap_or(""),
        "createdAt": status.get("created_at").cloned().unwrap_or(Value::Null),
        "account": status.get("account").and_then(|a| a.get("acct")).cloned().unwrap_or(Value::Null),
    })
}

/// Home timeline (bounded).
pub async fn timeline_home(limit: usize) -> Result<Value, String> {
    let n = limit.clamp(1, 40);
    let raw = get_json(&format!("{MASTODON_BASE}/timelines/home?limit={n}")).await?;
    Ok(Value::Array(
        raw.as_array()
            .map(|list| list.iter().map(project_status).collect())
            .unwrap_or_default(),
    ))
}

/// Local timeline (bounded).
pub async fn timeline_local(limit: usize) -> Result<Value, String> {
    let n = limit.clamp(1, 40);
    let raw = get_json(&format!("{MASTODON_BASE}/timelines/public?limit={n}")).await?;
    Ok(Value::Array(
        raw.as_array()
            .map(|list| list.iter().map(project_status).collect())
            .unwrap_or_default(),
    ))
}

/// Notifications projection.
pub async fn notifications(limit: usize) -> Result<Value, String> {
    let n = limit.clamp(1, 40);
    let raw = get_json(&format!("{MASTODON_BASE}/notifications?limit={n}")).await?;
    Ok(Value::Array(
        raw.as_array()
            .map(|list| {
                list.iter()
                    .map(|item| {
                        json!({
                            "id": item.get("id").cloned().unwrap_or(Value::Null),
                            "kind": "MastodonNotification",
                            "type": item.get("type").cloned().unwrap_or(Value::Null),
                            "from": item.get("account").and_then(|a| a.get("acct")).cloned().unwrap_or(Value::Null),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
    ))
}

/// Compose a status; returns the created id + visibility provenance.
pub async fn compose(status: &str) -> Result<Value, String> {
    let body = json!({ "status": status, "visibility": "public" });
    let res = HTTP
        .post(format!("{MASTODON_BASE}/statuses"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let value: Value = res.json().await.map_err(|e| e.to_string())?;
    Ok(json!({
        "id": value.get("id").cloned().unwrap_or(Value::Null),
        "visible": true,
    }))
}

/// v271 agentic chat: create/continue a conversation with typed context.
pub async fn v271_chat(prompt: &str) -> Result<Value, String> {
    let body = json!({ "prompt": prompt });
    let res = HTTP
        .post(format!("{V271_BASE}/chat"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let value: Value = res.json().await.map_err(|e| e.to_string())?;
    Ok(json!({
        "kind": "V271Conversation",
        "answer": value.get("answer").cloned().unwrap_or(value),
    }))
}
