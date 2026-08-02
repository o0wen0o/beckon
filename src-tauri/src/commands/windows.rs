//! Everything the three surfaces ask of the trigger layer: showing and hiding
//! themselves, and driving the Exchange the Popover renders.

use tauri::{AppHandle, Manager, State};

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

/// Retry after an error: resend the last user message as a new turn.
#[tauri::command]
pub fn retry_exchange(app: AppHandle, exchange_id: String) -> Result<(), String> {
    let last_user = app
        .state::<AppState>()
        .exchanges
        .last_user_message(&exchange_id)
        .ok_or_else(|| "this Exchange is gone; trigger the Action again".to_string())?;
    trigger::follow_up(&app, &exchange_id, &last_user)
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
