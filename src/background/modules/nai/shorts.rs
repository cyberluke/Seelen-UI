//! YouTube / Shorts visual media engine (ADR/06).
//!
//! Shorts-first vertical surface: candidate search through the YouTube Data
//! API, semantic queue with reason/provenance, PiP as a shell object and the
//! typed `media:youtube:<videoId>` graph projection. A short duration is only
//! a *candidate* signal — never equated with the native Shorts identity.

use serde_json::{Value, json};
use std::sync::LazyLock;

static HTTP: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("reqwest client")
});

/// Bounded vertical queue: (videoId, reason). Insertion order = play order.
static QUEUE: LazyLock<std::sync::Mutex<Vec<(String, String)>>> =
    LazyLock::new(|| std::sync::Mutex::new(Vec::new()));

const QUEUE_CAPACITY: usize = 50;

fn yt_key() -> Option<String> {
    std::env::var("NAI_YOUTUBE_KEY")
        .ok()
        .filter(|k| !k.is_empty())
}

fn push_queue(video_id: String, reason: String) {
    let mut queue = QUEUE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(slot) = queue.iter_mut().find(|(id, _)| *id == video_id) {
        slot.1 = reason;
        return;
    }
    if queue.len() == QUEUE_CAPACITY {
        queue.remove(0);
    }
    queue.push((video_id, reason));
}

/// Candidate pipeline step 1-2: official search; `videoDuration=short` marks
/// candidates only (<4 min), the visual classifier step refines them.
pub async fn search(query: &str, limit: usize) -> Result<Value, String> {
    let key = yt_key().ok_or("NAI_YOUTUBE_KEY not configured")?;
    let n = limit.clamp(1, 25).to_string();
    let url = format!(
        "https://www.googleapis.com/youtube/v3/search?part=snippet&type=video&videoDuration=short&maxResults={n}&q={query}&key={key}"
    );
    let raw: Value = HTTP
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    let items: Vec<Value> = raw
        .get("items")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|item| {
                    let id = item
                        .get("id")
                        .and_then(|i| i.get("videoId"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let title = item
                        .get("snippet")
                        .and_then(|s| s.get("title"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    json!({
                        "id": format!("media:youtube:{id}"),
                        "kind": "MediaItem",
                        "videoId": id,
                        "title": title,
                        "candidate": true,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(Value::Array(items))
}

/// Enqueue a search candidate with its queue reason (provenance).
pub fn enqueue(video_id: &str, reason: &str) -> Value {
    push_queue(video_id.to_string(), reason.to_string());
    queue_state()
}

/// Dequeue next (FIFO).
pub fn next() -> Value {
    let mut queue = QUEUE.lock().unwrap_or_else(|e| e.into_inner());
    match queue.is_empty() {
        true => json!({ "next": null }),
        false => {
            let (id, reason) = queue.remove(0);
            json!({ "next": { "videoId": id, "reason": reason } })
        }
    }
}

pub fn queue_state() -> Value {
    let queue = QUEUE.lock().unwrap_or_else(|e| e.into_inner());
    json!({
        "capacity": QUEUE_CAPACITY,
        "items": queue
            .iter()
            .map(|(id, reason)| json!({ "videoId": id, "reason": reason }))
            .collect::<Vec<_>>(),
    })
}

/// PiP shell-object contract (ADR/06 §PiP).
pub fn pip_contract() -> Value {
    json!({
        "schema": "nai.pip/v1",
        "state": [
            "media source", "video id", "playback position", "channel/creator",
            "queue", "activity", "geometry", "monitor", "always-on-top"
        ],
        "magnetZones": ["top-left", "top-right", "bottom-left", "bottom-right", "vertical-rail"],
        "followActivity": true
    })
}
