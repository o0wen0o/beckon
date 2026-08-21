//! Shared application state.
//!
//! Rust owns every authoritative value here; the windows are views over it
//! (ADR-0003). Locks are plain `std` ones and are never held across an `await` —
//! read what you need, drop the guard, then do the slow thing.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

use serde::Serialize;

use crate::action::registry::Registry;
use crate::action::watcher::{SelfWrites, WatcherGuard};
use crate::action::ModelParams;
use crate::config::Config;
use crate::exchange::ExchangeManager;
use crate::hotkey::HotkeyState;

/// The Beckon directory and the two paths inside it (README): `%APPDATA%\Beckon\`
/// on Windows, `~/Library/Application Support/Beckon/` on macOS.
#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
    pub config_file: PathBuf,
    pub actions_dir: PathBuf,
}

impl Paths {
    /// Deliberately *not* Tauri's identifier-derived config dir: the README
    /// promises a directory named after the app, not after `com.beckon.app`.
    ///
    /// Read straight from the environment rather than through `AppHandle`,
    /// because the state has to exist *before* the first window is created — a
    /// webview starts loading the moment it exists and can invoke a command
    /// before `setup` would have run.
    pub fn resolve() -> Result<Self, String> {
        let root = Self::root()?.join("Beckon");
        Ok(Self {
            config_file: root.join("config.toml"),
            actions_dir: root.join("actions"),
            root,
        })
    }

    /// The per-user application-data directory Beckon's own folder sits in.
    ///
    /// Each platform names one place a resident tool's editable config belongs,
    /// and it is the place its users already know to look — so this is a lookup
    /// per platform, not one path with substitutions (ADR-0013).
    fn root() -> Result<PathBuf, String> {
        #[cfg(windows)]
        {
            std::env::var_os("APPDATA")
                .map(PathBuf::from)
                .ok_or_else(|| "APPDATA is not set; cannot locate the config directory".to_string())
        }
        #[cfg(target_os = "macos")]
        {
            std::env::var_os("HOME")
                .map(|home| {
                    PathBuf::from(home)
                        .join("Library")
                        .join("Application Support")
                })
                .ok_or_else(|| "HOME is not set; cannot locate the config directory".to_string())
        }
        #[cfg(not(any(windows, target_os = "macos")))]
        {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config"))
                })
                .ok_or_else(|| "HOME is not set; cannot locate the config directory".to_string())
        }
    }
}

/// What the Popover is currently showing. Rust decides this, the window renders
/// it — otherwise the `input_source` rules would live in two places.
#[derive(Debug, Clone, Serialize)]
pub struct PopoverView {
    pub action_id: String,
    pub action_name: String,
    pub model: ModelParams,
    pub phase: PopoverPhase,
    /// The input that was sent, for display above the answer.
    pub input: Option<String>,
    pub exchange_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PopoverPhase {
    /// `prompt`, or `auto` with an empty grab: ask for typed input.
    NeedsInput,
    /// `selection` with an empty grab: a hint, and no request (README).
    EmptySelection,
    /// A request is in flight; the streaming states come from events.
    Running,
}

pub struct AppState {
    pub paths: Paths,
    pub config: RwLock<Config>,
    pub registry: RwLock<Registry>,
    pub exchanges: ExchangeManager,
    pub hotkeys: Mutex<HotkeyState>,
    pub self_writes: Arc<SelfWrites>,
    pub http: reqwest::Client,

    /// The Selection grabbed eagerly at hotkey press, waiting for the user to
    /// pick an Action in the Launcher.
    pub pending_selection: Mutex<Option<String>>,
    /// The window that had focus before we showed anything, to hand it back.
    pub previous_foreground: Mutex<Option<isize>>,
    pub popover_view: Mutex<Option<PopoverView>>,

    /// Kept alive for the process lifetime; dropping it stops the watcher.
    pub watcher: Mutex<Option<WatcherGuard>>,
    /// The startup hotkey-failure balloon fires once only (README).
    pub balloon_shown: AtomicBool,
    /// Errors that put the tray icon into its error state.
    pub startup_errors: Mutex<Vec<String>>,
}

impl AppState {
    pub fn new(paths: Paths, config: Config, registry: Registry) -> Self {
        Self {
            paths,
            config: RwLock::new(config),
            registry: RwLock::new(registry),
            exchanges: ExchangeManager::default(),
            hotkeys: Mutex::new(HotkeyState::default()),
            self_writes: Arc::new(SelfWrites::default()),
            http: crate::llm::client::build_http_client(),
            pending_selection: Mutex::new(None),
            previous_foreground: Mutex::new(None),
            popover_view: Mutex::new(None),
            watcher: Mutex::new(None),
            balloon_shown: AtomicBool::new(false),
            startup_errors: Mutex::new(Vec::new()),
        }
    }

    pub fn config_snapshot(&self) -> Config {
        self.config.read().expect("config lock").clone()
    }
}
