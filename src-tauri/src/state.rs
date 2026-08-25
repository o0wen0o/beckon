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
use crate::config::{Config, Language, PopoverSize};
use crate::exchange::ExchangeManager;
use crate::failure::Failure;
use crate::hotkey::HotkeyState;
use crate::models_cache::ModelsCache;
use crate::platform::capture::{self, Capture, Fault, Outcome};

/// The Beckon directory and the paths inside it (README): `%APPDATA%\Beckon\`
/// on Windows, `~/Library/Application Support/Beckon/` on macOS.
///
/// Two of the three are the user's, watched and broadcast whole (ADR-0003). The
/// third, `models_cache`, is Beckon's own and is neither — see
/// [`crate::models_cache`] and ADR-0024.
#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
    pub config_file: PathBuf,
    pub actions_dir: PathBuf,
    pub models_cache: PathBuf,
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
            models_cache: root.join("models.json"),
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
    /// The Captures the user has attached and not yet sent, oldest first
    /// (ADR-0016, ADR-0017).
    ///
    /// The view is where they live, not a second slot beside it: the request is
    /// built in Rust from these bytes and the window draws its thumbnails from
    /// the same ones, so one owner means the two can never disagree about what
    /// is attached (ADR-0003).
    ///
    /// Append-only until something removes one by index, and only the Popover
    /// removes: that is what makes an index a safe thing for the window to name
    /// (ADR-0017).
    pub captures: Vec<Capture>,
    /// What the last snip had to say, if it had anything.
    pub capture_notice: Option<CaptureNotice>,
}

/// What one run of the snip tool left to say, once it has been said in a
/// language (ADR-0016).
///
/// One field rather than a `bool` beside an `Option`: a run either attached a
/// Capture, produced nothing, or produced bytes that cannot be sent, so the two
/// notices can never both stand. Two fields would leave that invariant to be
/// re-established by hand at every layer it crosses — and the Popover renders
/// one line either way.
///
/// Adjacently tagged, so the window switches on `kind` and reads the quoted
/// cause out of `failure` — the same `describeFailure` a refused command goes
/// through.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "failure", rename_all = "kebab-case")]
pub enum CaptureNotice {
    /// Nothing was captured — Esc, or a tool that never answered. Not an error:
    /// nothing was captured, so nothing is being dropped.
    Cancelled,
    /// A screenshot *was* taken and cannot be used: over the size ceiling, over
    /// the count ceiling, or bytes no decoder recognised. Distinct from
    /// `Cancelled`, because the two have different advice attached.
    Failed(Failure),
}

/// How many Captures one turn may carry.
///
/// The per-image ceiling is 8 MiB ([`capture::MAX_BYTES`]) inside a 48 MiB body,
/// so the wire would take six; the binding constraint is ADR-0004 instead — the
/// history is resent untruncated on every follow-up, so each attachment is paid
/// for again by every later turn in the Exchange.
pub const MAX_CAPTURES: usize = 4;

/// The two capture fields are only ever moved through these, because the
/// difference between the three moves is the whole subtlety (ADR-0016): a snip
/// that produced nothing must leave the attached Captures alone, while sending
/// them must not leave them attached to be sent twice. Field-by-field at the
/// call sites is how one of the two drifts.
impl PopoverView {
    /// Take the attached Captures for the turn that is starting, and drop what
    /// the last snip had to say with them.
    pub fn take_captures(&mut self) -> Vec<Capture> {
        self.capture_notice = None;
        std::mem::take(&mut self.captures)
    }

    /// One tile's remove button.
    ///
    /// Out-of-range is a no-op rather than a panic: the index came from a window
    /// that rendered the list a moment ago, and a snip landing in between grows
    /// the list without moving what is already in it.
    pub fn remove_capture(&mut self, index: usize) {
        if index < self.captures.len() {
            self.captures.remove(index);
        }
        // Whatever the last snip said was about a different attempt.
        self.capture_notice = None;
    }

    /// Record what one run of the snip tool produced.
    ///
    /// `Nothing` and `Failed` deliberately leave `captures` alone: the user
    /// pressed Esc in the snip tool, not in the Popover. A snip taken with the
    /// tray already full is the same shape of answer — the bytes exist and
    /// cannot be sent — so it lands as an error beside them, not instead of them.
    pub fn apply_capture(&mut self, outcome: Outcome, language: Language) {
        self.capture_notice = match outcome {
            Outcome::Captured(_) if self.captures.len() >= MAX_CAPTURES => {
                Some(CaptureNotice::Failed(Failure::new(
                    "capture-too-many",
                    crate::i18n::capture_too_many(language, MAX_CAPTURES),
                )))
            }
            Outcome::Captured(capture) => {
                self.captures.push(capture);
                None
            }
            Outcome::Nothing => Some(CaptureNotice::Cancelled),
            Outcome::Failed(fault) => Some(CaptureNotice::Failed(describe_fault(language, fault))),
        };
    }
}

/// A [`Fault`] as the kind-plus-message pair the Popover reads.
///
/// The kind is a contract string the frontend catalogs key on; the message is
/// either Beckon's own sentence in the reader's language or a cause quoted
/// verbatim from a decoder that does not speak it (ADR-0015). `platform/` hands
/// up the fact precisely so this choice is made where the language is known.
fn describe_fault(language: Language, fault: Fault) -> Failure {
    match fault {
        Fault::TooLarge { bytes } => Failure::new(
            "capture-too-large",
            crate::i18n::capture_too_large(language, bytes, capture::MAX_BYTES),
        ),
        Fault::Unreadable(detail) => Failure::new("capture-unreadable", detail),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PopoverPhase {
    /// `prompt`, or `auto` with an empty grab: ask for typed input. An empty
    /// grab is a phase, never an error (ADR-0002) — and since ADR-0020 removed
    /// the `selection` Input Source, it is always *this* phase.
    NeedsInput,
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
    /// The last list each endpoint served (ADR-0024). Behind a `Mutex` because
    /// it is the one writer of its file: `write_atomic` uses a fixed temp path,
    /// and opening Settings primes every row at once.
    pub models_cache: Mutex<ModelsCache>,

    /// The Selection grabbed eagerly at hotkey press, waiting for the user to
    /// pick an Action in the Launcher.
    pub pending_selection: Mutex<Option<String>>,
    /// The window that had focus before we showed anything, to hand it back.
    pub previous_foreground: Mutex<Option<isize>>,
    pub popover_view: Mutex<Option<PopoverView>>,

    /// Kept alive for the process lifetime; dropping it stops the watcher.
    pub watcher: Mutex<Option<WatcherGuard>>,
    /// A snip is running (ADR-0016). One at a time: the button and its shortcut
    /// can both land before the window is even hidden, and two snip tools
    /// fighting over one clipboard is not a state worth defining.
    pub capturing: AtomicBool,
    /// The startup hotkey-failure balloon fires once only (README).
    pub balloon_shown: AtomicBool,
    /// The version the update endpoint is offering, once a check has found one
    /// (ADR-0022). Not config and not derived from disk: it is what the tray
    /// menu's one variable item is labelled from, and the only thing that can
    /// act on it is that item.
    pub pending_update: Mutex<Option<String>>,
    /// A download-and-install is running. One at a time, for the same reason
    /// `capturing` is: the menu stays clickable throughout, and two installers
    /// writing the same files is not a state worth defining.
    pub updating: AtomicBool,
    /// The size the Popover window was last *told* to be (ADR-0018).
    ///
    /// Every resize reports itself, ours included, and the two have to be told
    /// apart: the `set_size` at the start of each trigger would otherwise be
    /// written back as if the user had dragged there, and a clamp or a rounding
    /// difference would then walk the remembered size a pixel at a time. This is
    /// the one value that distinguishes them, and it is window state rather than
    /// config — nothing outside the trigger flow has an opinion about it.
    pub popover_asked_size: Mutex<PopoverSize>,
    /// Errors that put the tray icon into its error state.
    pub startup_errors: Mutex<Vec<String>>,
}

impl AppState {
    pub fn new(paths: Paths, config: Config, registry: Registry) -> Self {
        let models_cache = Mutex::new(ModelsCache::load(&paths.models_cache));
        Self {
            paths,
            config: RwLock::new(config),
            registry: RwLock::new(registry),
            exchanges: ExchangeManager::default(),
            hotkeys: Mutex::new(HotkeyState::default()),
            self_writes: Arc::new(SelfWrites::default()),
            http: crate::llm::client::build_http_client(),
            models_cache,
            pending_selection: Mutex::new(None),
            previous_foreground: Mutex::new(None),
            popover_view: Mutex::new(None),
            watcher: Mutex::new(None),
            capturing: AtomicBool::new(false),
            balloon_shown: AtomicBool::new(false),
            pending_update: Mutex::new(None),
            updating: AtomicBool::new(false),
            popover_asked_size: Mutex::new(PopoverSize::default()),
            startup_errors: Mutex::new(Vec::new()),
        }
    }

    pub fn config_snapshot(&self) -> Config {
        self.config.read().expect("config lock").clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::ModelParams;

    fn capture(tag: &str) -> Capture {
        Capture {
            data_url: format!("data:image/png;base64,{tag}"),
            width: 10,
            height: 10,
            bytes: 4,
        }
    }

    /// The `kind` a [`CaptureNotice::Failed`] carries, which is the contract
    /// string the frontend catalogs key on.
    fn notice_kind(view: &PopoverView) -> Option<&str> {
        match view.capture_notice.as_ref()? {
            CaptureNotice::Failed(failure) => Some(failure.kind.as_str()),
            CaptureNotice::Cancelled => None,
        }
    }

    fn view() -> PopoverView {
        PopoverView {
            action_id: "a".into(),
            action_name: "A".into(),
            model: ModelParams {
                provider: "p".into(),
                model: "m".into(),
                thinking: false,
                web_search: false,
            },
            phase: PopoverPhase::NeedsInput,
            input: None,
            exchange_id: None,
            captures: Vec::new(),
            capture_notice: None,
        }
    }

    /// Snips append, so the order on the wire is the order they were taken
    /// (ADR-0017) — a note saying "the second one" depends on it.
    #[test]
    fn captures_attach_in_the_order_they_were_taken() {
        let mut view = view();
        view.apply_capture(Outcome::Captured(capture("one")), Language::En);
        view.apply_capture(Outcome::Captured(capture("two")), Language::En);
        let urls: Vec<&str> = view
            .captures
            .iter()
            .map(|one| one.data_url.as_str())
            .collect();
        assert_eq!(
            urls,
            ["data:image/png;base64,one", "data:image/png;base64,two"]
        );
    }

    /// A cancelled snip must leave what is already attached alone: the user
    /// pressed Esc in the snip tool, not in the Popover (ADR-0016).
    #[test]
    fn a_cancelled_snip_keeps_what_was_attached() {
        let mut view = view();
        view.apply_capture(Outcome::Captured(capture("one")), Language::En);
        view.apply_capture(Outcome::Nothing, Language::En);
        assert_eq!(view.captures.len(), 1);
        assert!(matches!(
            view.capture_notice,
            Some(CaptureNotice::Cancelled)
        ));
    }

    /// Over the ceiling is an error *beside* the tray, not instead of it: the
    /// bytes are dropped, and what the user already attached is not.
    #[test]
    fn a_snip_past_the_ceiling_is_refused_and_keeps_the_rest() {
        let mut view = view();
        for index in 0..MAX_CAPTURES {
            view.apply_capture(Outcome::Captured(capture(&index.to_string())), Language::En);
        }
        view.apply_capture(Outcome::Captured(capture("extra")), Language::En);
        assert_eq!(view.captures.len(), MAX_CAPTURES);
        assert_eq!(notice_kind(&view), Some("capture-too-many"));
    }

    /// Remove takes the one named and nothing else; an index the window
    /// rendered before a snip landed is still that tile.
    #[test]
    fn remove_takes_only_the_tile_named() {
        let mut view = view();
        for tag in ["one", "two", "three"] {
            view.apply_capture(Outcome::Captured(capture(tag)), Language::En);
        }
        view.remove_capture(1);
        let urls: Vec<&str> = view
            .captures
            .iter()
            .map(|one| one.data_url.as_str())
            .collect();
        assert_eq!(
            urls,
            ["data:image/png;base64,one", "data:image/png;base64,three"]
        );
        // Out of range changes nothing rather than panicking.
        view.remove_capture(9);
        assert_eq!(view.captures.len(), 2);
    }

    /// Sending empties the tray: leaving them attached would send them again
    /// with the next follow-up (ADR-0016).
    #[test]
    fn sending_consumes_the_whole_tray() {
        let mut view = view();
        view.apply_capture(Outcome::Captured(capture("one")), Language::En);
        view.apply_capture(Outcome::Nothing, Language::En);
        let taken = view.take_captures();
        assert_eq!(taken.len(), 1);
        assert!(view.captures.is_empty());
        assert!(view.capture_notice.is_none());
    }
}
