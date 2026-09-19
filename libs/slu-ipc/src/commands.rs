use std::path::PathBuf;

use seelen_core::resource::ResourceKind;
use serde::{Deserialize, Serialize};

// ===== Execution mode =====

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandExecutionMode {
    Direct,
    MainInstance,
}

pub trait SluCliCommand {
    fn execution_mode(&self) -> CommandExecutionMode {
        CommandExecutionMode::MainInstance
    }
}

// ===== Top-level =====

/// Seelen UI Command Line Interface
#[derive(Debug, clap::Parser)]
#[command(version, name = "Seelen UI")]
pub struct AppCli {
    /// Prints some extra information on the console.
    #[arg(long, default_value_t)]
    pub verbose: bool,
    /// Renders command output as raw json (otherwise pretty human layout).
    #[arg(long, default_value_t)]
    pub json: bool,
    /// Boot pipeline flight recorder of the queried instance.
    #[arg(long)]
    pub boot: bool,
    /// Instance count of the queried session.
    #[arg(long)]
    pub instances: bool,
    #[command(subcommand)]
    pub command: Option<AppCommand>,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum AppCommand {
    /// Opens the Seelen settings gui.
    Settings,
    VirtualDesk(VirtualDesktopCli),
    Debugger(DebuggerCli),
    WindowManager(WindowManagerCli),
    Popup(PopupsCli),
    Weg(WegCli),
    Widget(WidgetCli),
    Resource(ResourceManagerCli),
    Art(ArtCli),
    TaskSwitcher(TaskSwitcherClient),
    Wallpaper(WallpaperCli),
    /// Toggle the global shortcuts pause state
    ToggleShortcutsPause,
    /// System tray semantic API
    Tray(TrayCli),
    /// Runtime provenance / instance diagnostics
    Runtime(RuntimeCli),
    /// NAI semantic desktop kernel surface
    Nai(NaiCli),
}

impl SluCliCommand for AppCommand {
    fn execution_mode(&self) -> CommandExecutionMode {
        match self {
            AppCommand::Art(_) => CommandExecutionMode::Direct,
            AppCommand::Resource(r) => r.execution_mode(),
            _ => CommandExecutionMode::MainInstance,
        }
    }
}

// ===== Runtime =====

/// Runtime diagnostics
#[derive(Debug, Serialize, Deserialize, clap::Args)]
pub struct RuntimeCli {
    #[command(subcommand)]
    pub subcommand: RuntimeCommand,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum RuntimeCommand {
    /// GUI PID, service PID, session id, mutex ownership, HTTP port owner
    Instance,
    /// Build provenance: git sha, profile, tauri mode, bundle hashes
    Provenance,
}

// ===== Debugger =====

/// Debugger cli
#[derive(Debug, Serialize, Deserialize, clap::Args)]
pub struct DebuggerCli {
    #[command(subcommand)]
    pub subcommand: DebuggerSubCommand,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum DebuggerSubCommand {
    /// Toggles the tracing of window events
    ToggleWinEvents,
}

// ===== Art =====

#[derive(Debug, Clone, Copy, Serialize, Deserialize, clap::ValueEnum)]
pub enum ArtVariant {
    SeelenLogo,
    SeelenLogoSmall,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
pub struct ArtCli {
    pub variant: ArtVariant,
}

// ===== Resource =====

/// Manage the Seelen Resources.
#[derive(Debug, Serialize, Deserialize, clap::Args)]
pub struct ResourceManagerCli {
    #[command(subcommand)]
    pub subcommand: ResourceSubCommand,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum ResourceSubCommand {
    /// loads a widget into the internal registry
    Load {
        kind: ClapResourceKind,
        path: PathBuf,
    },
    /// deletes the widget from internal registry
    Unload {
        kind: ClapResourceKind,
        path: PathBuf,
    },
    /// Bundles a widget into a single file to be shared.
    ///
    /// Exported file will be at the same location as the passed path
    /// with a filename `export_{date}.yml`.
    Bundle {
        kind: ClapResourceKind,
        path: PathBuf,
    },
    /// Translates a resource text file to all the supported languages by Seelen UI
    /// this file should contain the source language key and value in order to be translated.
    ///
    /// Example:
    /// ```yaml
    /// # The file will be completed with the rest of the supported languages
    /// en: Some text to be translated
    /// ```
    Translate {
        /// The file to be translated
        path: PathBuf,
        /// The source language of the file, by default `en`
        source_lang: Option<String>,
    },
}

impl SluCliCommand for ResourceSubCommand {
    fn execution_mode(&self) -> CommandExecutionMode {
        match self {
            ResourceSubCommand::Bundle { .. } => CommandExecutionMode::Direct,
            ResourceSubCommand::Translate { .. } => CommandExecutionMode::Direct,
            ResourceSubCommand::Load { .. } | ResourceSubCommand::Unload { .. } => {
                CommandExecutionMode::MainInstance
            }
        }
    }
}

impl SluCliCommand for ResourceManagerCli {
    fn execution_mode(&self) -> CommandExecutionMode {
        self.subcommand.execution_mode()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
pub enum ClapResourceKind {
    Theme,
    Widget,
    Plugin,
    IconPack,
    Wallpaper,
    SoundPack,
}

impl From<ClapResourceKind> for ResourceKind {
    fn from(value: ClapResourceKind) -> Self {
        match value {
            ClapResourceKind::Theme => ResourceKind::Theme,
            ClapResourceKind::IconPack => ResourceKind::IconPack,
            ClapResourceKind::Widget => ResourceKind::Widget,
            ClapResourceKind::Plugin => ResourceKind::Plugin,
            ClapResourceKind::Wallpaper => ResourceKind::Wallpaper,
            ClapResourceKind::SoundPack => ResourceKind::SoundPack,
        }
    }
}

// ===== VirtualDesktop =====

/// Manage the Seelen Window Manager.
#[derive(Debug, Serialize, Deserialize, clap::Args)]
#[command(alias = "vd")]
pub struct VirtualDesktopCli {
    #[command(subcommand)]
    pub subcommand: VdCommand,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum VdCommand {
    /// Switch to a neighbor workspace
    SwitchTo { direction: DirectionOrIndex },
    /// Send the active window to a neighbor workspace
    SendTo { direction: DirectionOrIndex },
    /// Move to a neighbor workspace and switch to it
    MoveTo { direction: DirectionOrIndex },
    /// Create a new workspace column
    CreateNewWorkspace,
    /// Create a new workspace row
    CreateNewWorkspaceRow,
    /// Destroy the current workspace column or row if there is only one column
    DestroyCurrentWorkspace,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DirectionOrIndex {
    Direction(Direction),
    Index(usize),
}

impl std::str::FromStr for DirectionOrIndex {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Ok(n) = s.parse::<usize>() {
            Ok(Self::Index(n))
        } else {
            Ok(match s.to_lowercase().as_str() {
                "left" => Self::Direction(Direction::Left),
                "right" => Self::Direction(Direction::Right),
                "up" => Self::Direction(Direction::Up),
                "down" => Self::Direction(Direction::Down),
                _ => return Err("Invalid direction or index".to_string()),
            })
        }
    }
}

// ===== Widget =====

#[derive(Debug, Serialize, Deserialize, clap::Args)]
pub struct WidgetCli {
    #[command(subcommand)]
    pub command: WidgetCommand,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum WidgetCommand {
    /// Triggers a widget
    Trigger { widget_id: String },
    /// List all widget instances with status, HWND and monitor/instance ids
    List,
    /// Print the boot pipeline flight recorder (all pods or one widget)
    Boot {
        /// widget id (e.g. @seelen/weg); omit for the global overview
        widget_id: Option<String>,
    },
    /// Open DevTools for a widget (by widget id)
    Devtools {
        /// widget id (e.g. @seelen/weg)
        widget_id: String,
    },
}

// ===== NAI semantic kernel =====

/// NAI OS semantic desktop commands
#[derive(Debug, Serialize, Deserialize, clap::Args)]
pub struct NaiCli {
    #[command(subcommand)]
    pub subcommand: NaiCommand,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum NaiCommand {
    /// Snapshot of the semantic desktop graph (windows, workspaces, monitors)
    Graph,
    /// Capability registry (id, risk, provider, latency class)
    Capabilities,
    /// Activate an object by logical identity or alias
    Activate {
        /// identification token
        identification: String,
    },
    /// Undo the last reversible NAI action
    Undo,
    /// Flight-recorder trace
    Trace,
}

// ===== Popups =====

/// Manage the Seelen Popups.
#[derive(Debug, Serialize, Deserialize, clap::Args)]
pub struct PopupsCli {
    #[command(subcommand)]
    pub subcommand: PopupsCommand,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum PopupsCommand {
    Create {
        /// json config
        config: String,
    },
    Update {
        /// id
        id: String,
        /// json config
        config: String,
    },
    Close {
        /// id
        id: String,
    },
    #[command(hide = true)]
    InternalSetShortcut { json: String },
}

// ===== Weg =====

/// Seelen's dock commands
#[derive(Debug, Serialize, Deserialize, clap::Args)]
pub struct WegCli {
    #[command(subcommand)]
    pub subcommand: WegCommand,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum WegCommand {
    /// Set foreground to the application which is idx-nth on the weg. If it is not started, then starts it.
    ForegroundOrRunApp {
        /// Which index should be started on weg.
        index: usize,
    },
    /// List all managed windows with stable ids
    Windows,
    /// List grouped applications with window counts
    Apps,
    /// Get one window by runtime id, logical identity, alias or title
    Get {
        /// identification token
        identification: String,
    },
    /// Fuzzy-match windows by identity / title / alias
    Find {
        /// search query
        query: String,
    },
    /// Focus a window
    Focus {
        /// identification token
        identification: String,
    },
    /// Focus and maximize a window atomically
    #[command(alias = "fm")]
    FocusMaximize {
        /// identification token
        identification: String,
    },
    /// Maximize a window
    Maximize {
        /// identification token
        identification: String,
    },
    /// Restore a window from maximized
    Restore {
        /// identification token
        identification: String,
    },
    /// Minimize a window
    Minimize {
        /// identification token
        identification: String,
    },
    /// Close a window
    Close {
        /// identification token
        identification: String,
    },
    /// Move a window to the n-th monitor (0 based)
    MoveToMonitor {
        /// identification token
        identification: String,
        /// monitor index
        monitor: u32,
    },
    /// List the manual order of an application group
    OrderList {
        /// application key
        app: String,
    },
    /// Move an identity to a position inside the manual order
    OrderMove {
        /// application key
        app: String,
        /// logical identity / alias / title
        identity: String,
        /// target position
        to: usize,
    },
    /// Recently focused windows
    Recent {
        /// max entries
        limit: Option<u32>,
    },
    /// Unified runtime state in camelCase: version, nextId, latency stats,
    /// focused / monitored / monitors and perMonitor[] grouped by index
    State,
    /// Latency metrics of the control plane (p50/p95/p99/max)
    Metrics,
    /// Dump the flight recorder trace
    Trace,
}

// ===== WindowManager =====

#[derive(Debug, Clone, Copy, Serialize, Deserialize, clap::ValueEnum)]
pub enum AllowedReservations {
    Left,
    Right,
    Top,
    Bottom,
    Stack,
    Float,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum Sizing {
    Increase,
    Decrease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum StepWay {
    Next,
    Prev,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum Axis {
    Horizontal,
    Vertical,
    Top,
    Bottom,
    Left,
    Right,
}

/// Manage the Seelen Window Manager.
#[derive(Debug, Serialize, Deserialize, clap::Args)]
#[command(alias = "wm")]
pub struct WindowManagerCli {
    #[command(subcommand)]
    pub subcommand: WmCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::Subcommand)]
pub enum WmCommand {
    /// Toggles the Seelen Window Manager.
    Toggle,
    /// Reserve space for a incoming window.
    Reserve {
        /// The position of the new window.
        side: AllowedReservations,
    },
    /// Cancels the current reservation
    CancelReservation,
    /// Increases or decreases the size of the window
    Width {
        /// What to do with the width.
        action: Sizing,
    },
    /// Increases or decreases the size of the window
    Height {
        /// What to do with the height.
        action: Sizing,
    },
    /// Resets the size of the containers in current workspace to the default size.
    ResetWorkspaceSize,
    /// Toggles the floating state of the window
    ToggleFloat,
    /// Toggles workspace layout mode to monocle (single stack)
    ToggleMonocle,
    /// Cycles the foregrounf node if it is a stack
    CycleStack { way: StepWay },
    /// Focuses the window in the specified position.
    Focus {
        /// The position of the window to focus.
        side: Direction,
    },
    /// Moves the window to the specified position
    Move {
        /// Direction to move
        side: Direction,
    },
    /// Moves the window to another monitor in the specified side
    MoveToMonitor {
        /// Direction to move
        side: Direction,
    },
}

// ===== TaskSwitcher =====

#[derive(Debug, Serialize, Deserialize, clap::Args)]
pub struct TaskSwitcherClient {
    #[command(subcommand)]
    pub command: TaskSwitcherCommand,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum TaskSwitcherCommand {
    SelectNextTask {
        #[clap(long)]
        auto_confirm: bool,
    },
    SelectPreviousTask {
        #[clap(long)]
        auto_confirm: bool,
    },
}

// ===== Wallpaper =====

/// Wallpaper manager commands
#[derive(Debug, Serialize, Deserialize, clap::Args)]
pub struct WallpaperCli {
    #[command(subcommand)]
    pub command: WallpaperCommand,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum WallpaperCommand {
    /// Cycle to the next wallpaper
    Next,
    /// Cycle to the previous wallpaper
    Prev,
}

// ===== System Tray =====

#[derive(Debug, Serialize, Deserialize, clap::Args)]
#[command(alias = "t")]
pub struct TrayCli {
    #[command(subcommand)]
    pub command: TrayCommand,
}

#[derive(Debug, Serialize, Deserialize, clap::Subcommand)]
pub enum TrayCommand {
    /// List all known tray icons (with pinned/order info).
    ListIcons,
    /// List only currently-pinned tray icons in pinned order.
    ListPinned,
    /// Print the persisted pin state (ordered logical identities).
    GetPinState,
    /// Pin a tray icon by logical identity key.
    Pin {
        /// Logical identity key (guid:.../exe:...|uid:...)
        logical_id: String,
    },
    /// Unpin a tray icon by logical identity key.
    Unpin { logical_id: String },
    /// Replace the whole pinned order with the provided keys.
    SetOrder {
        /// Ordered logical identity keys (space separated).
        order: Vec<String>,
    },
    /// Send a native action to a tray icon.
    Send {
        logical_id: String,
        /// One of: LeftClick, RightClick, MiddleClick, LeftDoubleClick, HoverEnter, HoverLeave, HoverMove
        action: String,
    },
}
