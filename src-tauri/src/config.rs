//! `config.toml` load/save/merge.
//!
//! Every field has a default: a missing file means "write the defaults", a
//! missing field means "use the default", never an error.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::atomic::write_atomic;

/// The Launcher's out-of-the-box hotkey.
///
/// Space with two modifiers on both platforms; only the first modifier differs,
/// because the platform's own launcher does — Spotlight is Cmd+Space, so Cmd is
/// the key a Mac user reaches for and Ctrl is the Windows equivalent. macOS also
/// spends Ctrl+Space and Ctrl+Option+Space on input-source switching.
///
/// Neither default is a Ctrl+Alt chord: that is AltGr on every ISO keyboard, so
/// it doubles as a character-composing combination, and it is the modifier state
/// `selection::grab_selection` has to release before its copy can land at all.
#[cfg(not(target_os = "macos"))]
pub const DEFAULT_LAUNCHER_HOTKEY: &str = "Ctrl+Shift+Space";
#[cfg(target_os = "macos")]
pub const DEFAULT_LAUNCHER_HOTKEY: &str = "Cmd+Shift+Space";

pub const DEFAULT_BASE_URL: &str = "https://api.deepseek.com";
/// The authority `DEFAULT_BASE_URL` carries, on its own: the documented model
/// catalog stands in only for this host (ADR-0021).
const DEEPSEEK_HOST: &str = "api.deepseek.com";
pub const DEFAULT_MODEL: &str = "deepseek-v4-flash";
/// The id of the row a fresh install gets, and therefore of `[defaults]
/// provider` (ADR-0021). Also the account the pre-provider credential is
/// migrated onto — see [`crate::secrets::migrate_legacy`].
pub const DEFAULT_PROVIDER_ID: &str = "deepseek";
pub const DEFAULT_PROVIDER_LABEL: &str = "DeepSeek";
/// Where a DeepSeek key comes from. A `key_page` is data on the row, not a
/// constant in a command, because every provider has a different one.
pub const DEFAULT_KEY_PAGE: &str = "https://platform.deepseek.com/api_keys";
/// DeepSeek's own guidance for general conversation and translation. Was pinned
/// for every request (ADR-0019); it is now one row's value, because it was only
/// ever a fact about DeepSeek and a `temperature` DeepSeek likes is not one
/// Ollama does (ADR-0021).
pub const DEEPSEEK_TEMPERATURE: f64 = 1.3;

/// The Popover's size out of the box, in logical pixels (ADR-0018). Mirrored in
/// `tauri.conf.json` so the very first paint is not at the wrong size.
pub const DEFAULT_POPOVER_W: f64 = 620.0;
pub const DEFAULT_POPOVER_H: f64 = 500.0;
/// The floors, mirrored in `tauri.conf.json` as `minWidth`/`minHeight` so the
/// window manager refuses a smaller drag rather than us undoing one afterwards.
///
/// The width floor is the composer's row — camera, box, Send — which wraps
/// below it. The height floor is a composer plus one line of answer: with the
/// hint window gone (ADR-0020) the product no longer sizes the Popover itself,
/// so the floor only has to keep a hand-drag readable.
pub const MIN_POPOVER_W: f64 = 380.0;
pub const MIN_POPOVER_H: f64 = 200.0;
/// The ceilings exist only to keep a garbled value out of the file; a 4K panel
/// is the largest thing a window can usefully be dragged to.
pub const MAX_POPOVER_W: f64 = 3840.0;
pub const MAX_POPOVER_H: f64 = 2160.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub launcher_hotkey: String,
    pub autostart: bool,
    /// Declared before the tables: `toml` serializes in field order and a
    /// scalar written after a table would land inside it.
    pub theme: Theme,
    pub language: Language,
    pub defaults: ModelDefaults,
    pub popover: PopoverSize,
    /// Last, because it is the only field that serializes as an *array* of
    /// tables: `[[api.providers]]` swallows every table header written after it,
    /// so nothing may be.
    ///
    /// Its own `default`, not the container's: the container's would hand a
    /// missing `[api]` the row [`Config::default`] synthesises, and a file that
    /// names `[defaults] model` without naming `base_url` would then have that
    /// model silently ignored — `fold_legacy` only fires on an empty table.
    #[serde(default)]
    pub api: ApiConfig,
}

/// Which palette the three surfaces paint in.
///
/// `Light` is the default, so an absent `theme` resolves to light like every
/// other missing field. `System` reads the OS appearance — but only once it
/// has been *chosen*: the OS preference never applies on its own, which is why
/// this is a three-valued setting rather than a bool plus a media query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Light,
    Dark,
    System,
}

/// Which language the three surfaces are written in.
///
/// `En` is the default, so an absent `language` resolves to English like every
/// other missing field. There is no `system` arm the way [`Theme`] has one: the
/// OS locale is a guess about a *reader*, not a setting, and a wrong guess here
/// replaces every word in the product rather than its palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Zh,
}

/// How an endpoint is told **not** to think (ADR-0021).
///
/// A property of the *endpoint*, never of the model: `deepseek-ai/DeepSeek-V3`
/// served by SiliconFlow speaks the plain OpenAI dialect, so no rule over model
/// ids can produce this — the row states it, or a preset states it for the row.
///
/// The field exists for one reason: `thinking = false` has to be expressible.
/// That only matters for families that reason *by default* — DeepSeek V4 and
/// Qwen3 both do — so those are the two named arms. [`Reasoning::None`] is every
/// other endpoint, reasoning models included: there is nothing to suppress, so
/// there is nothing to send. An unknown field is a 400 on a strict endpoint, not
/// a field politely ignored, which is why the default is to send nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reasoning {
    /// `thinking: {"type": "enabled"|"disabled"}` — DeepSeek's own API.
    Deepseek,
    /// `chat_template_kwargs: {"enable_thinking": bool}` — Qwen3 behind vLLM,
    /// SGLang or DashScope.
    Qwen,
    /// Nothing on the wire either way. The endpoint's own default stands.
    #[default]
    None,
}

impl Reasoning {
    /// The arm a pre-provider `[api] base_url` folds into (ADR-0021).
    ///
    /// A host guess, and the *only* one in the codebase: it runs once, on a file
    /// written before providers existed, whose `base_url` defaulted to DeepSeek's
    /// own host. Everywhere else the row says what it speaks.
    fn guess(base_url: &str) -> Self {
        let host = host_of(base_url);
        if host.contains("deepseek.com") {
            Self::Deepseek
        } else if host.contains("dashscope") {
            Self::Qwen
        } else {
            Self::None
        }
    }
}

/// One endpoint the user keeps: where requests go, what they carry, and which
/// credential account they are signed with (ADR-0021).
///
/// There is no `active` row and no global switch. Which provider a turn goes to
/// is an Action-level question with a default, exactly like `model` and
/// `thinking` already were — so two endpoints can be live at once, which is the
/// whole reason this is a table rather than a single `base_url`.
///
/// Deliberately no `auth` field: the Bearer header is sent when a key is stored
/// for this row and not when there is none. An explicit field could be wrong in
/// two ways a rule cannot — `none` beside a stored key ignores it silently,
/// `bearer` with no key refuses a turn the endpoint would have served.
///
/// A defaulted row is blank throughout — no reasoning wire, no temperature, no
/// key — which is a working local endpoint, and the safest thing an unknown host
/// can be. That is exactly what `derive(Default)` produces, so it is derived
/// rather than written out: a field added here cannot then be forgotten there.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Provider {
    /// Identity, and the credential account: `provider:{id}` ([`crate::secrets`]).
    /// An Action's `[model] provider` names this.
    pub id: String,
    /// Display only, like an Action's `name`.
    pub label: String,
    pub base_url: String,
    /// Per provider, because model ids do not transfer between endpoints.
    pub model: String,
    /// Was `[defaults] thinking`. Per provider now, because whether it can be
    /// honoured at all is a fact about the endpoint. Actions still override it.
    pub thinking: bool,
    pub reasoning: Reasoning,
    /// Omitted means send no `temperature` and let the endpoint decide — which
    /// is the only honest answer for an endpoint we know nothing about.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Where this provider's keys come from, for the "Get a key" link. Data on
    /// the row rather than a constant, because every vendor has a different one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_page: Option<String>,
}

impl Provider {
    /// The row a fresh install gets, and what a pre-provider config folds into.
    pub fn deepseek() -> Self {
        Self {
            id: DEFAULT_PROVIDER_ID.to_string(),
            label: DEFAULT_PROVIDER_LABEL.to_string(),
            base_url: DEFAULT_BASE_URL.to_string(),
            model: DEFAULT_MODEL.to_string(),
            // DeepSeek thinks *by default*, which is pure latency for
            // translation-shaped Actions — hence `false`, sent explicitly.
            thinking: false,
            reasoning: Reasoning::Deepseek,
            temperature: Some(DEEPSEEK_TEMPERATURE),
            key_page: Some(DEFAULT_KEY_PAGE.to_string()),
        }
    }

    /// Whether a missing key here is a local setup rather than a mistake.
    ///
    /// Loopback and the three private ranges only: a host we cannot place is
    /// treated as remote, because sending nothing to something that wanted a key
    /// fails as a 401 the user then has to decode.
    pub fn is_local(&self) -> bool {
        let host = host_of(&self.base_url);
        const LOCAL: [&str; 6] = ["localhost", "127.", "0.0.0.0", "[::1]", "192.168.", "10."];
        if LOCAL.iter().any(|prefix| host.starts_with(prefix)) {
            return true;
        }
        // 172.16.0.0/12 — the one private range whose prefix is not a literal.
        host.strip_prefix("172.")
            .and_then(|rest| rest.split('.').next())
            .and_then(|octet| octet.parse::<u8>().ok())
            .is_some_and(|octet| (16..=31).contains(&octet))
    }

    /// Whether the documented DeepSeek catalog describes what this endpoint
    /// serves ([`crate::llm::models`]). The host, not the dialect: a vLLM
    /// speaking DeepSeek's `thinking` object serves its own ids, not these.
    pub fn is_deepseek_host(&self) -> bool {
        // The authority, matched whole: a substring test over the entire URL
        // also answers yes to a path or a query that merely mentions the host.
        let host = host_of(&self.base_url);
        host == DEEPSEEK_HOST || host.starts_with(&format!("{DEEPSEEK_HOST}:"))
    }
}

/// The authority of a `base_url`: trimmed, scheme dropped, path dropped,
/// lowercased.
///
/// One normaliser for all three host rules, so "does this look local", "which
/// dialect does a pre-provider file speak" and "is this DeepSeek's own host"
/// cannot disagree about what they are reading.
fn host_of(base_url: &str) -> String {
    // Lowercased before the scheme is stripped, not after: a scheme is
    // case-insensitive, and `HTTP://` is a URL a person types.
    let url = base_url.trim().to_ascii_lowercase();
    let authority = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(&url);
    authority.split('/').next().unwrap_or(authority).to_string()
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// Every endpoint the user keeps. **Empty means "the file said nothing"**,
    /// which is what [`Config::fold_legacy`] fills in — so this default has to
    /// stay empty, and every reader gets its non-emptiness from the load path
    /// rather than from here.
    pub providers: Vec<Provider>,
    /// The pre-provider single endpoint (ADR-0021). Read once, folded into a
    /// row, and never written back — so it leaves the file on the next save,
    /// the same way a withdrawn `[model]` key does (ADR-0019).
    #[serde(skip_serializing)]
    pub base_url: Option<String>,
}

/// "Add from preset": a filled-in row per first-party endpoint (ADR-0021).
///
/// **Data, not code paths.** Every arm of this list produces an ordinary
/// [`Provider`] the user can then edit or delete, so nothing downstream knows a
/// row came from here. What a preset buys is the one field a person cannot look
/// up: [`Reasoning`], which is a wire fact about the endpoint.
///
/// ## What may be in this list
///
/// **The request has to terminate at the company whose key it carries.** No
/// aggregator, no gateway, no OpenRouter.
///
/// That is the line, and it is not "does this company own the model". Groq,
/// SiliconFlow and a hosted vLLM all serve somebody else's open weights on their
/// own GPUs — your key is theirs, the inference is theirs, and nothing is
/// forwarded. A broker is different in kind: it holds keys to *other* APIs and
/// relays your request to one, so a third party ends up inside a relationship
/// that was between you and a vendor. `every_preset_goes_direct_to_its_own_vendor`
/// is what keeps that true, because a broker would satisfy the type and nothing
/// else in the codebase would notice.
///
/// ## `model` may be empty
///
/// A starting value, not a claim. It is filled where the vendor publishes a
/// stable id — an alias like `mistral-medium-latest`, or a plain family name —
/// and left empty where their ids carry dated `-preview` suffixes that rot. An
/// id that has rotted is a `400` the user has to decode; an empty one sends them
/// to the dropdown, which is where the endpoint's own list lands anyway.
///
/// ## Why Rust
///
/// A wrong `reasoning` here is a `400` on every turn, so it belongs beside the
/// enum that documents the wire.
pub fn presets() -> Vec<Provider> {
    let row = |id: &str, label: &str, base_url: &str, model: &str, key_page: &str| Provider {
        id: id.to_string(),
        label: label.to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        thinking: false,
        reasoning: Reasoning::None,
        temperature: None,
        key_page: (!key_page.is_empty()).then(|| key_page.to_string()),
    };
    vec![
        Provider::deepseek(),
        // `None`, not an `openai` arm. `reasoning_effort` is the nearest
        // thing OpenAI has, and its floor is `minimal` rather than off — the
        // model still reasons, so sending it for `thinking = false` would be
        // claiming something untrue. It is also not accepted uniformly: the
        // o-series takes low/medium/high and rejects `minimal`, which would put
        // per-model knowledge back into a field that exists to avoid it.
        row(
            "openai",
            "OpenAI",
            "https://api.openai.com/v1",
            "gpt-5-mini",
            "https://platform.openai.com/api-keys",
        ),
        row(
            "groq",
            "Groq",
            "https://api.groq.com/openai/v1",
            "llama-3.3-70b-versatile",
            "https://console.groq.com/keys",
        ),
        row(
            "xai",
            "xAI",
            "https://api.x.ai/v1",
            "grok-4",
            "https://console.x.ai",
        ),
        row(
            "mistral",
            "Mistral",
            "https://api.mistral.ai/v1",
            "mistral-medium-latest",
            "https://console.mistral.ai/api-keys",
        ),
        // Mainland China's host. `api.moonshot.ai` is the international one —
        // same API, different account, so it is an edit to `base_url` rather
        // than a second row. No model: Moonshot's ids carry dated `-preview`
        // suffixes, so anything here rots.
        row(
            "moonshot",
            "Moonshot (Kimi)",
            "https://api.moonshot.cn/v1",
            "",
            "https://platform.moonshot.cn/console/api-keys",
        ),
        row(
            "zhipu",
            "Zhipu (GLM)",
            "https://open.bigmodel.cn/api/paas/v4",
            "glm-4.6",
            "https://open.bigmodel.cn/usercenter/apikeys",
        ),
        // Ids here are `org/model`, and `reasoning` is `None` even for a
        // DeepSeek-weighted one: SiliconFlow serves it behind its own stack,
        // which speaks plain OpenAI. This row is the example ADR-0021 uses for
        // why the dialect cannot be read off a model id.
        row(
            "siliconflow",
            "SiliconFlow",
            "https://api.siliconflow.cn/v1",
            "deepseek-ai/DeepSeek-V3",
            "https://cloud.siliconflow.cn/account/ak",
        ),
        Provider {
            // Qwen3 through Alibaba's compatible mode: the one hosted endpoint
            // in this list that takes the `chat_template_kwargs` form.
            // `dashscope-intl.aliyuncs.com` is the international host, again an
            // edit to `base_url` rather than a row of its own.
            reasoning: Reasoning::Qwen,
            ..row(
                "dashscope",
                "Alibaba DashScope",
                "https://dashscope.aliyuncs.com/compatible-mode/v1",
                "qwen3-max",
                "https://bailian.console.aliyun.com",
            )
        },
        row(
            "ollama",
            "Ollama (local)",
            "http://localhost:11434/v1",
            "",
            "",
        ),
        row(
            "lmstudio",
            "LM Studio (local)",
            "http://localhost:1234/v1",
            "",
            "",
        ),
        Provider {
            // A Qwen3 chat template is the usual reason to run vLLM by hand,
            // and it is the case a user cannot discover for themselves.
            reasoning: Reasoning::Qwen,
            ..row("vllm", "vLLM (Qwen3)", "http://localhost:8000/v1", "", "")
        },
    ]
}

/// What an Action that overrides nothing gets.
///
/// One field, and it names a *row* rather than restating one: `model` and
/// `thinking` moved onto [`Provider`], because which of them can be honoured is
/// a fact about the endpoint (ADR-0021). An Action's `[model]` table overrides
/// the resolved row field-by-field; see
/// [`ModelOverrides::merge_over`](crate::action::ModelOverrides).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelDefaults {
    /// A [`Provider::id`]. Not "active": nothing is — this is only what an
    /// Action that names no provider falls back to.
    pub provider: String,
    /// Pre-provider `[defaults] model` / `thinking`, folded onto the row this
    /// migration synthesises and then dropped from the file (ADR-0021).
    #[serde(skip_serializing)]
    pub model: Option<String>,
    #[serde(skip_serializing)]
    pub thinking: Option<bool>,
}

/// The size the Popover is summoned at, in logical pixels (ADR-0018).
///
/// It is config rather than window state because the window is created hidden at
/// startup and re-sized on every trigger (ADR-0007): a size that lived only in
/// the window would be overwritten by the next summon, so remembering a drag at
/// all means writing it to the file ADR-0003 makes authoritative.
///
/// Logical, not physical: the same file has to mean the same window on a 100%
/// monitor and a 150% one, and it is what `set_size` takes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PopoverSize {
    pub width: f64,
    pub height: f64,
}

impl PopoverSize {
    /// Clamped rather than refused: the value arrives from a drag, and a window
    /// dragged to the edge of the plausible is not an error to report. A `NaN`
    /// loses both comparisons, so it falls back to the default instead of
    /// poisoning the placement arithmetic.
    pub fn clamped(self) -> Self {
        Self {
            width: clamp_or_default(self.width, MIN_POPOVER_W, MAX_POPOVER_W, DEFAULT_POPOVER_W),
            height: clamp_or_default(self.height, MIN_POPOVER_H, MAX_POPOVER_H, DEFAULT_POPOVER_H),
        }
    }
}

fn clamp_or_default(value: f64, min: f64, max: f64, fallback: f64) -> f64 {
    if !value.is_finite() {
        return fallback;
    }
    value.clamp(min, max)
}

impl Default for PopoverSize {
    fn default() -> Self {
        Self {
            width: DEFAULT_POPOVER_W,
            height: DEFAULT_POPOVER_H,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut config = Self {
            launcher_hotkey: DEFAULT_LAUNCHER_HOTKEY.to_string(),
            autostart: true,
            theme: Theme::default(),
            language: Language::default(),
            defaults: ModelDefaults::default(),
            popover: PopoverSize::default(),
            api: ApiConfig::default(),
        };
        // The fresh DeepSeek row comes from the same place a migrated one does,
        // so "what a new install has" cannot drift from "what an old file
        // becomes" — and a `Config` handed out anywhere already holds the
        // invariants everything downstream leans on.
        config.fold_legacy();
        config
    }
}

impl Default for ModelDefaults {
    fn default() -> Self {
        Self {
            provider: DEFAULT_PROVIDER_ID.to_string(),
            model: None,
            thinking: None,
        }
    }
}

impl ApiConfig {
    pub fn find(&self, id: &str) -> Option<&Provider> {
        self.providers.iter().find(|one| one.id == id)
    }
}

impl Config {
    /// The row a turn goes to: the Action's override if it named one, otherwise
    /// `[defaults] provider`.
    ///
    /// `None` means the config names a row that is not there — a hand-edit, or a
    /// row removed while an Exchange was open. Reported at request time rather
    /// than papered over with the first row: sending to a different endpoint
    /// than the file says is worse than refusing.
    pub fn provider(&self, id: Option<&str>) -> Option<&Provider> {
        self.api.find(self.provider_id(id))
    }

    /// Which row an override resolves to: the id it named, or `[defaults]
    /// provider`.
    ///
    /// One accessor rather than the fallback re-spelled per caller — which row
    /// an Action that says nothing inherits is the load-bearing rule of
    /// ADR-0021, and a second spelling of it is a dropdown that disagrees with
    /// the wire. Borrowed, so a caller comparing ids allocates nothing.
    pub fn provider_id<'a>(&'a self, id: Option<&'a str>) -> &'a str {
        id.unwrap_or(&self.defaults.provider)
    }

    /// Bring a pre-provider file up to date, in memory only (ADR-0021).
    ///
    /// The file itself is left as the user left it until something writes it —
    /// silently rewriting a config on load is the data loss ADR-0003 warns
    /// about — but the legacy keys never serialise, so the first save drops them.
    ///
    /// Also the invariants everything downstream leans on: `providers` is never
    /// empty, every id is distinct, and `defaults.provider` always names a row
    /// that exists. Which is why this is not private to the load path — the IPC
    /// boundary folds too, so a table arriving from a window cannot be the one
    /// thing on disk that breaks them.
    pub(crate) fn fold_legacy(&mut self) {
        if self.api.providers.is_empty() {
            let base_url = self
                .api
                .base_url
                .clone()
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
            let reasoning = Reasoning::guess(&base_url);
            self.api.providers.push(Provider {
                base_url,
                model: self
                    .defaults
                    .model
                    .clone()
                    .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
                thinking: self.defaults.thinking.unwrap_or(false),
                reasoning,
                // The pinned 1.3 was DeepSeek's own guidance (ADR-0019), so it
                // travels only where the dialect says DeepSeek. Anywhere else the
                // value was already a guess about somebody's endpoint.
                temperature: (reasoning == Reasoning::Deepseek).then_some(DEEPSEEK_TEMPERATURE),
                ..Provider::deepseek()
            });
        }

        // Blank ids cannot be addressed by an Action, so they are not rows —
        // and neither is the second of two rows sharing one: the id *is* the
        // credential account (`provider:{id}`), so a duplicate would hand one
        // row another's key, and `ApiConfig::find` would shadow it anyway.
        let mut seen = HashSet::new();
        self.api
            .providers
            .retain(|one| !one.id.trim().is_empty() && seen.insert(one.id.clone()));
        if self.api.providers.is_empty() {
            self.api.providers.push(Provider::deepseek());
        }
        if self.api.find(&self.defaults.provider).is_none() {
            self.defaults.provider = self.api.providers[0].id.clone();
        }
    }
}

/// The result of loading `config.toml`.
///
/// A corrupt config is *reported*, not overwritten — silently replacing a file
/// the user was editing is the data loss ADR-0003 warns about.
pub struct Loaded {
    pub config: Config,
    pub error: Option<String>,
}

pub fn load_or_create(path: &Path) -> Loaded {
    match fs::read_to_string(path) {
        Ok(text) => match toml::from_str::<Config>(&text) {
            // The one field clamped on the way in rather than reported: it is
            // written by a drag, not typed, so a value out of range is this
            // program's own bug or a hand-edit — and either way the window has
            // to be some size. The file is left as the user left it.
            Ok(mut config) => {
                config.popover = config.popover.clamped();
                config.fold_legacy();
                Loaded {
                    config,
                    error: None,
                }
            }
            Err(err) => Loaded {
                config: Config::default(),
                error: Some(format!("config.toml could not be parsed: {err}")),
            },
        },
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let config = Config::default();
            let error = save(path, &config)
                .err()
                .map(|e| format!("config.toml could not be written: {e}"));
            Loaded { config, error }
        }
        Err(err) => Loaded {
            config: Config::default(),
            error: Some(format!("config.toml could not be read: {err}")),
        },
    }
}

pub fn save(path: &Path, config: &Config) -> Result<(), String> {
    let text = toml::to_string_pretty(config).map_err(|e| e.to_string())?;
    write_atomic(path, &text).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_writes_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let loaded = load_or_create(&path);
        assert!(loaded.error.is_none());
        assert_eq!(loaded.config, Config::default());
        assert!(path.exists(), "defaults must be persisted");

        // Round-trips through what we just wrote.
        assert_eq!(load_or_create(&path).config, Config::default());
    }

    /// `fold_legacy` runs on the load path, not on `from_str`, so a parse alone
    /// is where the raw shape is checked and `loaded` is where the invariants
    /// are. Both matter: the first is the file, the second is what state holds.
    fn loaded(text: &str) -> Config {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, text).unwrap();
        let loaded = load_or_create(&path);
        assert!(loaded.error.is_none(), "{:?}", loaded.error);
        loaded.config
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let parsed = loaded("autostart = false\n");
        assert!(!parsed.autostart);
        assert_eq!(parsed.launcher_hotkey, DEFAULT_LAUNCHER_HOTKEY);
        assert_eq!(parsed.defaults.provider, DEFAULT_PROVIDER_ID);
        assert_eq!(parsed.api.providers, vec![Provider::deepseek()]);
    }

    /// The whole point of the layer: DeepSeek out of the box, and one row.
    #[test]
    fn a_fresh_install_has_exactly_the_deepseek_row() {
        let config = Config::default();
        let row = config.provider(None).unwrap();
        assert_eq!(row.id, DEFAULT_PROVIDER_ID);
        assert_eq!(row.base_url, DEFAULT_BASE_URL);
        assert_eq!(row.model, DEFAULT_MODEL);
        assert_eq!(row.reasoning, Reasoning::Deepseek);
        assert_eq!(row.temperature, Some(DEEPSEEK_TEMPERATURE));
        assert!(!row.is_local());
        assert!(row.is_deepseek_host());
    }

    /// A pre-provider file: one `base_url`, one `[defaults] model`. It must keep
    /// working, and it must keep pointing at the same endpoint with the same
    /// model — a migration that silently changes where text goes is worse than
    /// refusing to load (ADR-0021).
    #[test]
    fn a_pre_provider_config_folds_into_one_row() {
        let config = loaded(
            "[api]\nbase_url = \"https://api.deepseek.com\"\n\n\
             [defaults]\nmodel = \"deepseek-v4-pro\"\nthinking = true\n",
        );
        assert_eq!(config.api.providers.len(), 1);
        let row = config.provider(None).unwrap();
        assert_eq!(row.base_url, "https://api.deepseek.com");
        assert_eq!(row.model, "deepseek-v4-pro");
        assert!(row.thinking);
        assert_eq!(row.reasoning, Reasoning::Deepseek);
        assert_eq!(config.defaults.provider, DEFAULT_PROVIDER_ID);
    }

    /// The one host guess in the codebase, and why it is safe: a `base_url`
    /// pointed somewhere else was never speaking DeepSeek's dialect, so folding
    /// it in as `deepseek` would start sending a `thinking` object that endpoint
    /// has always 400'd on. The pinned 1.3 goes with it (ADR-0019 → ADR-0021).
    #[test]
    fn a_pre_provider_config_pointed_elsewhere_folds_in_as_plain_openai() {
        let config = loaded("[api]\nbase_url = \"http://localhost:11434/v1\"\n");
        let row = config.provider(None).unwrap();
        assert_eq!(row.reasoning, Reasoning::None);
        assert_eq!(row.temperature, None);
        assert!(row.is_local());
        assert!(!row.is_deepseek_host());
    }

    /// Legacy keys are read and never written: the first save drops them, the
    /// way a withdrawn `[model]` key does (ADR-0019).
    #[test]
    fn the_legacy_keys_leave_the_file_on_the_next_write() {
        let config = loaded("[api]\nbase_url = \"https://api.deepseek.com\"\n");
        let text = toml::to_string_pretty(&config).unwrap();
        assert!(
            !text.contains("base_url = \"https://api.deepseek.com\"\n[popover]"),
            "{text}"
        );
        // The row carries it now, so the string is still there — under
        // `[[api.providers]]`, and the `[api]`-level key is gone.
        let again: Config = toml::from_str(&text).unwrap();
        assert_eq!(again.api.base_url, None);
        assert_eq!(again.defaults.model, None);
        assert_eq!(again.api.providers.len(), 1);
    }

    /// Nothing downstream may face an empty table or a default naming a row
    /// that is not there — `provider()` is the only lookup, and a hand-edit is
    /// the one thing that can produce either.
    #[test]
    fn a_hand_edit_cannot_leave_the_table_unusable() {
        // A default naming a row that was deleted falls back to the first row.
        let config = loaded(
            "[defaults]\nprovider = \"gone\"\n\n\
             [[api.providers]]\nid = \"ollama\"\nbase_url = \"http://localhost:11434/v1\"\n",
        );
        assert_eq!(config.defaults.provider, "ollama");
        // A row with no id cannot be named by an Action, so it is not a row.
        let config = loaded("[[api.providers]]\nlabel = \"nameless\"\n");
        assert_eq!(config.api.providers, vec![Provider::deepseek()]);
        // Nor is the second of two rows sharing an id: the id *is* the
        // credential account, so a duplicate would hand one row another's key —
        // and `ApiConfig::find` would never reach it anyway.
        let config = loaded(
            "[[api.providers]]\nid = \"ollama\"\nbase_url = \"http://localhost:11434/v1\"\n\n\
             [[api.providers]]\nid = \"ollama\"\nbase_url = \"https://api.openai.com/v1\"\n",
        );
        assert_eq!(config.api.providers.len(), 1);
        assert_eq!(
            config.api.providers[0].base_url,
            "http://localhost:11434/v1"
        );
    }

    /// The three host rules read the same thing: the authority, lowercased, with
    /// no scheme and no path — so a URL that merely *mentions* DeepSeek's host is
    /// not DeepSeek's host.
    #[test]
    fn host_rules_read_the_authority_only() {
        let row = |base_url: &str| Provider {
            base_url: base_url.into(),
            ..Provider::default()
        };
        assert!(row("HTTP://LocalHost:8000/v1").is_local());
        assert!(row("http://172.20.0.5:11434/v1").is_local());
        assert!(!row("https://api.deepseek.com/v1").is_local());
        assert!(row("https://api.deepseek.com/v1").is_deepseek_host());
        assert!(row("https://API.DeepSeek.com").is_deepseek_host());
        assert!(!row("https://proxy.example.com/api.deepseek.com/v1").is_deepseek_host());
    }

    /// An Action's override wins; naming nothing takes the default; naming a row
    /// that is not there is `None`, reported rather than redirected.
    #[test]
    fn provider_lookup_prefers_the_override() {
        let config = loaded(
            "[defaults]\nprovider = \"deepseek\"\n\n\
             [[api.providers]]\nid = \"deepseek\"\nbase_url = \"https://api.deepseek.com\"\n\n\
             [[api.providers]]\nid = \"ollama\"\nbase_url = \"http://localhost:11434/v1\"\n",
        );
        assert_eq!(config.provider(None).unwrap().id, "deepseek");
        assert_eq!(config.provider(Some("ollama")).unwrap().id, "ollama");
        assert!(config.provider(Some("nope")).is_none());
    }

    #[test]
    fn a_provider_table_survives_a_toml_round_trip() {
        let mut config = Config::default();
        config.api.providers.push(Provider {
            id: "ollama".into(),
            label: "Ollama (local)".into(),
            base_url: "http://localhost:11434/v1".into(),
            model: "qwen3:8b".into(),
            thinking: true,
            reasoning: Reasoning::Qwen,
            temperature: None,
            key_page: None,
        });
        config.defaults.provider = "ollama".into();
        let text = toml::to_string_pretty(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
        // The array of tables is last, so no table header lands inside it.
        assert!(
            text.find("[popover]") < text.find("[[api.providers]]"),
            "{text}"
        );
    }

    /// The product decision, as a test: every hosted preset is a host that
    /// **terminates** the request, reached over TLS (ADR-0021).
    ///
    /// The names below are brokers — they hold keys to other APIs and relay to
    /// one of them. An inference provider serving somebody else's open weights
    /// on its own GPUs is not one of these and is welcome in the list; Groq and
    /// SiliconFlow are both already in it.
    #[test]
    fn every_preset_goes_direct_to_its_own_vendor() {
        const BROKERS: [&str; 6] = [
            "openrouter",
            "litellm",
            "requesty",
            "helicone",
            "portkey",
            "unify.ai",
        ];
        let presets = presets();
        assert!(presets.len() > 1);
        for row in &presets {
            assert!(!row.id.trim().is_empty(), "{}", row.label);
            assert!(!row.label.trim().is_empty(), "{}", row.id);
            let host = row.base_url.to_ascii_lowercase();
            for name in BROKERS {
                assert!(!host.contains(name), "{} relays through {name}", row.id);
            }
            if row.is_local() {
                // A local server is plain HTTP and needs no key page.
                assert!(row.key_page.is_none(), "{}", row.id);
            } else {
                assert!(host.starts_with("https://"), "{} is not TLS", row.id);
                assert!(row.key_page.is_some(), "{} has no key page", row.id);
            }
        }
        // Ids are the credential account, so a duplicate would share a key.
        let mut ids: Vec<&str> = presets.iter().map(|one| one.id.as_str()).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate preset id");
    }

    /// The default row and the preset of the same id must not disagree: the
    /// former is what a fresh install has, the latter what "add DeepSeek back"
    /// gives you, and a difference between them is invisible.
    #[test]
    fn the_deepseek_preset_is_the_default_row() {
        let preset = presets()
            .into_iter()
            .find(|one| one.id == DEFAULT_PROVIDER_ID);
        assert_eq!(preset, Some(Provider::deepseek()));
    }

    /// Every private range and loopback spelling, because "no key stored" is a
    /// fault on a remote host and a working setup on a local one.
    #[test]
    fn local_hosts_are_told_from_remote_ones() {
        let local = [
            "http://localhost:11434/v1",
            "http://127.0.0.1:8000/v1",
            "http://192.168.1.9:1234/v1",
            "http://10.0.0.4:8000",
            "http://172.16.0.2:8000",
            "http://172.31.255.1:8000",
            "http://[::1]:8000/v1",
        ];
        for base_url in local {
            let row = Provider {
                base_url: base_url.into(),
                ..Provider::default()
            };
            assert!(row.is_local(), "{base_url}");
        }
        for base_url in [
            "https://api.deepseek.com",
            "https://api.openai.com/v1",
            // Outside 172.16/12 — a public address that merely looks close.
            "http://172.32.0.1:8000",
            "http://172.15.0.1:8000",
        ] {
            let row = Provider {
                base_url: base_url.into(),
                ..Provider::default()
            };
            assert!(!row.is_local(), "{base_url}");
        }
    }

    #[test]
    fn absent_theme_is_light() {
        assert_eq!(Config::default().theme, Theme::Light);
        assert_eq!(
            toml::from_str::<Config>("autostart = false\n")
                .unwrap()
                .theme,
            Theme::Light
        );
        // Not "whatever the OS is set to": `system` has to be asked for.
        assert_eq!(toml::from_str::<Config>("").unwrap().theme, Theme::Light);
    }

    #[test]
    fn absent_language_is_english() {
        assert_eq!(Config::default().language, Language::En);
        assert_eq!(toml::from_str::<Config>("").unwrap().language, Language::En);
        // Not the OS locale: Chinese has to be asked for.
        assert_eq!(
            toml::from_str::<Config>("language = \"zh\"\n")
                .unwrap()
                .language,
            Language::Zh
        );
    }

    #[test]
    fn language_survives_a_toml_round_trip() {
        for language in [Language::En, Language::Zh] {
            let config = Config {
                language,
                ..Config::default()
            };
            let text = toml::to_string_pretty(&config).unwrap();
            assert!(!text.contains("[api]\nlanguage"), "{text}");
            assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
        }
    }

    #[test]
    fn every_theme_round_trips_through_toml() {
        for theme in [Theme::Light, Theme::Dark, Theme::System] {
            let config = Config {
                theme,
                ..Config::default()
            };
            let text = toml::to_string_pretty(&config).unwrap();
            assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
        }
    }

    #[test]
    fn theme_is_written_as_a_lowercase_string() {
        let text = toml::to_string_pretty(&Config::default()).unwrap();
        assert!(text.contains("theme = \"light\""), "{text}");
    }

    #[test]
    fn theme_survives_a_file_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let config = Config {
            theme: Theme::Dark,
            ..Config::default()
        };

        save(&path, &config).unwrap();
        let loaded = load_or_create(&path);
        assert!(loaded.error.is_none());
        assert_eq!(loaded.config.theme, Theme::Dark);
    }

    #[test]
    fn absent_popover_size_is_the_default() {
        let parsed: Config = toml::from_str("").unwrap();
        assert_eq!(parsed.popover.width, DEFAULT_POPOVER_W);
        assert_eq!(parsed.popover.height, DEFAULT_POPOVER_H);
    }

    /// One dimension named is one dimension changed: the container-level
    /// `serde(default)` fills the other from [`PopoverSize::default`], not from
    /// `f64::default`, which would be a zero-width window.
    #[test]
    fn half_a_popover_size_keeps_the_other_default() {
        let parsed: Config = toml::from_str("[popover]\nwidth = 900\n").unwrap();
        assert_eq!(parsed.popover.width, 900.0);
        assert_eq!(parsed.popover.height, DEFAULT_POPOVER_H);
    }

    #[test]
    fn popover_size_survives_a_toml_round_trip() {
        let config = Config {
            popover: PopoverSize {
                width: 900.0,
                height: 700.0,
            },
            ..Config::default()
        };
        let text = toml::to_string_pretty(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
    }

    #[test]
    fn an_out_of_range_popover_size_is_clamped_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "[popover]\nwidth = 12\nheight = 99999\n").unwrap();

        let loaded = load_or_create(&path);
        assert!(loaded.error.is_none());
        assert_eq!(loaded.config.popover.width, MIN_POPOVER_W);
        assert_eq!(loaded.config.popover.height, MAX_POPOVER_H);
        // Clamping is ours, not a rewrite of the user's file (ADR-0003).
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "[popover]\nwidth = 12\nheight = 99999\n"
        );
    }

    #[test]
    fn a_nonsense_popover_size_falls_back_to_the_default() {
        let clamped = PopoverSize {
            width: f64::NAN,
            height: f64::INFINITY,
        }
        .clamped();
        assert_eq!(clamped, PopoverSize::default());
    }

    #[test]
    fn unknown_fields_are_tolerated() {
        let parsed: Config =
            toml::from_str("launcher_hotkey = \"Ctrl+Alt+K\"\nfuture_option = 3\n").unwrap();
        assert_eq!(parsed.launcher_hotkey, "Ctrl+Alt+K");
    }

    /// The hotkey is interpolated rather than spelled out: it is the one
    /// default that differs per platform, and the rest of the example is what
    /// this test is actually about.
    #[test]
    fn readme_example_parses() {
        let text = format!(
            r#"
launcher_hotkey = "{DEFAULT_LAUNCHER_HOTKEY}"
autostart = true
theme = "light"
language = "en"

[defaults]
provider = "deepseek"

[popover]
width = 620.0
height = 500.0

[[api.providers]]
id = "deepseek"
label = "DeepSeek"
base_url = "https://api.deepseek.com"
model = "deepseek-v4-flash"
thinking = false
reasoning = "deepseek"
temperature = 1.3
key_page = "https://platform.deepseek.com/api_keys"
"#
        );
        let parsed: Config = toml::from_str(&text).unwrap();
        assert_eq!(parsed, Config::default());
    }

    #[test]
    fn corrupt_config_reports_and_does_not_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "launcher_hotkey = ").unwrap();

        let loaded = load_or_create(&path);
        assert!(loaded.error.is_some());
        assert_eq!(loaded.config, Config::default());
        assert_eq!(fs::read_to_string(&path).unwrap(), "launcher_hotkey = ");
    }
}
