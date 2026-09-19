use seelen_core::system_state::UserAppWindow;

use super::application::WegPersistedState;

/// Stable logical identity of a managed window.
#[derive(Debug, Clone)]
pub struct LogicalWindowIdentity {
    /// normalized application key (provider table key, umid or exe basename, lowercase)
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

/// Which identity strategy a window belongs to. One provider per application
/// family instead of one generic parser pretending to fit every app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityProvider {
    VsCode,
    Terminal,
    /// the Edge family (stable/beta/dev/canary share the slot strategy)
    Edge,
    Generic,
}

/// The lowercase executable stem of a window (without `.exe`), if known.
fn exe_stem(win: &UserAppWindow) -> Option<String> {
    win.process
        .path
        .as_ref()
        .and_then(|p| p.file_stem())
        .map(|s| s.to_string_lossy().to_lowercase())
}

/// Normalized umid for matching ("Microsoft Edge" -> "microsoft.edge").
fn normalized_umid(win: &UserAppWindow) -> Option<String> {
    win.umid
        .as_ref()
        .map(|u| u.to_lowercase().replace(' ', "."))
}

/// Classify a window into exactly one identity provider.
pub fn provider_for(win: &UserAppWindow) -> IdentityProvider {
    let stem = exe_stem(win).unwrap_or_default();
    let umid = normalized_umid(win).unwrap_or_default();

    if stem.starts_with("msedge") || stem == "edge" || umid.starts_with("microsoft.edge") {
        return IdentityProvider::Edge;
    }
    if matches!(stem.as_str(), "code" | "code - insiders")
        || umid.starts_with("microsoft.visualstudiocode")
    {
        return IdentityProvider::VsCode;
    }
    if stem == "wt" || umid.starts_with("microsoft.windowsterminal") {
        return IdentityProvider::Terminal;
    }
    IdentityProvider::Generic
}

/// Resolve the deterministic application key (and, for Edge, the channel).
fn application_key(win: &UserAppWindow) -> String {
    let stem = exe_stem(win).unwrap_or_default();
    let umid = normalized_umid(win).unwrap_or_default();
    let raw_path = win
        .process
        .path
        .as_ref()
        .map(|p| p.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match provider_for(win) {
        IdentityProvider::VsCode => {
            let insiders = stem == "code - insiders"
                || umid.contains(".insiders")
                || raw_path.contains("insiders");
            if insiders {
                "vscode-insiders".to_string()
            } else {
                "code".to_string()
            }
        }
        IdentityProvider::Terminal => {
            if umid.ends_with(".canary") || raw_path.contains("canary") {
                "wt-canary".to_string()
            } else {
                "wt".to_string()
            }
        }
        IdentityProvider::Edge => edge_channel_key(&stem, &umid, &raw_path),
        IdentityProvider::Generic => {
            if !umid.is_empty() {
                umid
            } else if !stem.is_empty() {
                stem
            } else {
                win.app_name.to_lowercase()
            }
        }
    }
}

/// Map the real Edge product metadata (exe name / umid / install path) to the
/// channel-specific application key. No `insiders` condition is used for the
/// browser channels.
fn edge_channel_key(stem: &str, umid: &str, raw_path: &str) -> String {
    if stem == "msedge_beta"
        || stem.ends_with("_beta")
        || umid.ends_with(".beta")
        || raw_path.contains("edge beta")
    {
        "edge-beta".to_string()
    } else if stem == "msedge_dev"
        || stem.ends_with("_dev")
        || umid.ends_with(".dev")
        || raw_path.contains("edge dev")
    {
        "edge-dev".to_string()
    } else if stem == "msedge_canary"
        || stem.ends_with("_canary")
        || umid.ends_with(".canary")
        || raw_path.contains("edge canary")
    {
        "edge-canary".to_string()
    } else {
        "edge-stable".to_string()
    }
}

/// Generic title parser shared by the VsCode / Terminal / Generic providers.
/// For these applications the first meaningful title segment is a stable
/// project / document name.
fn logical_local_id(win: &UserAppWindow) -> String {
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

        if let Some(p) = project {
            return p;
        }
        return title.to_string();
    }
    format!("win-{:x}", win.hwnd)
}

// ============================ resolution ============================

/// Resolve the deterministic application key for one window (public wrapper).
pub fn application_key_for(win: &UserAppWindow) -> String {
    application_key(win)
}

/// Resolve the stable identity for one managed window.
///
/// Browsers (Edge) use the persisted slot table as `local_id`, because the
/// active tab title is display/content state, not identity. Every other
/// provider uses the application-aware title parser.
pub fn identity_for(win: &UserAppWindow, persisted: &WegPersistedState) -> LogicalWindowIdentity {
    let application = application_key(win);
    let provider = provider_for(win);
    let local_id = match provider {
        IdentityProvider::Edge => edge_slot(win, &application, persisted),
        _ => logical_local_id(win),
    };
    let full = format!("{application}:{local_id}");
    let alias = persisted.aliases.get(&full).cloned();
    let display_title = alias.clone().unwrap_or_else(|| match provider {
        // Slot ids are positional labels, not content: the display keeps the
        // active content title (the app-normalized title segment).
        IdentityProvider::Edge => {
            let content = logical_local_id(win);
            if content.is_empty() {
                win.title.clone()
            } else {
                content
            }
        }
        _ => {
            if local_id.is_empty() {
                win.title.clone()
            } else {
                local_id.clone()
            }
        }
    });

    LogicalWindowIdentity {
        application,
        local_id,
        display_title,
        alias,
    }
}

/// Geometry/monitor fingerprint used for deterministic slot reclamation after
/// an application restart.
pub fn slot_fingerprint(win: &UserAppWindow) -> String {
    match &win.rect {
        Some(rect) => format!(
            "{}:{}x{}",
            win.monitor,
            rect.right - rect.left,
            rect.bottom - rect.top
        ),
        None => format!("{}:no-rect", win.monitor),
    }
}

/// Reclaim or assign the persistent window slot for one browser window.
///
/// Matching first by the saved geometry/monitor fingerprint keeps the same
/// logical slot across restarts; new windows take the smallest unused slot
/// number, in creation order.
pub fn edge_slot(win: &UserAppWindow, app_key: &str, persisted: &WegPersistedState) -> String {
    let fingerprint = slot_fingerprint(win);
    let table = persisted.slots.get(app_key);
    if let Some(record) = table.and_then(|records| {
        records
            .iter()
            .find(|r| r.fingerprint == fingerprint)
            .or_else(|| {
                records
                    .iter()
                    .find(|r| r.fingerprint.starts_with(win.monitor.as_str()))
            })
    }) {
        return record.name.clone();
    }

    let mut used: Vec<u32> = table
        .map(|records| {
            records
                .iter()
                .filter_map(|r| r.name.strip_prefix("window-")?.parse::<u32>().ok())
                .collect()
        })
        .unwrap_or_default();
    used.sort_unstable();
    let mut next = 1u32;
    for n in &used {
        if *n == next {
            next += 1;
        }
    }
    format!("window-{next}")
}

// Slot persistence lives in `application::save_slots` (called from
// `window_entries`); this module only reads the persisted table.

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

    #[test]
    fn edge_slot_identity_is_independent_from_tab_title() {
        let persisted = WegPersistedState::default();
        let win = sample(
            Some("Microsoft Edge"),
            Some("C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe"),
            "LinkedIn - Microsoft Edge",
        );
        let ident = identity_for(&win, &persisted);
        assert_eq!(ident.application, "edge-stable");
        assert_eq!(ident.local_id, "window-1");
        assert_eq!(ident.display_title, "LinkedIn");
    }

    #[test]
    fn edge_beta_channel_key() {
        let persisted = WegPersistedState::default();
        let win = sample(
            Some("Microsoft Edge Beta"),
            Some("C:\\Program Files (x86)\\Microsoft\\Edge Beta\\Application\\msedge_beta.exe"),
            "GitHub - Microsoft Edge Beta",
        );
        let ident = identity_for(&win, &persisted);
        assert_eq!(ident.application, "edge-beta");
    }
}
