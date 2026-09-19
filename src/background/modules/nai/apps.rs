//! NAI app registry: the capability-first launcher table for the OS forks.
//!
//! One mechanism installs/launches every participant: exe binaries, PowerShell
//! scripts, compose stacks and doc-only workspaces. Resolution is deterministic
//! against `D:\_SATIN_AI\*`, so every surface (GUI, `slu`, REST, MCP) sees the
//! same list without scraping the UI.

use serde::Serialize;

use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AppKind {
    Exe,
    Script,
    Compose,
    Docs,
    Shell,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppDescriptor {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub kind: AppKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub capabilities: &'static [&'static str],
    pub running_pids: Vec<u32>,
}

#[derive(Clone, Copy)]
struct Entry {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    kind: AppKind,
    capabilities: &'static [&'static str],
    /// candidate absolute paths, first existing wins
    candidates: &'static [&'static str],
}

const ENTRIES: &[Entry] = &[
    Entry {
        id: "browser",
        name: "NAI Browser",
        description: "BrowserOS: human-driven AI browser",
        kind: AppKind::Exe,
        capabilities: &["browser.open"],
        candidates: &[
            r"D:\_SATIN_AI\BrowserOS\packages\browseros\out\BrowserOS.exe",
            r"C:\Program Files\BrowserOS\BrowserOS.exe",
        ],
    },
    Entry {
        id: "browser-neo",
        name: "NAI Browser neo",
        description: "BrowserOS neo: agent browser with persistent logins, MCP and session replay",
        kind: AppKind::Exe,
        capabilities: &["browser.capture", "browser.navigate", "mcp.neo"],
        candidates: &[
            r"D:\_SATIN_AI\BrowserOS\packages\browseros-agent\out\no\Neo.exe",
            r"C:\Program Files\BrowserOS\Neo.exe",
        ],
    },
    Entry {
        id: "email",
        name: "NAI E-Mail",
        description: "Velo: local-first Tauri mail, search, calendar",
        kind: AppKind::Exe,
        capabilities: &["mail.read", "mail.search", "calendar.read"],
        candidates: &[
            r"D:\_SATIN_AI\velo\src-tauri\target\release\velo.exe",
            r"D:\_SATIN_AI\velo\target\release\velo.exe",
        ],
    },
    Entry {
        id: "office",
        name: "NAI Office",
        description: "LibreOffice over UNO with python bridge",
        kind: AppKind::Exe,
        capabilities: &["document.read", "document.write", "document.convert"],
        candidates: &[
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
        ],
    },
    Entry {
        id: "voice",
        name: "NAI Voice",
        description: "Toastovac/Jarvis: local voice kernel, OpenVINO NPU pipeline",
        kind: AppKind::Exe,
        capabilities: &["voice.transcribe"],
        candidates: &[r"D:\_SATIN_AI\Toastovac\jarvis\dist\Jarvis\Jarvis.exe"],
    },
    Entry {
        id: "memory",
        name: "NAI Psyche",
        description: "NeuralAccessPsyche: llamacpp models + Qdrant memory plane",
        kind: AppKind::Exe,
        capabilities: &["memory.write", "memory.search"],
        candidates: &[r"D:\_SATIN_AI\NeuralAccessPsyche-llamacpp\.venv\Scripts\python.exe"],
    },
    Entry {
        id: "search",
        name: "NAI Search",
        description: "searxng: local search fabric (docker compose)",
        kind: AppKind::Compose,
        capabilities: &["search.hybrid"],
        candidates: &[r"D:\_SATIN_AI\searxng\docker-compose.yml"],
    },
    Entry {
        id: "qos",
        name: "NAI QoS",
        description: "TaskQoS: Windows scheduling hot path",
        kind: AppKind::Exe,
        capabilities: &["qos.apply"],
        candidates: &[r"D:\_SATIN_AI\TaskQoS\target\release\hotpath-qos.exe"],
    },
    Entry {
        id: "governor",
        name: "NAI Governor",
        description: "Workstation Governor: CPUset / power plan control",
        kind: AppKind::Script,
        capabilities: &["system.govern"],
        candidates: &[r"D:\_SATIN_AI\Governor\Governor.ps1"],
    },
    Entry {
        id: "xeom",
        name: "XeOm",
        description: "XeOm data/graph surface",
        kind: AppKind::Exe,
        capabilities: &["data.graph"],
        candidates: &[r"D:\_SATIN_AI\XeOm\xeom.exe"],
    },
    Entry {
        id: "workstation",
        name: "NAI Workstation",
        description: "Kelvin workstation docs / plans",
        kind: AppKind::Docs,
        capabilities: &[],
        candidates: &[r"D:\_SATIN_AI\KelvinAIWorkstation"],
    },
    Entry {
        id: "harness",
        name: "NAI Workstation Harness",
        description: "Kelvin harness docs / plans",
        kind: AppKind::Docs,
        capabilities: &[],
        candidates: &[r"D:\_SATIN_AI\KelvinAIWorkstationHarness"],
    },
];

fn resolve(entry: &Entry) -> Option<String> {
    if entry.kind == AppKind::Shell {
        return std::env::current_exe()
            .ok()
            .and_then(|p| p.to_str().map(String::from));
    }
    entry.candidates.iter().find_map(|candidate| {
        let expanded = if let Some(rest) = candidate.strip_prefix("%LOCALAPPDATA%") {
            std::env::var("LOCALAPPDATA")
                .ok()
                .map(|base| format!(r"{base}{rest}"))
        } else {
            Some((*candidate).to_string())
        }?;
        std::path::Path::new(&expanded).exists().then_some(expanded)
    })
}

fn running_pids(path: Option<&str>, sys: &sysinfo::System) -> Vec<u32> {
    let Some(needle) = path.map(|p| {
        std::path::Path::new(p)
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    }) else {
        return Vec::new();
    };
    if needle.is_empty() {
        return Vec::new();
    }

    sys.processes()
        .values()
        .filter_map(|process| {
            let name = process.name().to_string_lossy().to_lowercase();
            (name == needle).then(|| process.pid().as_u32())
        })
        .collect()
}

pub fn apps() -> Vec<AppDescriptor> {
    use sysinfo::{ProcessesToUpdate, System};

    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    std::iter::once(Entry {
        id: "shell",
        name: "NAI OS",
        description: "This shell (semantic desktop kernel + launcher)",
        kind: AppKind::Shell,
        capabilities: &["window.focus", "shell.settings"],
        candidates: &[],
    })
    .chain(ENTRIES.iter().copied())
    .map(|entry| {
        let path = resolve(&entry);
        AppDescriptor {
            id: entry.id,
            name: entry.name,
            description: entry.description,
            kind: entry.kind,
            capabilities: entry.capabilities,
            running_pids: running_pids(path.as_deref(), &sys),
            path,
        }
    })
    .collect()
}

/// Launch one registry entry. Deterministic: only the first existing candidate
/// is used; missing targets return an error instead of a guess.
pub fn launch(id: &str) -> Result<serde_json::Value> {
    let entry = ENTRIES
        .iter()
        .find(|e| e.id.eq_ignore_ascii_case(id))
        .or_else(|| ENTRIES.iter().find(|e| e.name.eq_ignore_ascii_case(id)))
        .or(if id.eq_ignore_ascii_case("shell") {
            Some(&Entry {
                id: "shell",
                name: "NAI OS",
                description: "This shell",
                kind: AppKind::Shell,
                capabilities: &[],
                candidates: &[],
            })
        } else {
            None
        })
        .ok_or_else(|| format!("unknown app id: {id}"))?;

    let path = resolve(entry).ok_or_else(|| format!("target missing for app: {id}"))?;
    let mut command = match entry.kind {
        AppKind::Exe | AppKind::Shell => std::process::Command::new(&path),
        AppKind::Script => {
            let mut cmd = std::process::Command::new("pwsh");
            cmd.args(["-NoProfile", "-File", &path]);
            cmd
        }
        AppKind::Compose => {
            let mut cmd = std::process::Command::new("docker");
            cmd.current_dir(
                std::path::Path::new(&path)
                    .parent()
                    .unwrap_or(std::path::Path::new(r"D:\_SATIN_AI")),
            );
            cmd.args(["compose", "-f", &path, "up", "-d"]);
            cmd
        }
        AppKind::Docs => {
            return Ok(serde_json::json!({ "id": entry.id, "opened": path }));
        }
    };
    let pid = command
        .spawn()
        .map_err(|err| format!("spawn failed for {id}: {err}"))?
        .id();
    Ok(serde_json::json!({ "id": entry.id, "path": path, "pid": pid }))
}
