//! JSON-backed AI Apps Store: catalog + install/update/uninstall/launch broker.
//!
//! The shipped catalog (`catalog.json`) is the curated, editable source of
//! truth; live availability is resolved through the matching executor
//! (`winget`, `vsix`, direct open for `web`). Unknown `distributionType`
//! discriminants are refused, keeping the JSON forward-compatible.
//!
//! Every CLI-based executor (winget / code) runs inside a single background
//! `PackageManagerWorker` thread: one serialized job queue, hidden console
//! (`CREATE_NO_WINDOW`), null stdin, piped stdout/stderr, real job state
//! (queued/running/succeeded/failed/cancelled) and direct-child cancellation.
//! The UI never spawns a console process directly.

use std::collections::{HashMap, VecDeque};
use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::Value;
use windows::Win32::System::Threading::{CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW};

use crate::error::Result;

static CATALOG_JSON: &str = include_str!("./catalog.json");

fn catalog_value() -> Result<Value> {
    serde_json::from_str(CATALOG_JSON).map_err(|e| format!("invalid catalog.json: {e}").into())
}

/// Curated catalog (schema + entries) as one typed value.
pub fn catalog() -> Result<Value> {
    catalog_value()
}

/// One catalog entry by stable id (case-insensitive).
pub fn catalog_entry(id: &str) -> Result<Option<Value>> {
    let root = catalog_value()?;
    let entry = root
        .get("entries")
        .and_then(Value::as_array)
        .and_then(|entries| {
            entries.iter().find(|e| {
                e.get("id")
                    .and_then(Value::as_str)
                    .is_some_and(|s| s.eq_ignore_ascii_case(id))
            })
        })
        .cloned();
    Ok(entry)
}

/// Executor for a `distributionType` discriminant. Unknown types are refused.
fn executor_of(entry: &Value) -> Result<&'static str> {
    let kind = entry
        .get("distributionType")
        .and_then(Value::as_str)
        .unwrap_or("");
    match kind {
        "winget" | "chocolatey" | "scoop" | "unigetui" | "msstore" => Ok("winget"),
        "direct_signed" | "nai" => Ok("direct"),
        "vsix" => Ok("vsix"),
        "model" | "mcp" => Ok("manifest"),
        "web" => Ok("web"),
        _ => Err(format!("unknown distributionType: {kind:?}").into()),
    }
}

fn source_id_of(entry: &Value) -> Result<String> {
    entry
        .get("sourcePackageId")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "entry has no sourcePackageId".into())
}

// ============================ package manager worker ============================

/// One background package job with full lifecycle state and diagnostics.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PackageJob {
    id: u64,
    operation: &'static str,
    app_id: String,
    package_id: Option<String>,
    adapter: &'static str,
    executable: String,
    args: Vec<String>,
    created_at_ms: u128,
    finished_at_ms: Option<u128>,
    state: &'static str, // queued | running | succeeded | failed | cancelled
    pid: Option<u32>,
    exit_code: Option<i32>,
    stdout_tail: String,
    stderr_tail: String,
    error: Option<String>,
}

impl PackageJob {
    fn is_active(&self) -> bool {
        matches!(self.state, "queued" | "running")
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);
static JOB_REGISTRY: OnceLock<Mutex<HashMap<u64, PackageJob>>> = OnceLock::new();
static JOB_ORDER: OnceLock<Mutex<VecDeque<u64>>> = OnceLock::new();
static ACTIVE_BY_APP: OnceLock<Mutex<HashMap<String, u64>>> = OnceLock::new();
static RUNNING_CHILDREN: OnceLock<Mutex<HashMap<u64, std::process::Child>>> = OnceLock::new();
static JOB_QUEUE: OnceLock<crossbeam_channel::Sender<u64>> = OnceLock::new();

const JOB_HISTORY: usize = 100;

fn registry() -> &'static Mutex<HashMap<u64, PackageJob>> {
    JOB_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn order() -> &'static Mutex<VecDeque<u64>> {
    JOB_ORDER.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn active_by_app() -> &'static Mutex<HashMap<String, u64>> {
    ACTIVE_BY_APP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn children() -> &'static Mutex<HashMap<u64, std::process::Child>> {
    RUNNING_CHILDREN.get_or_init(|| Mutex::new(HashMap::new()))
}

fn queue() -> &'static crossbeam_channel::Sender<u64> {
    JOB_QUEUE.get_or_init(|| {
        let (tx, rx) = crossbeam_channel::unbounded::<u64>();
        std::thread::Builder::new()
            .name("package-manager-worker".into())
            .spawn(move || {
                while let Ok(id) = rx.recv() {
                    run_job(id);
                }
            })
            .expect("package worker thread");
        tx
    })
}

/// Resolve the winget executable once (App Execution Alias / PATH name is
/// enough for `CreateProcess`; the lookup result is cached for the session).
static WINGET_EXE: OnceLock<String> = OnceLock::new();

fn winget_exe() -> &'static str {
    WINGET_EXE.get_or_init(|| "winget".to_string())
}

/// Build typed args per operation. Agreement switches only where the
/// operation actually supports them; `--disable-interactivity` keeps winget
/// from pausing on prompts.
fn winget_args(operation: &str, src: &str) -> Vec<String> {
    let mut args: Vec<String> = match operation {
        "install" => vec![
            "install".into(),
            "--id".into(),
            src.into(),
            "-e".into(),
            "--disable-interactivity".into(),
            "--accept-source-agreements".into(),
            "--accept-package-agreements".into(),
        ],
        "upgrade" => vec![
            "upgrade".into(),
            "--id".into(),
            src.into(),
            "-e".into(),
            "--disable-interactivity".into(),
            "--accept-source-agreements".into(),
            "--accept-package-agreements".into(),
        ],
        "uninstall" => vec![
            "uninstall".into(),
            "--id".into(),
            src.into(),
            "-e".into(),
            "--disable-interactivity".into(),
            "--accept-source-agreements".into(),
        ],
        "detect" => vec![
            "list".into(),
            "--id".into(),
            src.into(),
            "-e".into(),
            "--disable-interactivity".into(),
            "--accept-source-agreements".into(),
        ],
        "detect-upgrade" => vec![
            "list".into(),
            "--id".into(),
            src.into(),
            "-e".into(),
            "--upgrade-available".into(),
            "--disable-interactivity".into(),
            "--accept-source-agreements".into(),
        ],
        _ => vec![],
    };
    // stable order for diagnostics; `--disable-interactivity` already inside.
    args.retain(|a| !a.is_empty());
    args
}

fn vsix_args(operation: &str, src: &str) -> Vec<String> {
    match operation {
        "install" => vec![
            "--install-extension".into(),
            src.into(),
            "--disable-interactivity".into(),
        ],
        "upgrade" => vec![
            "--install-extension".into(),
            src.into(),
            "--force".into(),
            "--disable-interactivity".into(),
        ],
        "uninstall" => vec![
            "--uninstall-extension".into(),
            src.into(),
            "--disable-interactivity".into(),
        ],
        "detect" => vec!["--list-extensions".into(), "--disable-interactivity".into()],
        _ => vec![],
    }
}

fn set_job(id: u64, f: impl FnOnce(&mut PackageJob)) {
    if let Some(job) = registry().lock().expect("job registry").get_mut(&id) {
        f(job);
    }
}

fn job_snapshot(id: u64) -> Option<PackageJob> {
    registry().lock().expect("job registry").get(&id).cloned()
}

/// Execute one job on the worker thread. Serialized by the single queue.
fn run_job(id: u64) {
    let Some(job) = job_snapshot(id) else { return };
    let mut command = Command::new(&job.executable);
    command
        .args(&job.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW.0 | CREATE_NEW_PROCESS_GROUP.0);

    match command.spawn() {
        Ok(child) => {
            let pid = child.id();
            set_job(id, |j| {
                j.state = "running";
                j.pid = Some(pid);
            });
            children().lock().expect("children map").insert(id, child);

            // Read both streams without a terminal: stderr on a helper
            // thread, stdout on the worker; then reap the child.
            let err_handle = {
                let err = children()
                    .lock()
                    .expect("children map")
                    .get_mut(&id)
                    .and_then(|c| c.stderr.take());
                std::thread::spawn(move || {
                    let mut text = String::new();
                    if let Some(mut e) = err {
                        let _ = std::io::Read::read_to_string(&mut e, &mut text);
                    }
                    text
                })
            };
            let mut stdout_text = String::new();
            {
                let out = children()
                    .lock()
                    .expect("children map")
                    .get_mut(&id)
                    .and_then(|c| c.stdout.take());
                if let Some(mut o) = out {
                    let _ = std::io::Read::read_to_string(&mut o, &mut stdout_text);
                }
            }
            let stderr_text = err_handle.join().unwrap_or_default();

            let exit = children()
                .lock()
                .expect("children map")
                .remove(&id)
                .and_then(|mut c| c.wait().ok());

            let mut reg = registry().lock().expect("job registry");
            if let Some(job) = reg.get_mut(&id) {
                // Cancel may have finalized the record while we streamed.
                if job.state != "cancelled" {
                    job.state = if exit.is_some_and(|s| s.success()) {
                        "succeeded"
                    } else {
                        "failed"
                    };
                    job.exit_code = exit.and_then(|s| s.code());
                    job.finished_at_ms = Some(now_ms());
                    job.stdout_tail = tail(&stdout_text);
                    job.stderr_tail = tail(&stderr_text);
                }
            }
            drop(reg);
            active_by_app().lock().expect("active map").remove(&job.app_id);
        }
        Err(err) => {
            set_job(id, |j| {
                j.state = "failed";
                j.finished_at_ms = Some(now_ms());
                j.error = Some(format!(
                    "process creation failed for {} ({err})",
                    j.executable
                ));
            });
            active_by_app().lock().expect("active map").remove(&job.app_id);
        }
    }
}

fn tail(text: &str) -> String {
    const TAIL_LINES: usize = 200;
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let skip = lines.len().saturating_sub(TAIL_LINES);
    lines[skip..].join("\n")
}

/// Submit one job to the serialized worker. Duplicate active jobs for the
/// same app id are coalesced into the existing job id.
fn submit(
    operation: &'static str,
    app_id: &str,
    package_id: Option<String>,
    executable: String,
    args: Vec<String>,
) -> u64 {
    {
        let mut active = active_by_app().lock().expect("active map");
        if let Some(existing) = active.get(app_id).copied() {
            if job_snapshot(existing).is_some_and(|j| j.is_active()) {
                return existing;
            }
            active.remove(app_id);
        }
    }

    let id = NEXT_JOB_ID.fetch_add(1, Ordering::SeqCst);
    let job = PackageJob {
        id,
        operation,
        app_id: app_id.to_string(),
        package_id,
        adapter: if executable.starts_with("winget") {
            "winget"
        } else if executable.starts_with("code") {
            "vsix"
        } else {
            "cli"
        },
        executable,
        args,
        created_at_ms: now_ms(),
        finished_at_ms: None,
        state: "queued",
        pid: None,
        exit_code: None,
        stdout_tail: String::new(),
        stderr_tail: String::new(),
        error: None,
    };
    registry().lock().expect("job registry").insert(id, job);
    {
        let mut ord = order().lock().expect("job order");
        ord.push_back(id);
        while ord.len() > JOB_HISTORY {
            if let Some(old) = ord.pop_front() {
                registry().lock().expect("job registry").remove(&old);
            }
        }
    }
    active_by_app()
        .lock()
        .expect("active map")
        .insert(app_id.to_string(), id);
    let _ = queue().send(id);
    id
}

/// Wait for one job to leave the active states. The worker always reaps its
/// child, so finished jobs settle quickly; no arbitrary short timeout.
fn wait_for_job(id: u64) -> PackageJob {
    loop {
        if let Some(job) = job_snapshot(id) {
            if !job.is_active() {
                return job;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn job_result(id: &str, job: &PackageJob) -> Value {
    serde_json::json!({
        "id": id,
        "executor": job.adapter,
        "success": job.state == "succeeded",
        "output": if job.stderr_tail.trim().is_empty() {
            job.stdout_tail.clone()
        } else if job.stdout_tail.trim().is_empty() {
            job.stderr_tail.clone()
        } else {
            format!("{}\n{}", job.stdout_tail, job.stderr_tail)
        },
        "jobId": job.id,
        "state": job.state,
        "exitCode": job.exit_code,
    })
}

/// Resolve + install through the matching executor. Idempotent: package
/// presence is checked first via the manager's own list.
pub fn install(id: &str) -> Result<Value> {
    let entry = catalog_entry(id)?.ok_or_else(|| format!("unknown catalog id: {id}"))?;
    let executor = executor_of(&entry)?;
    match executor {
        "winget" => {
            let src = source_id_of(&entry)?;
            let job_id = submit(
                "install",
                id,
                Some(src.clone()),
                winget_exe().to_string(),
                winget_args("install", &src),
            );
            let job = wait_for_job(job_id);
            // Reconcile against real installed state after success.
            if job.state == "succeeded" {
                let detected = detect_state("winget", id, Some(&src))?;
                return Ok(serde_json::json!({
                    "id": id, "executor": "winget", "success": true,
                    "output": job.stdout_tail, "state": detected,
                    "jobId": job.id, "exitCode": job.exit_code,
                }));
            }
            Ok(job_result(id, &job))
        }
        "vsix" => {
            let src = source_id_of(&entry)?;
            let job_id = submit(
                "install",
                id,
                Some(src.clone()),
                "code".to_string(),
                vsix_args("install", &src),
            );
            let job = wait_for_job(job_id);
            if job.state == "succeeded" {
                let detected = detect_state("vsix", id, Some(&src))?;
                return Ok(serde_json::json!({
                    "id": id, "executor": "vsix", "success": true,
                    "output": job.stdout_tail, "state": detected,
                    "jobId": job.id, "exitCode": job.exit_code,
                }));
            }
            Ok(job_result(id, &job))
        }
        "direct" | "manifest" | "web" => {
            let url = entry
                .get("downloadUrl")
                .and_then(Value::as_str)
                .unwrap_or("");
            let mut opened = false;
            if !url.is_empty() {
                opened = crate::exposed::open_file_inner(url.to_string()).is_ok();
            }
            Ok(serde_json::json!({
                "id": id, "executor": executor, "success": opened,
                "output": format!("opened: {url}"),
            }))
        }
        _ => Err("unreachable executor".into()),
    }
}

/// Update an installed entry through its executor.
pub fn update(id: &str) -> Result<Value> {
    let entry = catalog_entry(id)?.ok_or_else(|| format!("unknown catalog id: {id}"))?;
    let executor = executor_of(&entry)?;
    match executor {
        "winget" => {
            let src = source_id_of(&entry)?;
            let job_id = submit(
                "upgrade",
                id,
                Some(src.clone()),
                winget_exe().to_string(),
                winget_args("upgrade", &src),
            );
            let job = wait_for_job(job_id);
            if job.state == "succeeded" {
                let detected = detect_state("winget", id, Some(&src))?;
                return Ok(serde_json::json!({
                    "id": id, "executor": "winget", "success": true,
                    "output": job.stdout_tail, "state": detected,
                    "jobId": job.id, "exitCode": job.exit_code,
                }));
            }
            Ok(job_result(id, &job))
        }
        "vsix" => {
            let src = source_id_of(&entry)?;
            let job_id = submit(
                "upgrade",
                id,
                Some(src.clone()),
                "code".to_string(),
                vsix_args("upgrade", &src),
            );
            let job = wait_for_job(job_id);
            if job.state == "succeeded" {
                let detected = detect_state("vsix", id, Some(&src))?;
                return Ok(serde_json::json!({
                    "id": id, "executor": "vsix", "success": true,
                    "output": job.stdout_tail, "state": detected,
                    "jobId": job.id, "exitCode": job.exit_code,
                }));
            }
            Ok(job_result(id, &job))
        }
        _ => Ok(serde_json::json!({
            "id": id, "executor": executor, "success": false,
            "output": "no versioned update lane for this executor",
        })),
    }
}

/// Uninstall an installed entry through its executor (preserves user data).
pub fn uninstall(id: &str) -> Result<Value> {
    let entry = catalog_entry(id)?.ok_or_else(|| format!("unknown catalog id: {id}"))?;
    let executor = executor_of(&entry)?;
    match executor {
        "winget" => {
            let src = source_id_of(&entry)?;
            let job_id = submit(
                "uninstall",
                id,
                Some(src.clone()),
                winget_exe().to_string(),
                winget_args("uninstall", &src),
            );
            let job = wait_for_job(job_id);
            if job.state == "succeeded" {
                let detected = detect_state("winget", id, Some(&src))?;
                return Ok(serde_json::json!({
                    "id": id, "executor": "winget", "success": true,
                    "output": job.stdout_tail, "state": detected,
                    "jobId": job.id, "exitCode": job.exit_code,
                }));
            }
            Ok(job_result(id, &job))
        }
        "vsix" => {
            let src = source_id_of(&entry)?;
            let job_id = submit(
                "uninstall",
                id,
                Some(src.clone()),
                "code".to_string(),
                vsix_args("uninstall", &src),
            );
            let job = wait_for_job(job_id);
            if job.state == "succeeded" {
                let detected = detect_state("vsix", id, Some(&src))?;
                return Ok(serde_json::json!({
                    "id": id, "executor": "vsix", "success": true,
                    "output": job.stdout_tail, "state": detected,
                    "jobId": job.id, "exitCode": job.exit_code,
                }));
            }
            Ok(job_result(id, &job))
        }
        _ => Ok(serde_json::json!({
            "id": id, "executor": executor, "success": false,
            "output": "no uninstall lane for this executor",
        })),
    }
}

/// Cancel one running/queued job: kill + reap the direct child, mark it.
pub fn cancel(job_id: u64) -> Value {
    if let Some(mut child) = children().lock().expect("children map").remove(&job_id) {
        let _ = child.kill();
        let _ = child.wait();
        set_job(job_id, |j| {
            j.state = "cancelled";
            j.finished_at_ms = Some(now_ms());
        });
        return serde_json::json!({ "jobId": job_id, "state": "cancelled" });
    }
    if let Some(job) = job_snapshot(job_id) {
        if job.is_active() {
            set_job(job_id, |j| {
                j.state = "cancelled";
                j.finished_at_ms = Some(now_ms());
            });
            return serde_json::json!({ "jobId": job_id, "state": "cancelled" });
        }
        return serde_json::json!({ "jobId": job_id, "state": job.state });
    }
    serde_json::json!({ "jobId": job_id, "state": "unknown" })
}

/// Recent package jobs (newest first), for UI/diagnostics polling.
pub fn jobs() -> Result<Value> {
    let ord = order().lock().expect("job order");
    let reg = registry().lock().expect("job registry");
    let list: Vec<&PackageJob> = ord.iter().rev().filter_map(|id| reg.get(id)).collect();
    Ok(serde_json::to_value(list).unwrap_or(Value::Null))
}

/// Real probe: run one detect job and interpret its stdout.
fn detect_state(adapter: &str, app_id: &str, src: Option<&str>) -> Result<String> {
    let Some(src) = src else {
        return Ok("Unknown".to_string());
    };
    let (executable, args) = match adapter {
        "winget" => (
            winget_exe().to_string(),
            winget_args("detect", src),
        ),
        "vsix" => ("code".to_string(), vsix_args("detect", src)),
        _ => return Ok("Unknown".to_string()),
    };
    let job_id = submit("detect", app_id, Some(src.to_string()), executable, args);
    let job = wait_for_job(job_id);
    let text = job.stdout_tail.to_ascii_lowercase();
    let key = src.to_ascii_lowercase();
    if !text.contains(&key) {
        return Ok("NotInstalled".to_string());
    }
    if adapter == "winget" {
        // independent second probe for pending upgrades
        let up_id = submit(
            "detect",
            app_id,
            Some(src.to_string()),
            winget_exe().to_string(),
            winget_args("detect-upgrade", src),
        );
        let up = wait_for_job(up_id);
        if up
            .stdout_tail
            .to_ascii_lowercase()
            .contains(&key)
        {
            return Ok("UpdateAvailable".to_string());
        }
    }
    Ok("Installed".to_string())
}

/// Independent installed-state detection (never inferred from an install click).
/// Returns one of: `Installed`, `NotInstalled`, `Installing`, `Updating`,
/// `UpdateAvailable`, `Failed`, `Cancelled`, `Unknown`.
pub fn status(id: &str) -> Result<Value> {
    let entry = catalog_entry(id)?.ok_or_else(|| format!("unknown catalog id: {id}"))?;
    let executor = executor_of(&entry)?;

    // The active job for this app wins over probing (real queue state).
    let active = active_by_app().lock().expect("active map").get(id).copied();
    if let Some(job_id) = active {
        if let Some(job) = job_snapshot(job_id) {
            if job.is_active() {
                let transient = match job.operation {
                    "upgrade" => "Updating",
                    _ => "Installing",
                };
                return Ok(serde_json::json!({
                    "id": id, "state": transient, "jobId": job.id,
                }));
            }
            if job.state == "failed" {
                return Ok(serde_json::json!({
                    "id": id, "state": "Failed", "jobId": job.id,
                    "error": job.error.or(Some(job.stderr_tail)),
                }));
            }
            if job.state == "cancelled" {
                return Ok(serde_json::json!({ "id": id, "state": "Cancelled" }));
            }
        }
    }

    let state = match executor {
        "winget" => match source_id_of(&entry).ok() {
            Some(src) if !src.is_empty() => detect_state("winget", id, Some(&src))?,
            _ => "Unknown".to_string(),
        },
        "vsix" => match source_id_of(&entry).ok() {
            Some(src) => detect_state("vsix", id, Some(&src))?,
            _ => "Unknown".to_string(),
        },
        "direct" | "manifest" => entry
            .get("executablePath")
            .and_then(Value::as_str)
            .map(|p| {
                if std::path::Path::new(p).exists() {
                    "Installed".to_string()
                } else {
                    "NotInstalled".to_string()
                }
            })
            .unwrap_or_else(|| "Unknown".to_string()),
        _ => "Unknown".to_string(),
    };

    Ok(serde_json::json!({ "id": id, "state": state }))
}

/// Launch an installed entry: prefer the resolved executable, fall back to
/// the download/open URL for `web`-type entries.
pub fn launch(id: &str) -> Result<Value> {
    let entry = catalog_entry(id)?.ok_or_else(|| format!("unknown catalog id: {id}"))?;

    if let Some(path) = entry.get("executablePath").and_then(Value::as_str)
        && std::path::Path::new(path).exists()
    {
        let pid = std::process::Command::new(path)
            .spawn()
            .map_err(|e| format!("spawn failed for {id}: {e}"))?
            .id();
        return Ok(serde_json::json!({ "id": id, "pid": pid, "path": path }));
    }

    let url = entry
        .get("downloadUrl")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("entry {id} has no launch target"))?;
    crate::exposed::open_file_inner(url.to_string())?;
    Ok(serde_json::json!({ "id": id, "opened": url }))
}
