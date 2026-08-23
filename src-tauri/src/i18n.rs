//! Every sentence Rust writes for a person to read, in both languages.
//!
//! The frontend has its own catalog (`src/lib/i18n/`) and it is the larger one
//! by far: almost all of Beckon's words are rendered in a webview. What is left
//! here is what no window can phrase — the tray menu and its balloon, and the
//! diagnostics that are *derived state* (`hotkey::apply`, `Registry::load`),
//! built where the failure is detected and only then handed to a window.
//!
//! Free functions rather than a table of `&'static str`, because most of these
//! interpolate: a struct of format strings would move the argument order into a
//! literal nothing type-checks. `match language` in one place per sentence is
//! the whole mechanism.
//!
//! What is deliberately *not* here: [`LlmError`](crate::llm::LlmError)'s
//! `Display`, `toml`'s parse errors and the OS's own messages. Each of those is
//! a cause quoted verbatim from something that does not speak Chinese; the
//! frontend names the *kind* in the reader's language (`FAILURE_PREFIX`) and
//! the quoted detail follows it as evidence.

use crate::config::Language;

/// Which modifiers `hotkey::parse` accepts, in the platform's own names — the
/// Rust half of `MODIFIER_ADVICE` in `src/lib/i18n/`. Only the wording differs
/// per platform: the parser takes `Ctrl`, `Alt`, `Shift`, `Cmd`/`Super` on both,
/// so a config written on one machine still registers on the other.
#[cfg(not(target_os = "macos"))]
const MODIFIERS: [&str; 2] = ["Ctrl, Alt or Shift", "Ctrl、Alt 或 Shift"];
#[cfg(target_os = "macos")]
const MODIFIERS: [&str; 2] = [
    "Cmd, Control, Option or Shift",
    "Cmd、Control、Option 或 Shift",
];

/// What the platform calls the thing the tray icon sits in — the sentences that
/// send a reader to the update item have to name a place they can find. Same
/// shape as [`MODIFIERS`]: one wording per platform, both languages in a row.
#[cfg(not(target_os = "macos"))]
const TRAY_SURFACE: [&str; 2] = ["tray menu", "通知区域菜单"];
#[cfg(target_os = "macos")]
const TRAY_SURFACE: [&str; 2] = ["menu bar", "菜单栏"];

// --- tray -----------------------------------------------------------------

pub fn tray_settings(language: Language) -> &'static str {
    match language {
        Language::En => "Settings…",
        Language::Zh => "设置…",
    }
}

pub fn tray_quit(language: Language) -> &'static str {
    match language {
        Language::En => "Quit Beckon",
        Language::Zh => "退出 Beckon",
    }
}

pub fn tray_error_title(language: Language) -> &'static str {
    match language {
        Language::En => "Beckon: a hotkey is not active",
        Language::Zh => "Beckon：有热键未生效",
    }
}

pub fn tray_error_body(language: Language, summary: &str) -> String {
    match language {
        Language::En => {
            format!("{summary}\n\nClick the Beckon icon to open Settings and fix it.")
        }
        Language::Zh => format!("{summary}\n\n点按 Beckon 图标打开设置即可修复。"),
    }
}

/// The Settings window's own title bar — the one window with decorations, so
/// the one title a person reads.
pub fn settings_window_title(language: Language) -> &'static str {
    match language {
        Language::En => "Beckon Settings",
        Language::Zh => "Beckon 设置",
    }
}

// --- updates (ADR-0022) -------------------------------------------------------

/// The update item with nothing pending. It asks; it does not promise.
pub fn tray_check_updates(language: Language) -> &'static str {
    match language {
        Language::En => "Check for Updates…",
        Language::Zh => "检查更新…",
    }
}

/// The same item once a check has found something. Named by version, because
/// that is the one fact that makes clicking it a decision rather than a leap.
pub fn tray_update_to(language: Language, version: &str) -> String {
    match language {
        Language::En => format!("Update to {version}…"),
        Language::Zh => format!("更新到 {version}…"),
    }
}

pub fn update_available_title(language: Language, version: &str) -> String {
    match language {
        Language::En => format!("Beckon {version} is available"),
        Language::Zh => format!("Beckon {version} 可更新"),
    }
}

/// Where to go, not what happened: neither platform routes a notification's
/// click back to us reliably (see `tray::build`), so the sentence has to name
/// the surface the reader must open themselves.
pub fn update_available_body(language: Language, version: &str) -> String {
    match language {
        Language::En => format!(
            "Open Beckon's {} and choose \"Update to {version}…\".",
            TRAY_SURFACE[0]
        ),
        Language::Zh => format!(
            "打开 Beckon 的{}，选择“更新到 {version}…”。",
            TRAY_SURFACE[1]
        ),
    }
}

pub fn update_current_title(language: Language) -> &'static str {
    match language {
        Language::En => "Beckon is up to date",
        Language::Zh => "Beckon 已是最新版本",
    }
}

pub fn update_current_body(language: Language, version: &str) -> String {
    match language {
        Language::En => format!("Version {version} is the latest release."),
        Language::Zh => format!("当前版本 {version} 已是最新发布。"),
    }
}

/// One title for both halves of the channel: an endpoint that cannot be
/// reached, a manifest that does not parse, a signature that does not verify and
/// a download that died are the same event to the reader — Beckon tried to get a
/// newer version and did not. Which one it was is the quoted cause below.
pub fn update_failed_title(language: Language) -> &'static str {
    match language {
        Language::En => "Beckon: the update failed",
        Language::Zh => "Beckon：更新失败",
    }
}

pub fn update_failed_body(language: Language, detail: &str) -> String {
    match language {
        Language::En => format!(
            "{detail}

Try again from the {} later.",
            TRAY_SURFACE[0]
        ),
        Language::Zh => format!(
            "{detail}

稍后可从{}再试。",
            TRAY_SURFACE[1]
        ),
    }
}

/// A refusal, not a delay: installing ends this process, and the Exchange in an
/// open Popover is not on disk anywhere to come back to (ADR-0004).
pub fn update_busy_title(language: Language) -> &'static str {
    match language {
        Language::En => "Beckon cannot update right now",
        Language::Zh => "Beckon 暂时无法更新",
    }
}

pub fn update_busy_body(language: Language) -> &'static str {
    match language {
        Language::En => {
            "Close the Popover first: Beckon restarts to finish updating, and an Exchange is never saved."
        }
        Language::Zh => "请先关闭浮窗：Beckon 需要重启以完成更新，而对话不会被保存。",
    }
}

// --- hotkeys --------------------------------------------------------------

/// How the Launcher hotkey names itself in [`ApplyReport::summary`](crate::hotkey::ApplyReport::summary).
pub fn hotkey_launcher(language: Language) -> &'static str {
    match language {
        Language::En => "Launcher hotkey",
        Language::Zh => "启动器热键",
    }
}

pub fn hotkey_missing(language: Language) -> &'static str {
    match language {
        Language::En => "no hotkey given",
        Language::Zh => "未设置热键",
    }
}

pub fn hotkey_invalid(language: Language, accelerator: &str, detail: &str) -> String {
    match language {
        Language::En => format!("\"{accelerator}\" is not a valid hotkey: {detail}"),
        Language::Zh => format!("“{accelerator}”不是有效的热键：{detail}"),
    }
}

/// The whole sentence, not just the advice: the accelerator is what the reader
/// has to change, and Chinese does not put it where English does.
pub fn hotkey_needs_modifier(language: Language, accelerator: &str) -> String {
    match language {
        Language::En => format!("\"{accelerator}\" has no modifier; add {}", MODIFIERS[0]),
        Language::Zh => format!("“{accelerator}”没有修饰键；请加上 {}", MODIFIERS[1]),
    }
}

pub fn hotkey_not_registered(language: Language, accelerator: &str, detail: &str) -> String {
    match language {
        Language::En => format!("\"{accelerator}\" could not be registered: {detail}"),
        Language::Zh => format!("“{accelerator}”无法注册：{detail}"),
    }
}

/// The owner named in a conflict message when the winner is the Launcher.
pub fn hotkey_owner_launcher(language: Language) -> &'static str {
    match language {
        Language::En => "the Launcher hotkey",
        Language::Zh => "启动器热键",
    }
}

pub fn hotkey_taken(language: Language, accelerator: &str, owner: &str) -> String {
    match language {
        Language::En => format!("{accelerator} is already used by {owner}"),
        Language::Zh => format!("{accelerator} 已被{owner}占用"),
    }
}

/// The same collision as [`hotkey_taken`], decided from the files before the OS
/// is asked at all (`Registry::hotkey_plan`), where the winner is always an
/// Action and is named by its display name.
pub fn hotkey_claimed(language: Language, accelerator: &str, winner: &str) -> String {
    match language {
        Language::En => format!("{accelerator} is already claimed by \"{winner}\""),
        Language::Zh => format!("{accelerator} 已被“{winner}”占用"),
    }
}

// --- the Actions directory ------------------------------------------------

pub fn actions_dir_unreadable(language: Language, detail: &str) -> String {
    match language {
        Language::En => format!("actions directory could not be read: {detail}"),
        Language::Zh => format!("无法读取 actions 目录：{detail}"),
    }
}

pub fn action_file_unreadable(language: Language, detail: &str) -> String {
    match language {
        Language::En => format!("could not be read: {detail}"),
        Language::Zh => format!("无法读取：{detail}"),
    }
}

// --- the credential and the model list ------------------------------------

pub fn credential_unreadable(language: Language, detail: &str) -> String {
    match language {
        Language::En => format!("The Credential Manager could not be read: {detail}"),
        Language::Zh => format!("无法读取凭据存储：{detail}"),
    }
}

/// Why the documented catalog is being shown: no key to ask the endpoint with.
pub fn models_need_key(language: Language) -> &'static str {
    match language {
        Language::En => "Store one to list the models your endpoint actually serves.",
        Language::Zh => "保存密钥后，才能列出此端点实际提供的模型。",
    }
}

pub fn models_empty(language: Language) -> &'static str {
    match language {
        Language::En => "Its list came back empty.",
        Language::Zh => "端点返回的模型列表为空。",
    }
}

pub fn test_needs_key(language: Language) -> &'static str {
    match language {
        Language::En => "No API key is stored yet. Enter one above, then test again.",
        Language::Zh => "尚未保存 API 密钥。请在上方输入后再测试。",
    }
}

// --- the provider row (ADR-0021) ------------------------------------------

/// An Action, or `[defaults]`, naming a row that is not in the table. A
/// hand-edit, or a row removed while a Popover was open. Named rather than
/// redirected: sending to a different endpoint than the file says would be worse
/// than refusing.
pub fn provider_missing(language: Language, id: &str) -> String {
    match language {
        Language::En => format!(
            "No endpoint named \"{id}\" is configured. Open Settings → Connection and add \
             it, or point this Action at one that is there."
        ),
        Language::Zh => format!(
            "配置中没有名为“{id}”的端点。请在“设置 → 连接”中添加，或让此 Action 指向已有的端点。"
        ),
    }
}

/// The turn's own version of "no credential", naming which endpoint wants one.
/// With a provider per Action, one Action can be broken while every other one
/// works, so the sentence has to say *whose* key is missing (ADR-0021).
pub fn turn_needs_key(language: Language, label: &str) -> String {
    match language {
        Language::En => {
            format!("No API key is stored for {label}. Open Settings to add one.")
        }
        Language::Zh => format!("尚未为 {label} 保存 API 密钥。请打开设置添加。"),
    }
}

// --- the Capture ----------------------------------------------------------

/// A screenshot over Beckon's ceiling (ADR-0016). Beckon's own prose with
/// Beckon's own advice, so it belongs here rather than in `platform/`, which
/// hands up the two numbers and has no `Language` in reach.
///
/// Said in whole mebibytes: the numbers are megabyte-scale, and a byte count is
/// not something a reader can act on.
pub fn capture_too_large(language: Language, bytes: usize, max: usize) -> String {
    let mib = |value: usize| (value as f64) / (1024.0 * 1024.0);
    let (bytes, max) = (mib(bytes), mib(max));
    match language {
        Language::En => format!(
            "the screenshot is {bytes:.1} MB, over Beckon's {max:.0} MB limit; \
             capture a smaller region"
        ),
        Language::Zh => {
            format!("这张截图有 {bytes:.1} MB，超过 Beckon 的 {max:.0} MB 上限；请截取更小的范围")
        }
    }
}

/// A screenshot taken with the tray already full (ADR-0017). Says the ceiling
/// and what to do about it, because the bytes are being dropped either way.
pub fn capture_too_many(language: Language, max: usize) -> String {
    match language {
        Language::En => {
            format!("a turn carries at most {max} screenshots; remove one before taking another")
        }
        Language::Zh => format!("一次对话最多附带 {max} 张截图；请先移除一张再截图"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two languages have to *differ*, or a missing arm has been filled in
    /// by copying the English one — which reads as translated and is not.
    #[test]
    fn every_sentence_is_translated() {
        let en = [
            tray_settings(Language::En).to_string(),
            tray_quit(Language::En).to_string(),
            tray_error_title(Language::En).to_string(),
            tray_error_body(Language::En, "x"),
            settings_window_title(Language::En).to_string(),
            tray_check_updates(Language::En).to_string(),
            tray_update_to(Language::En, "1.2.3"),
            update_available_title(Language::En, "1.2.3"),
            update_available_body(Language::En, "1.2.3"),
            update_current_title(Language::En).to_string(),
            update_current_body(Language::En, "1.2.3"),
            update_failed_title(Language::En).to_string(),
            update_failed_body(Language::En, "d"),
            update_busy_title(Language::En).to_string(),
            update_busy_body(Language::En).to_string(),
            hotkey_launcher(Language::En).to_string(),
            hotkey_missing(Language::En).to_string(),
            hotkey_invalid(Language::En, "a", "d"),
            hotkey_needs_modifier(Language::En, "a"),
            hotkey_not_registered(Language::En, "a", "d"),
            hotkey_owner_launcher(Language::En).to_string(),
            hotkey_taken(Language::En, "a", "o"),
            hotkey_claimed(Language::En, "a", "w"),
            actions_dir_unreadable(Language::En, "d"),
            action_file_unreadable(Language::En, "d"),
            credential_unreadable(Language::En, "d"),
            models_need_key(Language::En).to_string(),
            models_empty(Language::En).to_string(),
            test_needs_key(Language::En).to_string(),
            provider_missing(Language::En, "p"),
            turn_needs_key(Language::En, "p"),
            capture_too_large(Language::En, 1, 2),
            capture_too_many(Language::En, 4),
        ];
        let zh = [
            tray_settings(Language::Zh).to_string(),
            tray_quit(Language::Zh).to_string(),
            tray_error_title(Language::Zh).to_string(),
            tray_error_body(Language::Zh, "x"),
            settings_window_title(Language::Zh).to_string(),
            tray_check_updates(Language::Zh).to_string(),
            tray_update_to(Language::Zh, "1.2.3"),
            update_available_title(Language::Zh, "1.2.3"),
            update_available_body(Language::Zh, "1.2.3"),
            update_current_title(Language::Zh).to_string(),
            update_current_body(Language::Zh, "1.2.3"),
            update_failed_title(Language::Zh).to_string(),
            update_failed_body(Language::Zh, "d"),
            update_busy_title(Language::Zh).to_string(),
            update_busy_body(Language::Zh).to_string(),
            hotkey_launcher(Language::Zh).to_string(),
            hotkey_missing(Language::Zh).to_string(),
            hotkey_invalid(Language::Zh, "a", "d"),
            hotkey_needs_modifier(Language::Zh, "a"),
            hotkey_not_registered(Language::Zh, "a", "d"),
            hotkey_owner_launcher(Language::Zh).to_string(),
            hotkey_taken(Language::Zh, "a", "o"),
            hotkey_claimed(Language::Zh, "a", "w"),
            actions_dir_unreadable(Language::Zh, "d"),
            action_file_unreadable(Language::Zh, "d"),
            credential_unreadable(Language::Zh, "d"),
            models_need_key(Language::Zh).to_string(),
            models_empty(Language::Zh).to_string(),
            test_needs_key(Language::Zh).to_string(),
            provider_missing(Language::Zh, "p"),
            turn_needs_key(Language::Zh, "p"),
            capture_too_large(Language::Zh, 1, 2),
            capture_too_many(Language::Zh, 4),
        ];
        for (english, chinese) in en.iter().zip(zh.iter()) {
            assert_ne!(english, chinese, "untranslated: {english}");
        }
    }

    /// The interpolated values survive translation: a placeholder dropped from
    /// one arm loses the accelerator the whole sentence is about.
    #[test]
    fn arguments_reach_both_arms() {
        for language in [Language::En, Language::Zh] {
            assert!(hotkey_taken(language, "Ctrl+Alt+T", "owner").contains("Ctrl+Alt+T"));
            assert!(hotkey_claimed(language, "Ctrl+Alt+T", "Winner").contains("Winner"));
            assert!(hotkey_invalid(language, "Banana+T", "why").contains("why"));
            assert!(hotkey_needs_modifier(language, "T").contains("T"));
            assert!(tray_error_body(language, "summary").contains("summary"));
            // The version is what makes the update item a decision rather than
            // a leap, so it has to survive into both arms of all three.
            assert!(tray_update_to(language, "0.2.0").contains("0.2.0"));
            assert!(update_available_title(language, "0.2.0").contains("0.2.0"));
            assert!(update_available_body(language, "0.2.0").contains("0.2.0"));
            assert!(update_current_body(language, "0.1.0").contains("0.1.0"));
            assert!(update_failed_body(language, "no route to host").contains("no route to host"));
            // Both numbers, or a reader who hits the ceiling cannot tell what
            // to aim under.
            let too_large = capture_too_large(language, 9 * 1024 * 1024, 8 * 1024 * 1024);
            assert!(too_large.contains("9.0"), "{too_large}");
            assert!(too_large.contains("8 MB"), "{too_large}");
        }
    }
}
