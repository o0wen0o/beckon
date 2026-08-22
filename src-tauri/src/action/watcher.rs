//! Filesystem watcher over the config directory (ADR-0003).
//!
//! Two things make a naive watcher misbehave:
//!
//! * Editors save atomically — a temp file plus a rename shows up as
//!   delete + create rather than modify. We sidestep event-kind analysis
//!   entirely by reloading the whole directory on any relevant event.
//! * Our own writes would echo back and could loop. Every write registers the
//!   path in [`SelfWrites`] and the first event for that path is swallowed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use notify_debouncer_full::notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};

/// Debounce window. Long enough to collapse an editor's temp-file dance, short
/// enough that a hand edit feels immediate.
pub const DEBOUNCE: Duration = Duration::from_millis(300);
/// How long a self-written path stays suppressed. Must exceed [`DEBOUNCE`], or
/// our own debounced event arrives after the mark has expired.
const SUPPRESSION: Duration = Duration::from_millis(1_500);

/// Paths Beckon itself just wrote, so the watcher can ignore the echo.
#[derive(Debug, Default)]
pub struct SelfWrites {
    inner: Mutex<HashMap<PathBuf, Instant>>,
}

impl SelfWrites {
    pub fn mark(&self, path: &Path) {
        let mut map = self.inner.lock().expect("self-write lock");
        // Keyed the way the watcher will report it, not the way we wrote it: on
        // macOS the two differ whenever the config directory is reached through a
        // symlink, and an unrecognised echo is a reload mid-edit.
        map.insert(crate::platform::watch_path(path), Instant::now());
        map.retain(|_, at| at.elapsed() < SUPPRESSION);
    }

    /// True if this path was written by us recently. Consumes the mark, so a
    /// genuine external edit right after our own write is not swallowed twice.
    pub fn take(&self, path: &Path) -> bool {
        let mut map = self.inner.lock().expect("self-write lock");
        match map.remove(&crate::platform::watch_path(path)) {
            Some(at) => at.elapsed() < SUPPRESSION,
            None => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Config,
    Actions,
}

/// Watched layout. `root` is the directory handed to the OS watcher;
/// classification happens on our side.
pub struct Watched {
    pub root: PathBuf,
    pub config_file: PathBuf,
    pub actions_dir: PathBuf,
}

/// Start watching. The returned guard owns the debouncer — dropping it stops
/// the watcher.
pub fn spawn<F>(
    watched: Watched,
    self_writes: std::sync::Arc<SelfWrites>,
    on_change: F,
) -> Result<WatcherGuard, String>
where
    F: Fn(Change) + Send + 'static,
{
    std::fs::create_dir_all(&watched.actions_dir).map_err(|e| e.to_string())?;

    // Classify against the reported form of each path, resolved after the
    // directories exist. Without this, macOS reports every event under the
    // symlink-free path and `classify` matches none of them.
    let watched = Watched {
        root: crate::platform::watch_path(&watched.root),
        config_file: crate::platform::watch_path(&watched.config_file),
        actions_dir: crate::platform::watch_path(&watched.actions_dir),
    };

    let (tx, rx) = std::sync::mpsc::channel::<DebounceEventResult>();
    let mut debouncer = new_debouncer(DEBOUNCE, None, tx).map_err(|e| e.to_string())?;
    debouncer
        .watch(&watched.root, RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;

    let handle = std::thread::Builder::new()
        .name("beckon-config-watcher".into())
        .spawn(move || {
            // Owning the debouncer here keeps the watch alive for the thread's life.
            let _debouncer = debouncer;
            for result in rx {
                let events = match result {
                    Ok(events) => events,
                    Err(errors) => {
                        for error in errors {
                            log::warn!("watcher error: {error}");
                        }
                        continue;
                    }
                };

                let paths = events.iter().flat_map(|event| event.paths.iter());
                for change in classify(&watched, paths, &self_writes) {
                    on_change(change);
                }
            }
        })
        .map_err(|e| e.to_string())?;

    Ok(WatcherGuard { _handle: handle })
}

pub struct WatcherGuard {
    _handle: std::thread::JoinHandle<()>,
}

/// Decide what a batch of paths means. Returns at most one [`Change`] of each
/// kind, so a save touching several Actions triggers a single reload.
fn classify<'a>(
    watched: &Watched,
    paths: impl Iterator<Item = &'a PathBuf>,
    self_writes: &SelfWrites,
) -> Vec<Change> {
    let mut config = false;
    let mut actions = false;

    for path in paths {
        if is_ignored(path) {
            continue;
        }
        // Only swallow the echo when the path is genuinely one of ours.
        if self_writes.take(path) {
            continue;
        }
        if *path == watched.config_file {
            config = true;
        } else if path.starts_with(&watched.actions_dir) {
            actions = true;
        }
    }

    let mut out = Vec::new();
    if config {
        out.push(Change::Config);
    }
    if actions {
        out.push(Change::Actions);
    }
    out
}

/// Temp files, backup files and anything that is not TOML. Shared with
/// [`Registry::load`](crate::action::registry::Registry::load) so the loader and
/// the watcher cannot disagree about what counts as an Action file.
pub fn is_ignored(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
        return true;
    };
    if name.starts_with('.') || name.ends_with('~') {
        return true;
    }
    // A rename's "from" path may no longer exist; judge by extension only.
    !name.ends_with(".toml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn watched(root: &Path) -> Watched {
        Watched {
            root: root.to_path_buf(),
            config_file: root.join("config.toml"),
            actions_dir: root.join("actions"),
        }
    }

    #[test]
    fn classifies_config_and_actions_once_each() {
        let root = Path::new("C:/beckon");
        let w = watched(root);
        let writes = SelfWrites::default();
        let paths = [
            root.join("config.toml"),
            root.join("actions").join("a.toml"),
            root.join("actions").join("b.toml"),
        ];
        let changes = classify(&w, paths.iter(), &writes);
        assert_eq!(changes, vec![Change::Config, Change::Actions]);
    }

    #[test]
    fn ignores_temp_and_non_toml_paths() {
        let root = Path::new("C:/beckon");
        let w = watched(root);
        let writes = SelfWrites::default();
        let paths = [
            root.join("actions").join(".a.toml.beckon-tmp"),
            root.join("actions").join("notes.txt"),
            root.join("actions").join("a.toml~"),
        ];
        assert!(classify(&w, paths.iter(), &writes).is_empty());
    }

    #[test]
    fn suppresses_our_own_write_once_only() {
        let root = Path::new("C:/beckon");
        let w = watched(root);
        let writes = Arc::new(SelfWrites::default());
        let path = root.join("actions").join("a.toml");

        writes.mark(&path);
        let paths = [path];
        assert!(classify(&w, paths.iter(), &writes).is_empty());
        // The next event for the same path is a real external edit.
        assert_eq!(classify(&w, paths.iter(), &writes), vec![Change::Actions]);
    }

    #[test]
    fn suppression_window_outlives_the_debounce_window() {
        assert!(SUPPRESSION > DEBOUNCE);
    }

    /// The live watcher against the two patterns that actually break: a plain
    /// external write, and an editor's atomic save (temp file + rename, which
    /// arrives as delete + create rather than modify).
    #[test]
    fn reports_external_writes_including_atomic_saves() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let actions_dir = root.join("actions");
        let (tx, rx) = std::sync::mpsc::channel();

        let guard = spawn(
            Watched {
                root: root.clone(),
                config_file: root.join("config.toml"),
                actions_dir: actions_dir.clone(),
            },
            Arc::new(SelfWrites::default()),
            move |change| {
                let _ = tx.send(change);
            },
        )
        .unwrap();

        std::fs::write(actions_dir.join("a.toml"), "name = \"A\"\n").unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(10)).unwrap(),
            Change::Actions
        );

        // The rename dance an editor performs when saving over the same file.
        crate::atomic::write_atomic(&actions_dir.join("a.toml"), "name = \"B\"\n").unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(10)).unwrap(),
            Change::Actions
        );

        // Editing config.toml is classified separately.
        std::fs::write(root.join("config.toml"), "autostart = false\n").unwrap();
        assert_eq!(
            rx.recv_timeout(Duration::from_secs(10)).unwrap(),
            Change::Config
        );

        drop(guard);
    }
}
