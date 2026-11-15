//! Everything the three surfaces ask of the trigger layer: showing and hiding
//! themselves, and driving the Exchange the Popover renders.

use tauri::{AppHandle, State};

use crate::state::{AppState, PopoverView};
use crate::{hotkey, platform, trigger};

/// Register `accelerator` immediately to prove it is free, then release it.
#[tauri::command]
pub fn probe_hotkey(app: AppHandle, accelerator: String) -> Result<(), String> {
    hotkey::probe(&app, &accelerator)
}

#[tauri::command]
pub fn get_popover_view(state: State<AppState>) -> Option<PopoverView> {
    state
        .popover_view
        .lock()
        .expect("popover view lock")
        .clone()
}

#[tauri::command]
pub fn pick_action(app: AppHandle, action_id: String) {
    trigger::pick_from_launcher(&app, &action_id);
}

#[tauri::command]
pub fn submit_input(app: AppHandle, text: String) -> Result<String, String> {
    trigger::submit_input(&app, &text)
}

#[tauri::command]
pub fn follow_up(app: AppHandle, exchange_id: String, text: String) -> Result<(), String> {
    trigger::follow_up(&app, &exchange_id, &text)
}

/// Esc during a request (README): cancel, keep the window open.
#[tauri::command]
pub fn cancel_exchange(state: State<AppState>, exchange_id: String) {
    state.exchanges.cancel(&exchange_id);
}

/// Retry after an error: run the last user turn again, as it stands.
#[tauri::command]
pub fn retry_exchange(app: AppHandle, exchange_id: String) -> Result<(), String> {
    trigger::retry(&app, &exchange_id)
}

/// The Popover screenshot button (ADR-0016). Returns immediately: the snip runs
/// on its own thread and the window re-appears with the result attached.
#[tauri::command]
pub fn start_capture(app: AppHandle) {
    trigger::start_capture(&app);
}

/// One tile's remove button, by position in the attached list (ADR-0017).
#[tauri::command]
pub fn discard_capture(app: AppHandle, index: usize) {
    trigger::discard_capture(&app, index);
}

/// The Popover reporting its own size after a drag on one of its grips
/// (ADR-0018). Logical pixels, debounced by the window: a drag reports every
/// pixel and each report that is not ours is a write to `config.toml`.
#[tauri::command]
pub fn set_popover_size(app: AppHandle, width: f64, height: f64) -> Result<(), String> {
    trigger::window::remember_popover_size(&app, width, height)
}

#[tauri::command]
pub fn hide_popover(app: AppHandle) {
    trigger::hide_popover(&app);
}

#[tauri::command]
pub fn hide_launcher(app: AppHandle) {
    trigger::hide_launcher(&app);
}

#[tauri::command]
pub fn show_settings(app: AppHandle) {
    trigger::show_settings(&app);
}

/// The Popover's Copy button. A user-requested write, so it is *not* restored
/// (ADR-0002) — this is the only way a result leaves Beckon.
#[tauri::command]
pub fn copy_to_clipboard(text: String) -> Result<(), String> {
    platform::selection::write_clipboard_text(&text)
}

/// A link in an answer, clicked (ADR-0028). Opens in the system browser: the
/// Popover's webview never navigates, because it is the app.
///
/// Unlike `open_key_page`, this URL *does* come from the webview — it is text a
/// model wrote, and the model may have been steered by the Selection it was
/// handed. So the command is as narrow as a link can be: a web scheme and
/// nothing else, checked here rather than in the window, because the window is
/// the side that rendered the untrusted text. `file:`, `ms-settings:` and every
/// other handler the shell would happily launch stay unreachable.
#[tauri::command]
pub fn open_link(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;

    if !is_web_url(&url) {
        return Err("only http and https links can be opened".to_string());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// Whether `url` is something a browser, and only a browser, will take.
///
/// A prefix test rather than a parse: the question is which *handler* the shell
/// picks, and that is decided by the scheme alone. Whitespace and control
/// characters are refused outright — a real link has neither, and they are what
/// an argument smuggled past a shell looks like.
fn is_web_url(url: &str) -> bool {
    let scheme = url
        .get(..8)
        .is_some_and(|head| head.eq_ignore_ascii_case("https://"))
        || url
            .get(..7)
            .is_some_and(|head| head.eq_ignore_ascii_case("http://"));
    scheme && !url.chars().any(|c| c.is_whitespace() || c.is_control())
}

#[cfg(test)]
mod tests {
    use super::is_web_url;

    #[test]
    fn web_links_open() {
        assert!(is_web_url("https://example.com/a?b=c#d"));
        assert!(is_web_url("http://localhost:11434/"));
        assert!(is_web_url("HTTPS://EXAMPLE.COM"));
    }

    #[test]
    fn nothing_but_a_web_scheme_reaches_the_shell() {
        for url in [
            "",
            "file:///C:/Windows/System32/calc.exe",
            "ms-settings:privacy",
            "javascript:alert(1)",
            "mailto:someone@example.com",
            r"C:\Windows\System32\calc.exe",
            "//example.com",
            "https:/example.com",
            " https://example.com",
        ] {
            assert!(!is_web_url(url), "{url:?} must be refused");
        }
    }

    #[test]
    fn a_link_carries_no_whitespace_or_control_characters() {
        assert!(!is_web_url("https://example.com/ --flag"));
        assert!(!is_web_url("https://example.com/\n"));
        assert!(!is_web_url("https://example.com/\u{0}"));
    }
}
