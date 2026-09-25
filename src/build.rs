use std::{fs::create_dir, path::PathBuf};

use slu_utils::{checksums::CheckSums, signature::sign_minisign};

fn main() {
    let _ = create_dir("gen");

    // ---- semantic (schema) validation gate, independent of checksums ----
    // Parses every bundled widget / plugin resource with the very same
    // `seelen_core` types the runtime uses. A resource that cannot
    // deserialize into its runtime type fails the production build.
    let semantic = validate_resources_semantically();
    for line in &semantic {
        println!("cargo:warning={line}");
    }
    if semantic.iter().any(|l| l.starts_with("SEMANTIC-FAIL")) {
        panic!("semantic resource validation failed");
    }
    println!("cargo:warning=SEMANTIC-OK widgets+plugins parsed with runtime types");

    let mut checksums = CheckSums::new();
    read_folder_recursive(PathBuf::from("static"), &mut |path| {
        checksums.add(&path).unwrap();
    });

    let target_dir = target_dir();
    let sums_path = target_dir.join("SHA256SUMS");
    checksums.write(&sums_path).unwrap();

    if !cfg!(debug_assertions) {
        sign_sha256sums(&sums_path);
    } else {
        std::fs::write(
            sums_path.with_extension("sig"),
            "NOT SIGNED NEEDED FOR DEBUG",
        )
        .unwrap();
    }

    emit_provenance_env(&sums_path);
    emit_exe_path_env();
    write_provenance_file(&target_dir);

    // ---- release provenance gate (fails packaging on the old regression) ----
    if !cfg!(debug_assertions) {
        if !cfg!(feature = "custom-protocol") {
            panic!(
                "FATAL: production runtime is using Tauri development asset mode. Expected custom-protocol."
            );
        }
    }

    tauri_build::build();
}

/// Build-time exe location for the toolbar launcher icon (`get_runtime_exe_path`).
/// Points at the exact artifact produced by this build: the profile directory is
/// resolved from `OUT_DIR`, so both plain (`target/debug`) and triple-specific
/// (`target/<triple>/release`) layouts are covered automatically.
fn emit_exe_path_env() {
    let exe_path = target_dir().join("NAI-OS.exe");
    println!("cargo:rustc-env=SUL_EXE_PATH={}", exe_path.display());
}

/// Machine-readable provenance next to the artifacts, for the packaging gate.
/// Values are computed directly (same sources as `emit_provenance_env`) so the
/// file matches the `option_env!` constants baked into the binaries.
fn write_provenance_file(target_dir: &PathBuf) {
    let git_sha = git_output(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let build_timestamp = git_output(&["log", "-1", "--format=%cI"]).unwrap_or_default();
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());
    let build_id = read_build_id_file()
        .or_else(|| {
            (git_sha != "unknown")
                .then(|| format!("{}-{}", &git_sha, build_timestamp.trim()))
        })
        .unwrap_or_else(|| "unknown".into());
    let static_hash = std::fs::read(target_dir.join("SHA256SUMS"))
        .ok()
        .map(|raw| slu_utils::checksums::calculate_sha256(&raw))
        .unwrap_or_else(|| "unknown".into());

    let features: Vec<&str> = if cfg!(feature = "custom-protocol") {
        vec!["custom-protocol"]
    } else {
        vec![]
    };
    let provenance = serde_json::json!({
        "buildId": build_id,
        "gitSha": git_sha,
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "tauriMode": if cfg!(feature = "custom-protocol") { "custom-protocol" } else { "development" },
        "features": features,
        "frontendBundleHash": dist_bundle_hash().unwrap_or_else(|| "unknown".into()),
        "staticResourceHash": static_hash,
        "buildTimestamp": build_timestamp.trim(),
        "target": target,
    });
    std::fs::write(
        target_dir.join("provenance.json"),
        serde_json::to_vec_pretty(&provenance).unwrap(),
    )
    .unwrap();
}

/// Build-time provenance baked into the binary (`slu runtime provenance`).
fn emit_provenance_env(sums_path: &PathBuf) {
    let git_sha = git_output(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git_output(&["status", "--porcelain"])
        .map(|s| if s.trim().is_empty() { "0" } else { "1" })
        .unwrap_or("1");
    let build_timestamp = git_output(&["log", "-1", "--format=%cI"]).unwrap_or_default();
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());

    let resource_hash = std::fs::read(sums_path)
        .ok()
        .map(|raw| slu_utils::checksums::calculate_sha256(&raw));
    // dist bundle hash: hash of the concatenated generated index files, when present
    let frontend_hash = dist_bundle_hash();

    // Canonical build identity, shared with the frontend via `_build-id.json`.
    // The wrapper build script writes it first; when building with bare cargo
    // we recompute the same deterministic value here (sha + commit timestamp).
    let build_id = read_build_id_file()
        .or_else(|| {
            (git_sha != "unknown")
                .then(|| format!("{}-{}", &git_sha, build_timestamp.trim()))
        })
        .unwrap_or_else(|| "unknown".into());

    println!("cargo:rustc-env=SUL_GIT_SHA={git_sha}");
    println!("cargo:rustc-env=SUL_GIT_DIRTY={dirty}");
    println!("cargo:rustc-env=SUL_BUILD_TIME={build_timestamp}");
    println!("cargo:rustc-env=SUL_TARGET={target}");
    println!("cargo:rustc-env=SUL_BUILD_ID={build_id}");
    println!(
        "cargo:rustc-env=SUL_CUSTOM_PROTOCOL={}",
        if cfg!(feature = "custom-protocol") {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!(
        "cargo:rustc-env=SUL_STATIC_HASH={}",
        resource_hash.unwrap_or_else(|| "unknown".into())
    );
    println!(
        "cargo:rustc-env=SUL_FRONTEND_HASH={}",
        frontend_hash.unwrap_or_else(|| "unknown".into())
    );
}

fn read_build_id_file() -> Option<String> {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").ok()?);
    for candidate in [
        manifest.join("static").join("_build-id.json"),
        manifest.join("..").join("dist").join("_build-id.json"),
        manifest.join("..").join("_build-id.json"),
    ] {
        let Ok(raw) = std::fs::read(&candidate) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&raw) else {
            continue;
        };
        if let Some(id) = value.get("buildId").and_then(|v| v.as_str()) {
            return Some(id.to_string());
        }
    }
    None
}

fn dist_bundle_hash() -> Option<String> {
    // `dist` is produced by `npm run build:ui`; fold its file hashes.
    // Resolve relative to the package dir (`src`), not the shell CWD.
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").ok()?);
    for dist in [
        manifest.join("static").join("dist"),
        manifest.join("..").join("dist"),
    ] {
        if !dist.exists() {
            continue;
        }
        let mut sums = CheckSums::new();
        read_folder_recursive(dist, &mut |path| {
            let _ = sums.add(&path);
        });
        let text = sums.to_plain_text();
        return Some(slu_utils::checksums::calculate_sha256(text.as_bytes()));
    }
    None
}

fn git_output(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
}

fn target_dir() -> PathBuf {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    // see <https://github.com/rust-lang/cargo/issues/5457>
    out_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn read_folder_recursive<F>(path: PathBuf, cb: &mut F)
where
    F: FnMut(PathBuf),
{
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            read_folder_recursive(path, cb);
        } else {
            cb(path);
        }
    }
}

fn sign_sha256sums(path: &PathBuf) {
    let key_base64 =
        std::env::var("TAURI_SIGNING_PRIVATE_KEY").expect("TAURI_SIGNING_PRIVATE_KEY missing");
    // Windows clears variables set to "", so treat missing as empty password.
    let password = std::env::var("TAURI_SIGNING_PRIVATE_KEY_PASSWORD").unwrap_or_default();

    let data = std::fs::read(path).expect("Failed to read SHA256SUMS file");
    let signature = sign_minisign(&data, &key_base64, password).expect("Failed to write signature");

    let sig_path = path.with_extension("sig");
    std::fs::write(&sig_path, signature).expect("Failed to write signature");
}

// ============================ semantic validation ============================

/// Parse every bundled widget/plugin resource with the runtime types
/// (`seelen_core::state::{Widget, Plugin}` via `SluResource::load_ext`,
/// which performs sanitize + validate). Returns one report line per file.
fn validate_resources_semantically() -> Vec<String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime for semantic validation");
    rt.block_on(async move {
        use seelen_core::{
            resource::SluResource,
            state::{Plugin, Widget},
        };

        let mut reports = Vec::new();
        let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let static_dir = manifest.join("static");

        // widgets: static/widgets/<id>/metadata.yml
        for entry in walk(std::fs::read_dir(static_dir.join("widgets")).ok()) {
            let report = match Widget::load_ext(&entry, true).await {
                Ok(w) => format!("SEMANTIC-OK widget {} ({})", w.id, entry.display()),
                Err(err) => format!(
                    "SEMANTIC-FAIL widget entry {} failed: {}",
                    entry.display(),
                    err
                ),
            };
            reports.push(report);
        }

        // plugins: static/plugins/<id>/metadata.yml plus flat *.yml files
        let plugins_dir = static_dir.join("plugins");
        for entry in walk(std::fs::read_dir(&plugins_dir).ok()) {
            if entry.is_dir() {
                let meta = entry.join("metadata.yml");
                let report = match Plugin::load_ext(&meta, true).await {
                    Ok(p) => format!("SEMANTIC-OK plugin {} ({})", p.id, meta.display()),
                    Err(err) => {
                        format!("SEMANTIC-FAIL plugin {} failed: {}", meta.display(), err)
                    }
                };
                reports.push(report);
            } else if matches!(
                entry.extension().and_then(|e| e.to_str()),
                Some("yml") | Some("yaml")
            ) {
                let report = match Plugin::load_ext(&entry, true).await {
                    Ok(p) => format!("SEMANTIC-OK plugin {} ({})", p.id, entry.display()),
                    Err(err) => {
                        format!("SEMANTIC-FAIL plugin {} failed: {}", entry.display(), err)
                    }
                };
                reports.push(report);
            }
        }

        fn walk(dir: Option<std::fs::ReadDir>) -> Vec<PathBuf> {
            dir.map(|entries| entries.flatten().map(|e| e.path()).collect())
                .unwrap_or_default()
        }

        reports
    })
}
