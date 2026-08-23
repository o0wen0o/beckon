//! Self-update: ask the manifest, verify its signature, replace this binary
//! (ADR-0022).
//!
//! Beckon is never launched deliberately — it starts with the machine and lives
//! in the tray — so it is never *replaced* deliberately either. Nothing in the
//! product would ever surface a new version, which is why the check is here at
//! all rather than left to whoever downloaded the installer.
//!
//! The signature checked here is the minisign one over the release artifact,
//! against the public key compiled into this binary
//! (`plugins.updater.pubkey`). It is not a platform code signature and does not
//! become one: the first install still meets Gatekeeper and SmartScreen
//! unsigned (README), and every update after it is authenticated against a key
//! the already-trusted binary carries.
//!
//! Nothing here touches config or the Registry, so this module is outside the
//! reload path entirely: an available update is neither config nor an Action,
//! and the filesystem has no opinion about it (ADR-0003). The tray is the only
//! surface that renders it.

use std::sync::atomic::Ordering;

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::config::Language;
use crate::state::AppState;
use crate::{i18n, tray, trigger};

/// How long after startup the one automatic check waits.
///
/// Beckon starts at login, so an immediate check would race the network coming
/// up and fail about a machine that is merely still booting. A quiet check has
/// nothing to say about that either way — this only keeps it from being wasted.
const STARTUP_DELAY: std::time::Duration = std::time::Duration::from_secs(30);

/// Whether a check with nothing to report is allowed to say so.
#[derive(Debug, Clone, Copy)]
pub enum Voice {
    /// The automatic check. Speaks only when there is something to install: a
    /// resident app that announces "still up to date" at every login has taught
    /// its user to dismiss the one notification that mattered.
    Quiet,
    /// The tray item. A click that produces no notification reads as a broken
    /// menu, so "up to date" and every failure are said out loud.
    Aloud,
}

/// The one automatic check, fired from `setup`.
pub fn check_on_startup(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(STARTUP_DELAY).await;
        // Read after the wait rather than before it: Settings is open on first
        // run, so the switch can be turned off inside these thirty seconds, and
        // the check that has not happened yet is exactly the one being
        // declined. The tray item is unaffected — a click is not this switch.
        if !app.state::<AppState>().config_snapshot().update_check {
            return;
        }
        run_check(&app, Voice::Quiet).await;
    });
}

/// Ask the endpoint what the latest version is, and let the tray say so.
pub fn check(app: &AppHandle, voice: Voice) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { run_check(&app, voice).await });
}

/// The check itself, as a future rather than a spawn, so the one caller that is
/// already off the event loop — the startup task, which has nothing left to do
/// after its sleep — awaits it instead of spawning a second task to hold it.
async fn run_check(app: &AppHandle, voice: Voice) {
    let language = app.state::<AppState>().config_snapshot().language;
    let aloud = matches!(voice, Voice::Aloud);
    match fetch_update(app).await {
        Ok(Some(update)) => {
            let version = update.version;
            // The menu item is the only thing that can act on this, so it is
            // relabelled before the notification that points at it.
            tray::set_pending_update(app, Some(version.clone()));
            notify(
                app,
                &i18n::update_available_title(language, &version),
                &i18n::update_available_body(language, &version),
            );
        }
        Ok(None) => say_up_to_date(app, language, aloud),
        Err(detail) => {
            if aloud {
                say_failed(app, language, &detail);
            }
        }
    }
}

/// Download the release, verify it, and hand over to the installer.
pub fn install(app: &AppHandle) {
    // Installing ends this process — on Windows the NSIS installer needs the
    // files it replaces closed, and on macOS the swapped bundle has to be
    // relaunched — and an Exchange is never on disk to come back to (ADR-0004).
    // So a Popover on screen is a refusal, not a race to win.
    if popover_is_visible(app) {
        let language = app.state::<AppState>().config_snapshot().language;
        notify(
            app,
            i18n::update_busy_title(language),
            i18n::update_busy_body(language),
        );
        return;
    }

    // The menu stays clickable while the download runs, and two installers
    // writing the same files is not a state worth defining.
    if app
        .state::<AppState>()
        .updating
        .swap(true, Ordering::SeqCst)
    {
        return;
    }

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let outcome = download(&app).await;
        app.state::<AppState>()
            .updating
            .store(false, Ordering::SeqCst);
        // Read after the download rather than before it: it can run for minutes,
        // and the language the result is announced in is the one in force now.
        let language = app.state::<AppState>().config_snapshot().language;
        match outcome {
            Ok(Installed::Yes) => {
                // On Windows this is unreachable: the installer has already
                // ended the process. macOS arrives here with the new bundle in
                // place and nothing running out of it yet.
                //
                // Back to the main thread first — the teardown `restart` does
                // walks the windows and the tray, which is AppKit's thread and
                // nobody else's (ADR-0013).
                let restarting = app.clone();
                let _ = app.run_on_main_thread(move || restarting.restart());
            }
            // The version went away between the notification and the click.
            // Said out loud either way: the user asked for something and is
            // getting nothing.
            Ok(Installed::NothingToDo) => say_up_to_date(&app, language, true),
            Err(detail) => say_failed(&app, language, &detail),
        }
    });
}

/// Whether the download found anything to install.
enum Installed {
    Yes,
    NothingToDo,
}

/// What the manifest offers, if it offers anything.
///
/// The one place the channel's errors are flattened, so both paths phrase them
/// alike: an unreachable endpoint, a manifest that does not parse and a
/// signature that does not verify are each quoted verbatim to the reader, the
/// way `describeFailure` quotes a cause it did not write (ADR-0015).
async fn fetch_update(app: &AppHandle) -> Result<Option<Update>, String> {
    let updater = app.updater().map_err(|err| err.to_string())?;
    updater.check().await.map_err(|err| err.to_string())
}

/// Re-checks rather than carrying an `Update` over from the click that asked
/// for it: one more request is cheaper than keeping a handle alive across a
/// menu interaction, and it cannot be the stale one.
async fn download(app: &AppHandle) -> Result<Installed, String> {
    let Some(update) = fetch_update(app).await? else {
        return Ok(Installed::NothingToDo);
    };
    // No progress callbacks: the tray has a menu and a balloon and no third
    // place to put a percentage, and a notification per chunk is not it.
    update
        .download_and_install(|_chunk, _total| {}, || {})
        .await
        .map_err(|err| err.to_string())?;
    Ok(Installed::Yes)
}

/// Nothing to install. The clear happens whatever the voice — a version pending
/// since a check can be installed by hand or withdrawn from the release, and a
/// menu still offering it is wrong even when nobody is being told.
fn say_up_to_date(app: &AppHandle, language: Language, aloud: bool) {
    tray::set_pending_update(app, None);
    if aloud {
        notify(
            app,
            i18n::update_current_title(language),
            &i18n::update_current_body(language, &current_version(app)),
        );
    }
}

fn say_failed(app: &AppHandle, language: Language, detail: &str) {
    notify(
        app,
        i18n::update_failed_title(language),
        &i18n::update_failed_body(language, detail),
    );
}

fn current_version(app: &AppHandle) -> String {
    app.package_info().version.to_string()
}

fn popover_is_visible(app: &AppHandle) -> bool {
    app.get_webview_window(trigger::WINDOW_POPOVER)
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false)
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}
