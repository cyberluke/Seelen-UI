use seelen_core::system_state::UserAppWindow;

use super::application::WegPersistedState;

/// Stable logical identity of a managed window.
#[derive(Debug, Clone)]
pub struct LogicalWindowIdentity {
    /// normalized application key (umid or exe basename, lowercase)
    pub application: String,
    /// logical local id inside the application group
    pub local_id: String,
    /// normalized title used for display
    pub display_title: String,
    /// user alias when defined
    pub alias: Option<String>,
}

impl LogicalWindowIdentity {
    pub fn logical_identity(&self) -> String {
        format!("{}:{}", self.application, self.local_id)
    }
}

/// Title segments that carry no project meaning and are skipped by parsers.
const SKIP_SEGMENTS: &[&str] = &[
    "visual studio code",
    "visual studio code - insiders",
    "microsoft visual studio code",
    "microsoft visual studio code - insiders",
    "insiders",
    "microsoft edge",
    "microsoft edge beta",
    "microsoft edge dev",
    "microsoft edge canary",
    "microsoft edge for business",
    "windows terminal",
    "terminal",
    "nushell",
    "command prompt",
    "powershell",
    "pwsh",
];

/// A tiny application identity provider table. Each entry matches the
/// normalized exe/umid prefix and explains which parsing strategy to use.
#[derive(Debug)]
struct IdentityRule {
    /// exe basenames (lowercase) handled by this rule
    exe_prefixes: &'static [&'static str],
    /// umids (compared case-insensitively) handled by this rule
    umids: &'static [&'static str],
    /// normalized application key produced when the exe path identifies insiders/other
    insiders_key: Option<&'static str>,
}

const IDENTITY_RULES: &[IdentityRule] = &[
    IdentityRule {
        exe_prefixes: &["code", "code - insiders"],
        umids: &[
            "microsoft.visualstudiocode",
            "microsoft.visualstudiocode.insiders",
        ],
        insiders_key: Some("vscode-insiders"),
    },
    IdentityRule {
        exe_prefixes: &["msedge"],
        umids: &["microsoft.edge", "microsoft.edge.beta"],
        insiders_key: Some("msedge-beta"),
    },
    IdentityRule {
        exe_prefixes: &["wt"],
        umids: &["microsoft.windowsterminal", "microsoft.windowsterminal"],
        insiders_key: Some("wt-canary"),
    },
];

/// Normalized application key: umid first, then the exe basename.
fn application_key(win: &UserAppWindow) -> String {
    if let Some(umid) = &win.umid {
        return umid.to_lowercase();
    }
    win.process
        .path
        .as_ref()
        .and_then(|p| p.file_stem())
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| win.app_name.to_lowercase())
}

/// Stable, application-aware logical id. Never HWND-based so the identity
/// survives recreation (the first matching segment of the title is stable).
fn logical_local_id(win: &UserAppWindow, app_key: &str) -> String {
    let title = win.title.trim();
    if !title.is_empty() {
        let segments: Vec<&str> = title
            .split(" - ")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        let mut project: Option<String> = None;
        for seg in segments.iter() {
            let lower = seg.to_lowercase();
            if SKIP_SEGMENTS.contains(&lower.as_str()) {
                continue;
            }
            // skip numeric column markers like "1:" or paths (contain drive or slashes)
            let is_path =
                seg.contains(':') && !seg.ends_with(':') || seg.contains('\\') || seg.contains('/');
            if seg.chars().all(|c| c.is_ascii_digit()) || is_path {
                continue;
            }
            project = Some(seg.to_string());
            break;
        }

        // vscode-aware: prefer the workspace name segment (first meaningful),
        // otherwise fall back to the whole title.
        let _ = project.is_none() || app_key == "__any__";
        if let Some(p) = project {
            return p;
        }
        return title.to_string();
    }
    format!("win-{:x}", win.hwnd)
}

/// Resolve the stable identity for one managed window.
pub fn identity_for(win: &UserAppWindow, persisted: &WegPersistedState) -> LogicalWindowIdentity {
    let raw_app = application_key(win);

    // find matching table rule
    let rule = IDENTITY_RULES.iter().find(|r| {
        let stem = raw_app.trim_end_matches(".exe");
        r.exe_prefixes.contains(&stem)
            || r.umids.iter().any(|u| {
                raw_app == *u || raw_app.starts_with(u) && raw_app[u.len()..].starts_with('.')
            })
    });

    let application = match rule {
        Some(rule) => {
            let mut key = raw_app.clone();
            // prefer the table key (normalized) over the raw umid casing
            if let Some(first) = rule.exe_prefixes.first() {
                key = first.to_string();
            } else if let Some(first) = rule.umids.first() {
                key = first.to_string();
            }
            if let Some(insiders) = rule.insiders_key
                && let Some(path) = &win.process.path
                && path.to_string_lossy().to_lowercase().contains("insiders")
            {
                key = insiders.to_string();
            }
            key
        }
        None => raw_app,
    };

    let local_id = logical_local_id(win, &application);
    let full = format!("{application}:{local_id}");
    let alias = persisted.aliases.get(&full).cloned();
    let display_title = alias
        .clone()
        .or_else(|| (!local_id.is_empty()).then(|| local_id.clone()))
        .unwrap_or_else(|| win.title.clone());

    LogicalWindowIdentity {
        application,
        local_id,
        display_title,
        alias,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use seelen_core::system_state::{ProcessInformation, Relaunch};
    use std::path::PathBuf;

    fn sample(umid: Option<&str>, path: Option<&str>, title: &str) -> UserAppWindow {
        UserAppWindow {
            hwnd: 1,
            monitor: seelen_core::system_state::MonitorId::from("x"),
            title: title.to_string(),
            app_name: String::new(),
            is_zoomed: false,
            is_iconic: false,
            is_fullscreen: false,
            umid: umid.map(|u| u.to_string()),
            process: ProcessInformation {
                id: 0,
                path: path.map(PathBuf::from),
            },
            prevent_pinning: false,
            relaunch: Some(Relaunch {
                command: String::new(),
                args: None,
                working_dir: None,
                icon: None,
            }),
            rect: None,
            last_foreground_at: 0,
        }
    }

    #[test]
    fn vscode_workspace_identity_is_project_name() {
        let persisted = WegPersistedState::default();
        let win = sample(
            None,
            Some("C:\\Apps\\Microsoft VS Code\\Code.exe"),
            "TaskQoS - d:\\repos\\taskqos - Visual Studio Code",
        );
        let ident = identity_for(&win, &persisted);
        assert_eq!(ident.application, "code");
        assert_eq!(ident.local_id, "TaskQoS");
        assert_eq!(ident.logical_identity(), "code:TaskQoS");
    }

    #[test]
    fn insiders_uses_distinct_key() {
        let persisted = WegPersistedState::default();
        let win = sample(
            None,
            Some("C:\\Apps\\Microsoft VS Code\\Code - Insiders.exe"),
            "XeOm - Visual Studio Code - Insiders",
        );
        let ident = identity_for(&win, &persisted);
        assert_eq!(ident.application, "vscode-insiders");
        assert_eq!(ident.local_id, "XeOm");
    }

    #[test]
    fn generic_fallback_uses_title() {
        let persisted = WegPersistedState::default();
        let win = sample(
            None,
            Some("C:\\Windows\\notepad.exe"),
            "notes.txt - Notepad",
        );
        let ident = identity_for(&win, &persisted);
        assert_eq!(ident.application, "notepad");
        assert_eq!(ident.local_id, "notes.txt");
    }
}
