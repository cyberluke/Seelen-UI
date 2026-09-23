//! Boot pipeline flight recorder.
//!
//! `Mounting` alone conflates too many failure states, so every boot stage is
//! recorded with a monotonic timestamp plus the delta from the previous stage.
//! Two planes are recorded:
//!
//! - global backend stages (`process.started` ... `widget.reconcile.complete`),
//! - per-pod stages (backend creation + frontend bootstrap + `widget.ready.done`).
//!
//! The recorder is the single source for `slu widget boot`, liveness RTT
//! percentiles, reload / mount-failure / liveness-failure counters and the
//! "first missing/failing stage" analysis. It is a cache of evidence with a
//! bounded ring; never authoritative desktop state.

use std::{
    collections::{HashMap, VecDeque},
    sync::{LazyLock, Mutex, OnceLock},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

/// Max recorded stages per pod (bounded).
const POD_STAGE_CAPACITY: usize = 48;
/// Max RTT samples per pod (bounded).
const RTT_SAMPLE_CAPACITY: usize = 128;

static PROCESS_T0: OnceLock<Instant> = OnceLock::new();

fn t0() -> Instant {
    *PROCESS_T0.get_or_init(Instant::now)
}

fn mono_ns() -> u128 {
    t0().elapsed().as_nanos()
}

/// Monotonic microseconds from process start (for liveness RTT pairing).
pub fn mono_us() -> u64 {
    t0().elapsed().as_micros() as u64
}

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
struct StageRecord {
    stage: String,
    at_ns: u128,
    delta_ns: u128,
    unix_ms: u64,
    source: &'static str, // "backend" | "frontend"
    /// pod generation this record belongs to (0 for global stages)
    generation: u64,
}

#[derive(Debug, Default)]
struct PodRecorder {
    stages: VecDeque<StageRecord>,
    last_ns: Option<u128>,
    generation: u64,
    reloads: u32,
    mount_failures: u32,
    liveness_failures: u32,
    gave_up: bool,
    rtt_us: VecDeque<u64>,
}

#[derive(Debug, Default)]
struct BootRecorder {
    global: VecDeque<StageRecord>,
    global_last_ns: Option<u128>,
    pods: HashMap<String, PodRecorder>,
}

static RECORDER: LazyLock<Mutex<BootRecorder>> =
    LazyLock::new(|| Mutex::new(BootRecorder::default()));

fn push(rec: &mut VecDeque<StageRecord>, last: &mut Option<u128>, record: StageRecord) {
    if rec.len() == POD_STAGE_CAPACITY {
        rec.pop_front();
    }
    let at = record.at_ns;
    rec.push_back(record);
    *last = Some(at);
}

/// Record a global backend boot stage (`process.started`, `integrity.complete`, ...).
pub fn record_global(stage: &'static str) {
    let at = mono_ns();
    let mut guard = RECORDER.lock().unwrap_or_else(|e| e.into_inner());
    // destructure once so disjoint fields can be borrowed independently
    let BootRecorder {
        global,
        global_last_ns,
        pods: _,
    } = &mut *guard;
    let delta = global_last_ns
        .map(|prev| at.saturating_sub(prev))
        .unwrap_or(0);
    push(
        global,
        global_last_ns,
        StageRecord {
            stage: stage.to_string(),
            at_ns: at,
            delta_ns: delta,
            unix_ms: unix_ms(),
            source: "backend",
            generation: 0,
        },
    );
}

/// Record a per-pod backend stage.
pub fn record_pod(label: &str, stage: &'static str) {
    record_pod_inner(label, stage, "backend")
}

/// Record a per-pod frontend stage (via `record_boot_stage`).
pub fn record_pod_frontend(label: &str, stage: &str) {
    record_pod_inner(label, stage, "frontend")
}

fn record_pod_inner(label: &str, stage: &str, source: &'static str) {
    let at = mono_ns();
    let mut guard = RECORDER.lock().unwrap_or_else(|e| e.into_inner());
    let BootRecorder {
        global: _,
        global_last_ns: _,
        pods,
    } = &mut *guard;
    let pod = pods.entry(label.to_string()).or_default();
    let delta = pod.last_ns.map(|prev| at.saturating_sub(prev)).unwrap_or(0);
    if pod.stages.len() == POD_STAGE_CAPACITY {
        pod.stages.pop_front();
    }
    pod.stages.push_back(StageRecord {
        stage: stage.to_string(),
        at_ns: at,
        delta_ns: delta,
        unix_ms: unix_ms(),
        source,
        generation: pod.generation,
    });
    pod.last_ns = Some(at);
}

pub fn bump_generation(label: &str) -> u64 {
    let mut guard = RECORDER.lock().unwrap_or_else(|e| e.into_inner());
    let BootRecorder { pods, .. } = &mut *guard;
    let pod = pods.entry(label.to_string()).or_default();
    pod.generation += 1;
    pod.generation
}

#[allow(dead_code)]
pub fn generation(label: &str) -> u64 {
    let rec = RECORDER.lock().unwrap_or_else(|e| e.into_inner());
    rec.pods.get(label).map(|p| p.generation).unwrap_or(0)
}

pub fn record_rtt(label: &str, rtt_us: u64) {
    let mut guard = RECORDER.lock().unwrap_or_else(|e| e.into_inner());
    let BootRecorder { pods, .. } = &mut *guard;
    let pod = pods.entry(label.to_string()).or_default();
    if pod.rtt_us.len() == RTT_SAMPLE_CAPACITY {
        pod.rtt_us.pop_front();
    }
    pod.rtt_us.push_back(rtt_us);
}

pub fn record_reload(label: &str) {
    let mut guard = RECORDER.lock().unwrap_or_else(|e| e.into_inner());
    let BootRecorder { pods, .. } = &mut *guard;
    pods.entry(label.to_string()).or_default().reloads += 1;
}

pub fn record_mount_failure(label: &str) {
    let mut guard = RECORDER.lock().unwrap_or_else(|e| e.into_inner());
    let BootRecorder { pods, .. } = &mut *guard;
    pods.entry(label.to_string()).or_default().mount_failures += 1;
}

pub fn record_liveness_failure(label: &str) {
    let mut guard = RECORDER.lock().unwrap_or_else(|e| e.into_inner());
    let BootRecorder { pods, .. } = &mut *guard;
    pods.entry(label.to_string()).or_default().liveness_failures += 1;
}

pub fn record_gave_up(label: &str) {
    let mut guard = RECORDER.lock().unwrap_or_else(|e| e.into_inner());
    let BootRecorder { pods, .. } = &mut *guard;
    pods.entry(label.to_string()).or_default().gave_up = true;
}

// ============================ expected pipelines ============================

const BACKEND_PIPELINE: [&str; 7] = [
    "process.started",
    "single_instance.acquired",
    "integrity.complete",
    "bundle.complete",
    "resources.complete",
    "state.complete",
    "widget.reconcile.complete",
];

const POD_BACKEND_PIPELINE: [&str; 2] = ["widget.native_window.created", "widget.mounting"];

const POD_FRONTEND_PIPELINE: [&str; 12] = [
    "bootstrap.module.loaded",
    "bootstrap.liveness.listener.registered",
    "bootstrap.widget_definition.request.start",
    "bootstrap.widget_definition.request.done",
    "bootstrap.widget_definition.resolved",
    "bootstrap.local_storage.hooked",
    "bootstrap.widget_entry.fetch.start",
    "bootstrap.widget_entry.fetch.done",
    "bootstrap.widget_entry.injected",
    "widget.module.loaded",
    "widget.init.start",
    "widget.init.done",
];

fn first_missing<'a>(pipeline: &'a [&'a str], recorded: &[StageRecord]) -> Option<&'a str> {
    for stage in pipeline.iter() {
        if !recorded.iter().any(|r| r.stage == **stage) {
            // stages may roll off the ring; only report missing if any recorded
            // stage is at-or-after the missing one (i.e. boot passed it)
            if let Some(idx) = pipeline.iter().position(|s| *s == *stage) {
                let passed = recorded.iter().any(|r| {
                    pipeline
                        .iter()
                        .skip(idx + 1)
                        .any(|later| *later == r.stage.as_str())
                });
                if !passed {
                    return Some(stage);
                }
            }
        }
    }
    None
}

fn percentiles(values: &VecDeque<u64>) -> Option<Value> {
    if values.is_empty() {
        return None;
    }
    let mut v: Vec<u64> = values.iter().copied().collect();
    v.sort_unstable();
    let n = v.len();
    let at = |q: f64| -> f64 {
        let idx = ((n as f64 - 1.0) * q).ceil() as usize;
        v[idx.min(n - 1)] as f64
    };
    Some(serde_json::json!({
        "samples": n,
        "p50Us": at(0.50),
        "p95Us": at(0.95),
        "p99Us": at(0.99),
        "maxUs": *v.last().unwrap() as f64,
    }))
}

fn stage_json(r: &StageRecord) -> Value {
    serde_json::json!({
        "stage": r.stage,
        "monotonicNs": r.at_ns,
        "deltaNs": r.delta_ns,
        "unixMs": r.unix_ms,
        "source": r.source,
    })
}

/// Snapshot for `slu widget boot`. With `label` filters to one pod (raw or
/// decoded label); otherwise returns the global pipeline + all pods overview.
pub fn snapshot(label: Option<&str>) -> Value {
    let rec = RECORDER.lock().unwrap_or_else(|e| e.into_inner());
    match label.and_then(|l| {
        rec.pods.get(l).or_else(|| {
            // allow matching by decoded label prefix / widget id
            rec.pods
                .iter()
                .find(|(k, _)| k.starts_with(l))
                .map(|(_, v)| v)
        })
    }) {
        Some(pod) => serde_json::json!({
            "label": label,
            "pid": std::process::id(),
            "generation": pod.generation,
            "reloads": pod.reloads,
            "mountFailures": pod.mount_failures,
            "livenessFailures": pod.liveness_failures,
            "gaveUp": pod.gave_up,
            "livenessRtt": percentiles(&pod.rtt_us),
            "stages": pod.stages.iter().map(stage_json).collect::<Vec<_>>(),
            // generation-scoped analysis: only the stages recorded for the
            // CURRENT generation count; a fully rolled ring for this generation
            // is reported via `ringEmptyForGeneration` instead of being mixed
            // with older generations' stages.
            "firstMissingStage": first_missing(
                &POD_BACKEND_PIPELINE.into_iter().chain(POD_FRONTEND_PIPELINE).collect::<Vec<_>>(),
                &pod.stages.iter().filter(|r| r.generation == pod.generation).cloned().collect::<Vec<_>>(),
            ),
            "ringEmptyForGeneration": !pod.stages.iter().any(|r| r.generation == pod.generation),
        }),
        None => serde_json::json!({
            "pid": std::process::id(),
            "globalStages": rec.global.iter().map(stage_json).collect::<Vec<_>>(),
            "globalFirstMissingStage": first_missing(
                &BACKEND_PIPELINE,
                &rec.global.iter().cloned().collect::<Vec<_>>(),
            ),
            "pods": rec.pods.iter().map(|(k, pod)| serde_json::json!({
                "label": k,
                "generation": pod.generation,
                "lastStage": pod.stages.back().map(|r| r.stage.clone()),
                "reloads": pod.reloads,
                "mountFailures": pod.mount_failures,
                "livenessFailures": pod.liveness_failures,
                "gaveUp": pod.gave_up,
                "livenessRtt": percentiles(&pod.rtt_us),
            })).collect::<Vec<_>>(),
        }),
    }
}

// ============================ build provenance ============================

/// Build provenance for the running executable + bundled frontend.
pub fn provenance() -> Value {
    serde_json::json!({
        "gitSha": option_env!("SUL_GIT_SHA").unwrap_or("unknown"),
        "dirty": option_env!("SUL_GIT_DIRTY").map(|v| v == "1").unwrap_or(true),
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "tauriMode": if tauri::is_dev() { "development" } else { "custom-protocol" },
        "frontendBundleHash": option_env!("SUL_FRONTEND_HASH").unwrap_or("unknown"),
        "staticResourceHash": option_env!("SUL_STATIC_HASH").unwrap_or("unknown"),
        "buildTimestamp": option_env!("SUL_BUILD_TIME").unwrap_or("unknown"),
        "target": option_env!("SUL_TARGET").unwrap_or("unknown"),
        "packageVersion": env!("CARGO_PKG_VERSION"),
    })
}

/// Runtime instance identity for `slu runtime instance`.
pub fn instance_info() -> Value {
    let (owns_mutex, session_id, mutex_name) = crate::utils::integrity::instance_status();
    let service_pid = crate::cli::ServicePipe::service_pid();
    let http_port = {
        let state = crate::state::application::FULL_STATE.load();
        let automation = &state.settings.by_widget.weg.automation;
        if automation.enabled && automation.rest_enabled {
            Some(automation.rest_port)
        } else {
            None
        }
    };
    serde_json::json!({
        "guiPid": std::process::id(),
        "servicePid": service_pid,
        "sessionId": session_id,
        "mutexName": mutex_name,
        "ownsMutex": owns_mutex,
        "httpPortOwner": http_port,
    })
}

/// Count of live main GUI instances (`NAI-OS.exe` processes), answering
/// `--instances` for the queried session. Each primary holds one per-session
/// mutex; secondaries are short-lived CLI relays that exit right after the IPC
/// exchange, so a simple process-name count is exact here.
pub fn instance_count() -> Value {
    use sysinfo::{ProcessesToUpdate, System};

    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let pids: Vec<u32> = sys
        .processes()
        .iter()
        .filter(|(_, p)| p.name().to_str() == Some("NAI-OS.exe"))
        .map(|(pid, _)| (*pid).as_u32())
        .collect();

    serde_json::json!({
        "count": pids.len(),
        "pids": pids,
        "currentPid": std::process::id(),
    })
}
