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
            capture_too_large(Language::En, 1, 2),
            capture_too_many(Language::En, 4),
        ];
        let zh = [
            tray_settings(Language::Zh).to_string(),
            tray_quit(Language::Zh).to_string(),
            tray_error_title(Language::Zh).to_string(),
            tray_error_body(Language::Zh, "x"),
            settings_window_title(Language::Zh).to_string(),
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
            // Both numbers, or a reader who hits the ceiling cannot tell what
            // to aim under.
            let too_large = capture_too_large(language, 9 * 1024 * 1024, 8 * 1024 * 1024);
            assert!(too_large.contains("9.0"), "{too_large}");
            assert!(too_large.contains("8 MB"), "{too_large}");
        }
    }
}
