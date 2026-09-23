//! JSON-backed AI Apps Store: catalog + install/update/uninstall/launch broker.
//!
//! The shipped catalog (`catalog.json`) is the curated, editable source of
//! truth; live availability is resolved through the matching executor
//! (`winget`, `vsix`, direct open for `web`). Unknown `distributionType`
//! discriminants are refused, keeping the JSON forward-compatible.

use std::process::Command;

use serde_json::Value;

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
        "winget" | "chocolatey" | "scoop" | "unigetui" | "msstore" => Ok("package-manager"),
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

/// Resolve + install through the matching executor. Idempotent: package
/// presence is checked first via the manager's own list.
pub fn install(id: &str) -> Result<Value> {
    let entry = catalog_entry(id)?.ok_or_else(|| format!("unknown catalog id: {id}"))?;
    let executor = executor_of(&entry)?;
    let source_id = source_id_of(&entry).ok();

    let (exit_ok, stdout) = match executor {
        "package-manager" => {
            let src = source_id.ok_or("missing sourcePackageId")?;
            let out = Command::new("winget")
                .args([
                    "install",
                    "--id",
                    &src,
                    "-e",
                    "--accept-source-agreements",
                    "--accept-package-agreements",
                ])
                .output()?;
            (
                out.status.success(),
                String::from_utf8_lossy(&out.stdout).to_string(),
            )
        }
        "vsix" => {
            let src = source_id.ok_or("missing sourcePackageId")?;
            let out = Command::new("code")
                .args(["--install-extension", &src])
                .output()?;
            (
                out.status.success(),
                String::from_utf8_lossy(&out.stdout).to_string(),
            )
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
            (opened, format!("opened: {url}"))
        }
        _ => return Err("unreachable executor".into()),
    };

    Ok(serde_json::json!({
        "id": id,
        "executor": executor,
        "success": exit_ok,
        "output": stdout,
    }))
}

/// Update an installed entry through its executor.
pub fn update(id: &str) -> Result<Value> {
    let entry = catalog_entry(id)?.ok_or_else(|| format!("unknown catalog id: {id}"))?;
    let executor = executor_of(&entry)?;
    let source_id = source_id_of(&entry).ok();

    let (success, output) = match executor {
        "package-manager" => {
            let src = source_id.ok_or("missing sourcePackageId")?;
            let out = Command::new("winget")
                .args([
                    "upgrade",
                    "--id",
                    &src,
                    "-e",
                    "--accept-source-agreements",
                    "--accept-package-agreements",
                ])
                .output()?;
            (
                out.status.success(),
                String::from_utf8_lossy(&out.stdout).to_string(),
            )
        }
        "vsix" => {
            let src = source_id.ok_or("missing sourcePackageId")?;
            let out = Command::new("code")
                .args(["--install-extension", &src, "--force"])
                .output()?;
            (
                out.status.success(),
                String::from_utf8_lossy(&out.stdout).to_string(),
            )
        }
        _ => (
            false,
            "no versioned update lane for this executor".to_string(),
        ),
    };

    Ok(serde_json::json!({ "id": id, "executor": executor, "success": success, "output": output }))
}

/// Uninstall an installed entry through its executor (preserves user data).
pub fn uninstall(id: &str) -> Result<Value> {
    let entry = catalog_entry(id)?.ok_or_else(|| format!("unknown catalog id: {id}"))?;
    let executor = executor_of(&entry)?;
    let source_id = source_id_of(&entry).ok();

    let (success, output) = match executor {
        "package-manager" => {
            let src = source_id.ok_or("missing sourcePackageId")?;
            let out = Command::new("winget")
                .args([
                    "uninstall",
                    "--id",
                    &src,
                    "-e",
                    "--accept-source-agreements",
                ])
                .output()?;
            (
                out.status.success(),
                String::from_utf8_lossy(&out.stdout).to_string(),
            )
        }
        "vsix" => {
            let src = source_id.ok_or("missing sourcePackageId")?;
            let out = Command::new("code")
                .args(["--uninstall-extension", &src])
                .output()?;
            (
                out.status.success(),
                String::from_utf8_lossy(&out.stdout).to_string(),
            )
        }
        _ => (false, "no uninstall lane for this executor".to_string()),
    };

    Ok(serde_json::json!({ "id": id, "executor": executor, "success": success, "output": output }))
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
