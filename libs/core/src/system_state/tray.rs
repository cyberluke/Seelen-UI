use std::path::PathBuf;

/// Identifier for a systray icon.
///
/// A systray icon is either identified by a (window handle + uid) or
/// its guid. Since a systray icon can be updated to also include a
/// guid or window handle/uid later on, a stable ID is useful for
/// consistently identifying an icon.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
pub enum SysTrayIconId {
    HandleUid(isize, u32),
    Guid(uuid::Uuid),
}

impl std::fmt::Display for SysTrayIconId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SysTrayIconId::HandleUid(handle, uid) => write!(f, "{:x}_{}", handle, uid),
            SysTrayIconId::Guid(guid) => write!(f, "{}", guid),
        }
    }
}

impl std::str::FromStr for SysTrayIconId {
    type Err = crate::error::SeelenLibError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try parsing as handle and uid (format: "handle:uid").
        if let Some((handle_str, uid_str)) = s.split_once(':') {
            return Ok(SysTrayIconId::HandleUid(
                handle_str.parse().map_err(|_| "Invalid icon id")?,
                uid_str.parse().map_err(|_| "Invalid icon id")?,
            ));
        }

        // Try parsing as a guid.
        if let Ok(guid) = uuid::Uuid::parse_str(s) {
            return Ok(SysTrayIconId::Guid(guid));
        }

        Err("Invalid icon id".into())
    }
}

/// Kind of a logical tray identity, indicating how it was resolved.
///
/// The variants are ordered by resolution priority (see
/// [`TrayLogicalIdentity::resolve`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum TrayIdentityKind {
    /// The application supplied a GUID; identity is `guid:<uuid>`.
    ///
    /// This is the strongest identity and is expected to survive process
    /// restarts exactly (same application, same logical icon).
    Guid,
    /// Canonical executable path + application-supplied UID.
    ///
    /// Stable across process restarts for well-behaved apps that keep
    /// the same `uid` value.
    ExeUid,
    /// Canonical executable path + normalized tooltip discriminator.
    ///
    /// Used when no `uid` or `guid` is available.
    ExeTooltip,
    /// Last-resort fallback identity.
    ///
    /// Used when none of the above can be derived reliably.
    Fallback,
}

/// Persistent, process-agnostic identity for a notification-area icon.
///
/// Different from [`SysTrayIconId`] in that `SysTrayIconId` (HWND/UID pair)
/// is only stable for the lifetime of a specific icon instance, whereas
/// `TrayLogicalIdentity` is designed to survive application restarts.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TrayLogicalIdentity {
    /// Canonical persisted key (`guid:…`, `exe:…|uid:…`, `exe:…|tip:…`
    /// or a `fallback:<n>` string). Persist this value.
    pub key: String,
    /// Category describing how `key` was derived.
    pub kind: TrayIdentityKind,
}

impl TrayLogicalIdentity {
    /// Resolve a logical identity from an existing runtime icon.
    ///
    /// Priority: `guid` > `exe|uid` > `exe|tooltip` > `fallback`.
    /// The `hwnd` itself is intentionally excluded so that a restarted
    /// application rebinds to its previous pin.
    pub fn resolve(icon: &SysTrayIcon) -> Self {
        if let Some(guid) = &icon.guid {
            return Self {
                key: format!("guid:{}", guid),
                kind: TrayIdentityKind::Guid,
            };
        }

        // Prefer the exe path; if the OS never returned one we still emit a
        // process-name-based identifier to avoid dropping the pin.
        let exe = icon
            .process_path
            .as_deref()
            .and_then(normalize_exe_path)
            .or_else(|| icon.process_name.clone());

        if let (Some(exe), Some(uid)) = (exe.as_ref(), icon.uid) {
            return Self {
                key: format!("exe:{}|uid:{}", exe, uid),
                kind: TrayIdentityKind::ExeUid,
            };
        }

        if let Some(exe) = exe.as_ref() {
            let tooltip = normalize_tooltip(&icon.tooltip);
            if !tooltip.is_empty() {
                return Self {
                    key: format!("exe:{}|tip:{}", exe, tooltip),
                    kind: TrayIdentityKind::ExeTooltip,
                };
            }
            // Only executable identity available.
            return Self {
                key: format!("exe:{}|no:0", exe),
                kind: TrayIdentityKind::ExeUid,
            };
        }

        // Fallback chain: tooltip alone, then guid-less index (kept stable
        // by the backend manager).
        let tooltip = normalize_tooltip(&icon.tooltip);
        if !tooltip.is_empty() {
            return Self {
                key: format!("tip:{}", tooltip),
                kind: TrayIdentityKind::Fallback,
            };
        }

        Self {
            key: match &icon.stable_id {
                SysTrayIconId::Guid(g) => format!("guid:{}", g),
                SysTrayIconId::HandleUid(_, uid) => format!("fallback:{}", uid),
            },
            kind: TrayIdentityKind::Fallback,
        }
    }
}

fn normalize_exe_path(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    Some(path.replace('\\', "/").to_lowercase())
}

fn normalize_tooltip(input: &str) -> String {
    input.trim().to_lowercase()
}

/// Runtime view of a tray icon, shaped for the semantic desktop API.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TrayIconInfo {
    /// Persistent logical identity key. Use this when calling pin/unpin/
    /// set-order APIs.
    pub logical_id: String,
    /// Runtime identifier (HWND+UID or GUID), used with
    /// `SendSystemTrayIconAction`.
    pub runtime_id: SysTrayIconId,
    pub tooltip: String,
    pub application_display_name: Option<String>,
    pub process_id: Option<u32>,
    pub process_path: Option<String>,
    pub process_name: Option<String>,
    pub app_user_model_id: Option<String>,
    /// 1-based position within the user-defined pinned order, or `None`
    /// when the icon is not pinned.
    pub order: Option<u32>,
    pub pinned: bool,
    /// Whether the icon is currently present in the tray.
    pub online: bool,
    pub icon_path: Option<PathBuf>,
    pub icon_image_hash: Option<String>,
}

/// Persisted tray pin state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct TrayPinState {
    /// Ordered list of pinned logical identity keys.
    #[serde(default)]
    pub order: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct SysTrayIcon {
    /// Identifier for the icon. Will not change for the lifetime of the
    /// icon.
    ///
    /// The Windows shell uses either a (window handle + uid) or its guid
    /// to identify which icon to operate on.
    ///
    /// Read more: https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-notifyicondataw
    pub stable_id: SysTrayIconId,

    /// Persistent, restart-stable identity, derived from `guid`, then the
    /// owning process path + uid/tooltip.
    pub logical_id: String,

    /// Application-defined identifier for the icon, used in combination
    /// with the window handle.
    ///
    /// The uid only has to be unique for the window handle. Multiple
    /// icons (across different window handles) can have the same uid.
    pub uid: Option<u32>,

    /// Handle to the window that contains the icon. Used in combination
    /// with a uid.
    ///
    /// Note that multiple icons can have the same window handle.
    pub window_handle: Option<isize>,

    /// GUID for the icon.
    ///
    /// Used as an alternate way to identify the icon (versus its window
    /// handle and uid).
    pub guid: Option<uuid::Uuid>,

    /// Tooltip to show for the icon on hover.
    pub tooltip: String,

    /// Handle to the icon bitmap.
    pub icon_handle: Option<isize>,

    /// Path to the icon image file.
    pub icon_path: Option<PathBuf>,

    /// Hash of the icon image.
    ///
    /// Used to determine if the icon image has changed without having to
    /// compare the entire image.
    pub icon_image_hash: Option<String>,

    /// Application-defined message identifier.
    ///
    /// Used to send messages to the window that contains the icon.
    pub callback_message: Option<u32>,

    /// Version of the icon.
    pub version: Option<u32>,

    /// Whether the icon is visible in the system tray.
    ///
    /// This is determined by the `NIS_HIDDEN` flag in the icon's state.
    pub is_visible: bool,

    /// Owning process id (resolved from `window_handle`).
    pub process_id: Option<u32>,

    /// Canonical executable path of the owning process.
    pub process_path: Option<String>,

    /// Program executable name (e.g. `WhatsApp.exe`).
    pub process_name: Option<String>,

    /// Set by the application via `SetCurrentProcessExplicitAppUserModelID`.
    pub app_user_model_id: Option<String>,

    /// Application display name (AppUserModelID metadata or fallback).
    pub application_display_name: Option<String>,
}

/// Actions that can be performed on a `SystrayIcon`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(repr(enum = name)))]
pub enum SystrayIconAction {
    HoverEnter,
    HoverLeave,
    HoverMove,
    LeftClick,
    RightClick,
    MiddleClick,
    LeftDoubleClick,
}
