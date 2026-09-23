//! Memory / Qdrant semantic tier (ADR/02 §3 hybrid model).
//!
//! Durable relational identity stays in the native registries; Qdrant holds
//! semantic vectors. When the Qdrant server is unreachable the adapter falls
//! back to a bounded in-memory cosine scan so local search keeps working
//! offline (local-first dogma).

use std::sync::{LazyLock, Mutex};

use serde_json::{Value, json};

static HTTP: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

/// Bounded offline fallback store: (id, vector).
type Snapshot = Vec<(String, Vec<f32>)>;
static SNAPSHOT: LazyLock<Mutex<Snapshot>> = LazyLock::new(|| Mutex::new(Vec::new()));

const SNAPSHOT_CAPACITY: usize = 2048;
const COLLECTION: &str = "naic";

fn endpoint() -> String {
    std::env::var("NAI_QDRANT_URL").unwrap_or_else(|_| "http://127.0.0.1:6333".to_string())
}

fn push_snapshot(id: &str, vector: Vec<f32>) {
    let mut store = SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(slot) = store.iter_mut().find(|(existing, _)| existing == id) {
        slot.1 = vector;
        return;
    }
    if store.len() == SNAPSHOT_CAPACITY {
        store.remove(0);
    }
    store.push((id.to_string(), vector));
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

fn local_scan(query: &[f32], limit: usize) -> Vec<Value> {
    let store = SNAPSHOT.lock().unwrap_or_else(|e| e.into_inner());
    let mut scored: Vec<Value> = store
        .iter()
        .map(|(id, vector)| json!({ "id": id, "score": cosine(query, vector) }))
        .collect();
    scored.sort_by(|a, b| {
        b.get("score")
            .and_then(Value::as_f64)
            .partial_cmp(&a.get("score").and_then(Value::as_f64))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(limit);
    scored
}

/// Upsert one vector; always mirrored to the bounded offline snapshot.
pub async fn upsert(id: &str, vector: &[f32]) -> Value {
    push_snapshot(id, vector.to_vec());
    let body = json!({ "points": [{ "id": id, "vector": vector }] });
    match HTTP
        .post(format!("{}/collections/{COLLECTION}/points", endpoint()))
        .json(&body)
        .send()
        .await
    {
        Ok(res) => res
            .json::<Value>()
            .await
            .unwrap_or_else(|e| json!({ "cached": true, "note": e.to_string() })),
        Err(err) => json!({ "cached": true, "server": "unreachable", "note": err.to_string() }),
    }
}

/// Search by vector: Qdrant first, bounded cosine scan offline.
pub async fn search(query: &[f32], limit: usize) -> Value {
    let body = json!({ "vector": query, "limit": limit, "with_payload": true });
    let result = match HTTP
        .post(format!(
            "{}/collections/{COLLECTION}/points/search",
            endpoint()
        ))
        .json(&body)
        .send()
        .await
    {
        Ok(res) => res.json::<Value>().await.ok(),
        Err(_) => None,
    };

    match result {
        Some(value) => value,
        None => json!({
            "source": "offline-cosine",
            "results": local_scan(query, limit),
        }),
    }
}
