use std::{fs::create_dir, path::PathBuf};

use slu_utils::{checksums::CheckSums, signature::sign_minisign};

fn main() {
    let _ = create_dir("gen");

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

    println!("cargo:rustc-env=SUL_GIT_SHA={git_sha}");
    println!("cargo:rustc-env=SUL_GIT_DIRTY={dirty}");
    println!("cargo:rustc-env=SUL_BUILD_TIME={build_timestamp}");
    println!("cargo:rustc-env=SUL_TARGET={target}");
    println!(
        "cargo:rustc-env=SUL_STATIC_HASH={}",
        resource_hash.unwrap_or_else(|| "unknown".into())
    );
    println!(
        "cargo:rustc-env=SUL_FRONTEND_HASH={}",
        frontend_hash.unwrap_or_else(|| "unknown".into())
    );
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
    let signature = sign_minisign(&data, &key_base64, password).expect("Failed to sign data");

    let sig_path = path.with_extension("sig");
    std::fs::write(&sig_path, signature).expect("Failed to write signature");
}
