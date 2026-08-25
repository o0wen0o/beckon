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
    /// Whether the automatic once-per-launch update check runs (ADR-0022).
    ///
    /// Only the automatic one. The tray's own item is a click, and a click is
    /// never what this declines — turning it off means Beckon contacts nothing
    /// on its own, not that it refuses to answer when asked.
    pub update_check: bool,
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
/// A property of the *endpoint*, never of the model: a DeepSeek-weighted model
/// served by SiliconFlow speaks the plain OpenAI dialect, so no rule over model
/// ids can produce this — the row states it, or a preset states it for the row.
///
/// The field exists for one reason: `thinking = false` has to be expressible.
/// That only matters for endpoints that reason *by default* and document a way
/// to stop — so the named arms are exactly the dialects of that switch, and
/// [`Reasoning::None`] is every other endpoint, reasoning models included: there
/// is nothing to suppress, so there is nothing to send. An unknown field is a
/// 400 on a strict endpoint, not a field politely ignored, which is why the
/// default is to send nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Reasoning {
    /// `thinking: {"type": "enabled"|"disabled"}` — DeepSeek's own API.
    Deepseek,
    /// `chat_template_kwargs: {"enable_thinking": bool}` — Qwen3 behind vLLM,
    /// SGLang or DashScope.
    Qwen,
    /// `reasoning_effort: "none"` — OpenAI's own API, where `none` is a real
    /// floor rather than a low setting: the model answers without reasoning.
    /// Sent only to suppress; asking *for* thinking sends nothing, because the
    /// endpoint already reasons by default and naming a level the user never
    /// chose would be inventing one. The one arm whose host also serves models
    /// that reject the field, so `llm/request.rs` sends it only for the families
    /// documented there and stays silent for the rest.
    OpenAi,
    /// `thinking: {"type": "adaptive"|"disabled"}` plus `reasoning_split: true`
    /// — MiniMax's own API.
    ///
    /// Not a reuse of [`Reasoning::Deepseek`], which sends
    /// `{"type": "enabled"|"disabled"}`: `disabled` matches, but `enabled` is
    /// not a value MiniMax documents — `adaptive` is — so sharing the arm would
    /// be the exact failure this enum exists to prevent.
    ///
    /// `reasoning_split` rides along because without it MiniMax returns its
    /// thinking *inside* `content`, wrapped in `<think>` tags, and
    /// [`crate::llm::wire`] reads reasoning only from `reasoning_content` — so
    /// the Popover would render the tags as answer text. It is sent in both
    /// directions, because it says where thinking goes rather than whether to do
    /// any.
    ///
    /// A mixed host: M2.x accepts `disabled` and keeps thinking anyway, so
    /// `llm/request.rs` keeps a deny-list and *refuses* rather than sending it.
    Minimax,
    /// `reasoning: {"effort": "none"}` — OpenRouter's own parameter, which it
    /// translates into whatever the model behind it speaks.
    ///
    /// `enabled: false` is not documented (only `enabled: true`), and
    /// `exclude: true` hides the thinking rather than stopping it — the tokens
    /// are still paid for and the latency is still spent, which is the opposite
    /// of what `thinking = false` asks for.
    Openrouter,
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

/// How an endpoint is asked to **search the web** (ADR-0026).
///
/// The same shape as [`Reasoning`] and for the same reason: a property of the
/// *endpoint*, never of the model. What differs is the direction. Thinking is
/// something endpoints do unless stopped, so `Reasoning` names off-switches;
/// searching is something no endpoint does unless asked, so these name
/// on-switches — and [`Search::None`], the default, is every endpoint with no
/// switch Beckon can throw on a `/chat/completions` request.
///
/// The named arms are exactly the endpoints whose search is **one field and one
/// round trip**. A built-in tool the client has to answer — Moonshot's
/// `$web_search`, which replies with a `tool_calls` frame the caller must echo
/// back — is a second turn, and `exchange/turn.rs` streams one; those hosts stay
/// `None` until that is a feature rather than a field (ADR-0026).
///
/// Nothing detects these. `llm/detect.rs` probes a thinking dialect with a
/// one-token request; a search probe would run a real search and bill for it, so
/// a hand-made row states its arm and a preset states it for the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Search {
    /// `search_parameters: {"mode": "auto"|"off"}` — xAI's own API, on
    /// `/chat/completions` (checked 2026-08-25).
    ///
    /// The one arm that sends the **off** direction too: xAI documents `on` as
    /// the default for the object, and `auto` rather than `on` for the enabled
    /// direction because `on` searches every source on every turn whether the
    /// question needs it or not. Sending `off` explicitly is the same insurance
    /// `Reasoning::Deepseek` buys against a vendor default nobody chose.
    Xai,
    /// `enable_search: true` — Alibaba DashScope's compatible mode (checked
    /// 2026-08-25). Documented on this endpoint for the Qwen3.5-and-later Plus
    /// and Flash tiers and the `qwen-plus` alias; the Max tiers take web search
    /// through their Responses API instead, and a model this endpoint does not
    /// list is simply not documented as searching.
    ///
    /// The compatible mode does not return the search sources — that is
    /// DashScope's native protocol only — so the answer cites what it read and
    /// Beckon has nothing further to render.
    Dashscope,
    /// `plugins: [{"id": "web"}]` — OpenRouter, which runs the search itself and
    /// folds the results into the same completion (checked 2026-08-25). A
    /// broker, so this is a search *about* the request rather than by the model
    /// behind it, and it is billed per request on top of the tokens (ADR-0025,
    /// ADR-0026).
    Openrouter,
    /// Nothing on the wire either way. This endpoint has no one-field switch, so
    /// `web_search = true` reaches it as nothing and its own behaviour stands.
    #[default]
    None,
}

impl Search {
    /// Whether this endpoint's search field reaches a given model (ADR-0027).
    ///
    /// `None` is "the vendor documents neither", and the switch stays offered on
    /// it: an arm that answered `false` for every id it had not heard of would
    /// grey out each new model the day it shipped. `Some(false)` is the vendor's
    /// own word that this model does not take the field — the reason ADR-0027
    /// exists, and the only thing that disables a switch.
    ///
    /// Families rather than ids, deliberately. A list of exact model names is
    /// the thing ADR-0026 refused to keep, and the vendor documents these by
    /// tier; matching the tier ages at the speed the tier does.
    pub fn supports_model(self, model: &str) -> Option<bool> {
        let id = model.trim();
        if id.is_empty() {
            return None;
        }
        match self {
            // The field is the endpoint's own and every model behind it reads
            // it: xAI runs Live Search before the model sees the turn, and
            // OpenRouter is a broker running the search itself (ADR-0025).
            Search::Xai | Search::Openrouter => Some(true),
            // The one host with a documented split. Web search on the
            // OpenAI-compatible endpoint is the Qwen Plus and Flash tiers; the
            // Max tiers take it through a Responses API Beckon does not post to
            // (checked 2026-08-25). The only arm that reads the id, which is
            // why it is also the only one that pays for lowercasing it — a list
            // of a hundred ids is built one option at a time.
            Search::Dashscope => {
                let id = id.to_ascii_lowercase();
                // Both tiers under the same `qwen` guard. DashScope's
                // compatible endpoint serves other vendors' models too, and a
                // bare `contains("max")` would grey one of those on a claim
                // Alibaba never made about it.
                if !id.starts_with("qwen") {
                    None
                } else if id.contains("max") {
                    Some(false)
                } else if id.contains("plus") || id.contains("flash") {
                    Some(true)
                } else {
                    None
                }
            }
            // Not a fact about the model: there is no field here for any of
            // them, which is what `Provider::search` already says.
            Search::None => Some(false),
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
    /// What this row inherits to an Action that says nothing about searching
    /// (ADR-0026). `false` on every preset, because a search costs money and
    /// seconds on top of the turn and nobody asked for it by installing Beckon.
    pub web_search: bool,
    pub search: Search,
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
            // DeepSeek documents no search field on `/chat/completions`: the
            // web search their own products do rides their Anthropic-compatible
            // endpoint, which is a different protocol (checked 2026-08-25).
            web_search: false,
            search: Search::None,
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
        host_is_local(&host_of(&self.base_url))
    }

    /// The broker this row relays through, if it does (ADR-0025).
    ///
    /// Derived rather than stored: a field would have to be filled in by whoever
    /// added the row, which means a hand-typed OpenRouter URL would disclose
    /// nothing — the one row where the user is least likely to already know.
    ///
    /// The binary never calls this. What the pane draws from is the mirror in
    /// `src/lib/providers.ts`, beside the other four rules that answer what a
    /// row *says* rather than what goes on its wire — and the Rust half is what
    /// `every_relaying_preset_says_so` reads, so the rule and the preset list it
    /// polices stay in one file.
    #[allow(dead_code)]
    pub fn relays(&self) -> Option<&'static str> {
        let host = host_of(&self.base_url);
        BROKERS.into_iter().find(|name| host.contains(name))
    }
}

/// Loopback and the three private ranges only: a host we cannot place is
/// treated as remote, because sending nothing to something that wanted a key
/// fails as a 401 the user then has to decode.
///
/// Free-standing rather than a method, because [`normalise_base_url`] has to ask
/// it about a string before there is a `Provider` to ask about — and the scheme
/// it picks for a user who typed no scheme depends on the answer.
fn host_is_local(host: &str) -> bool {
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

/// The three ways a hand-typed `base_url` goes wrong, fixed rather than
/// reported.
///
/// This is the only field on the row a preset cannot fill in, so it is the only
/// one a user has to type — which makes it the one place a typo costs a turn.
/// Every rule here is *lossless*: it removes something that cannot belong in a
/// compatibility root, or supplies something that has exactly one right answer.
///
///  1. **A missing scheme.** `api.example.com` is what a person types. `https`,
///     unless the authority is loopback or private, where a local server
///     answers on `http` and `https` is the failure the user would have to
///     decode from a TLS error.
///  2. **A pasted request path.** Copying the URL out of a vendor's quickstart
///     yields `…/v1/chat/completions`, and [`crate::llm::client`] then posts to
///     `…/chat/completions/chat/completions`. `models` is deliberately *not*
///     stripped: `chat/completions` and `completions` are POST routes that can
///     never be a compatibility root, and a path segment called `models` might
///     legitimately be one.
///  3. **Trailing slashes**, which `api_url` tolerates anyway — normalised here
///     so the value the pane shows back is the value that goes on the wire.
///
/// Runs inside [`Config::fold_legacy`], so it holds for a file the user
/// hand-edited exactly as it does for a value a pane sent (ADR-0003, ADR-0021).
fn normalise_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    // Nothing to fix and nothing to invent: an empty row is the blank a pane
    // just created, and giving it a scheme would make it look configured.
    if trimmed.is_empty() {
        return String::new();
    }

    // Case-insensitively, and lowercased where it is there: a scheme is
    // case-insensitive and `HTTP://` is a URL a person types. `host_of` already
    // lowercases before it strips, so a case-sensitive test here would disagree
    // with it — `HTTP://localhost:1234/v1` would read as scheme-less, be given a
    // second one, and be written back by `fold_legacy` as an address no request
    // can reach.
    let with_scheme = match trimmed.split_once("://") {
        Some((scheme, rest))
            if scheme.eq_ignore_ascii_case("https") || scheme.eq_ignore_ascii_case("http") =>
        {
            format!("{}://{rest}", scheme.to_ascii_lowercase())
        }
        _ if host_is_local(&host_of(trimmed)) => format!("http://{trimmed}"),
        _ => format!("https://{trimmed}"),
    };

    // Longest first: `chat/completions` would otherwise leave a dangling `chat`.
    let stripped = ["/chat/completions", "/completions"]
        .iter()
        .find_map(|tail| with_scheme.strip_suffix(tail))
        .unwrap_or(&with_scheme);
    stripped.trim_end_matches('/').to_string()
}

/// Hosts that hold keys to *other* APIs and relay a request to one of them
/// (ADR-0025).
///
/// This was a ban list — no row was allowed to name one — and is now a
/// **disclosure** list: a row may point at a broker, and the pane says so before
/// the user stores a key. What changed is not the risk but who decides. A broker
/// is still a third party inside a relationship that was between the user and a
/// vendor, and Beckon still cannot say what it does with the text; the answer is
/// to state that plainly rather than to make the choice for someone.
///
/// A host match, deliberately, where [`Reasoning`] refuses one: being wrong here
/// costs a warning nobody needed, and being wrong there costs a `400` on every
/// turn. It also reaches a row the user typed by hand, which a field on the
/// preset never would — and that is the row most likely to have arrived at a
/// broker without meaning to.
#[allow(dead_code)]
const BROKERS: [&str; 6] = [
    "openrouter",
    "litellm",
    "requesty",
    "helicone",
    "portkey",
    "unify.ai",
];

/// The authority of a `base_url`: trimmed, scheme dropped, path dropped,
/// lowercased.
///
/// One normaliser for both host rules, so "does this look local" and "which
/// dialect does a pre-provider file speak" cannot disagree about what they are
/// reading. There used to be a third — `is_deepseek_host`, which decided whether
/// the documented catalog described this endpoint. A row carries no catalog now
/// (`docs/register-audit-2026-08-24.md`), so nothing asks.
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
/// Until ADR-0025 the rule was that the request had to **terminate** at the
/// company whose key it carries — no aggregator, no gateway, no OpenRouter. That
/// rule is now a disclosure instead of a ban: a row may relay, and
/// [`Provider::relays`] is what makes it say so before a key is stored.
///
/// The distinction the old rule drew is still the right one and still worth
/// stating. It is not "does this company own the model": a hosted vLLM serves
/// somebody else's open weights on its own GPUs — your key is theirs, the
/// inference is theirs, and nothing is forwarded. A broker is different in kind,
/// because it holds keys to *other* APIs and relays your request to one, so a
/// third party ends up inside a relationship that was between you and a vendor.
/// What changed is who gets to decide that is acceptable.
/// `every_relaying_preset_says_so` is what keeps the disclosure attached, because
/// a broker satisfies the type and nothing else in the codebase would notice.
///
/// ## A row carries no model
///
/// **`model` is empty on every row but `deepseek`.** A row says where to fetch
/// and how to connect; what an endpoint serves is the endpoint's own answer, read
/// from `GET {base_url}/models` and kept in [`crate::models_cache`]. A hand-kept
/// id rots silently — `glm-5.1` sat here two generations behind GLM-5.3, resolving
/// happily, with every gate in this repo green on it — and no test here reaches
/// the network, so nothing could have caught it.
///
/// `deepseek` is the one carve-out and it is the principle applied, not an
/// exception to it: it is the default row, the one a first-run user actually
/// reaches, and the only id in the codebase with dated provenance beside it
/// (`CATALOG`, which is load-bearing for `Thinking` regardless). Emptying it
/// would cost the first turn out of the box for nothing.
///
/// ## When these were last checked
///
/// `base_url` and `key_page` are vendor-side facts, and no test here reaches the
/// network — so this date is the only record of when they were last known good.
/// Both rot silently: a moved key page still `301`s, and it is the first link a
/// new user clicks. The 2026-08-24 pass found two moves and neither was a model.
///
/// **Checked 2026-08-24** against each vendor's own documentation.
///
/// ## Why Rust
///
/// A wrong `reasoning` here is a `400` on every turn, so it belongs beside the
/// enum that documents the wire.
pub fn presets() -> Vec<Provider> {
    // No `model` parameter: every row but `deepseek` leaves it empty, and
    // `Provider::deepseek` is where that one value lives.
    let row = |id: &str, label: &str, base_url: &str, key_page: &str| Provider {
        id: id.to_string(),
        label: label.to_string(),
        base_url: base_url.to_string(),
        model: String::new(),
        thinking: false,
        reasoning: Reasoning::None,
        // Off on every row, arm named per row below (ADR-0026).
        web_search: false,
        search: Search::None,
        temperature: None,
        key_page: (!key_page.is_empty()).then(|| key_page.to_string()),
    };
    vec![
        Provider::deepseek(),
        // This row was `None` for as long as `reasoning_effort`'s floor was
        // `minimal`: the model still reasoned at that setting, so sending it for
        // `thinking = false` claimed something untrue. GPT-5.6 added `none`,
        // which answers without reasoning, so the switch is real and the arm
        // says so — and `EFFORT_NONE_FAMILIES` in `llm/request.rs` is why it is
        // sent only for the families that document it, since this one host also
        // serves models the field is a 400 on.
        // `search` stays `None` on their own word: OpenAI documents web search
        // on `/chat/completions` for the search-specialised models only, and
        // those search on every turn with no field to ask them to — the switch
        // is the model id, which is a choice the model dropdown already offers.
        // Their general models take web search through the Responses API, which
        // is not the endpoint Beckon posts to (checked 2026-08-25).
        Provider {
            reasoning: Reasoning::OpenAi,
            ..row(
                "openai",
                "OpenAI",
                "https://api.openai.com/v1",
                "https://platform.openai.com/api-keys",
            )
        },
        // Their docs now brand the company SpaceXAI while every host stays on
        // `x.ai`; the label follows the hosts until the rename settles.
        Provider {
            // Live Search is a request field here rather than a tool, and it is
            // on the chat endpoint's own schema — the Responses API their docs
            // now lead with is where the *tool* form lives, and Beckon posts to
            // neither of those (checked 2026-08-25).
            search: Search::Xai,
            ..row("xai", "xAI", "https://api.x.ai/v1", "https://console.x.ai")
        },
        // `search` is `None` for a reason that is not "they have none": Kimi's
        // `$web_search` is declared as a `builtin_function` tool and answered
        // with a `tool_calls` frame the caller has to echo back before the
        // answer arrives. That is two round trips, and `exchange/turn.rs`
        // streams one (ADR-0026, checked 2026-08-25).
        //
        // Mainland China's host. `api.moonshot.ai` is the international one —
        // same API, different account, so it is an edit to `base_url` rather
        // than a second row. The key page moved: `platform.moonshot.cn` `301`s
        // to `platform.kimi.com`.
        row(
            "moonshot",
            "Moonshot (Kimi)",
            "https://api.moonshot.cn/v1",
            "https://platform.kimi.com/console/api-keys",
        ),
        // Mainland China's host, and the one row here whose path is versioned
        // with something other than `/v1` — `client::api_url` reads *every* path
        // segment for a version because of this row, which until then posted to
        // `/api/paas/v4/v1/chat/completions`. `thinking` is documented as opt-in
        // and nothing documents turning it off, so the row says nothing. The key
        // page moved to `bigmodel.cn/usercenter/proj-mgmt/apikeys`.
        //
        // TODO(register): Zhipu runs a server-side `web_search` tool in one
        // round trip, which is the shape this row could carry — but its
        // `search_engine` field is required and the two houses of their docs
        // name disjoint values for it: the mainland reference lists `search_std`,
        // `search_pro`, `search_pro_sogou` and `search_pro_quark`, while z.ai
        // documents `search_pro_jina` as the only supported one. A wrong engine
        // id is a 400 on every searching turn, and this row's `base_url` is the
        // mainland host while a user may edit it to z.ai — so settle it against
        // a real key on whichever host before adding a `Search::Zhipu` arm
        // (re-checked 2026-08-25).
        row(
            "zhipu",
            "Zhipu (GLM)",
            "https://open.bigmodel.cn/api/paas/v4",
            "https://bigmodel.cn/usercenter/proj-mgmt/apikeys",
        ),
        // Google's OpenAI compatibility layer, which they document as a
        // first-class path and which terminates at Google. `None` is not a
        // placeholder: the layer does accept `reasoning_effort`, `none`
        // included, but the docs say reasoning cannot be turned off for Gemini
        // 2.5 Pro or any Gemini 3 model — and the lineup is 3.x — so there is no
        // off-switch to express for anything a user would pick. If Google ever
        // documents `none` for a shipping tier, that is one family added to
        // `EFFORT_NONE_FAMILIES`, not a new arm.
        //
        // `search` is `None` for the same kind of reason: Grounding with Google
        // Search is reachable through this layer, but their compatibility page
        // documents it under image generation rather than chat completions, and
        // a parameter the layer does not list is silently ignored — so a switch
        // here would claim a search that may never run (checked 2026-08-25).
        row(
            "gemini",
            "Google Gemini",
            "https://generativelanguage.googleapis.com/v1beta/openai/",
            "https://aistudio.google.com/apikey",
        ),
        Provider {
            // Qwen3 through Alibaba's compatible mode: the one hosted endpoint
            // in this list that takes the `chat_template_kwargs` form.
            // `dashscope-intl.aliyuncs.com` is the international host, again an
            // edit to `base_url` rather than a row of its own.
            reasoning: Reasoning::Qwen,
            // And the one hosted endpoint whose web search is a single boolean
            // on the same body — a top-level field on the wire, whatever the
            // Python SDK's `extra_body` makes it look like. Documented here for
            // the Qwen3.5-and-later Plus and Flash tiers and `qwen-plus`; the
            // Max tiers search through their Responses API, which Beckon does
            // not post to, so the field reaches those as nothing rather than as
            // a search (checked 2026-08-25).
            search: Search::Dashscope,
            ..row(
                "dashscope",
                "Alibaba DashScope",
                "https://dashscope.aliyuncs.com/compatible-mode/v1",
                "https://bailian.console.aliyun.com",
            )
        },
        // Anthropic's OpenAI compatibility layer. `reasoning` is `None` on their
        // own word: the compatibility table lists `reasoning_effort` as
        // *Ignored*, so there is no off-switch to express and sending one would
        // claim something untrue. `search` is `None` for the neighbouring
        // reason: their web search is a server tool of the Messages API, and
        // this layer carries no field for it (checked 2026-08-25).
        //
        // The row that used to be impossible. Its `/v1/models` is native, not
        // compatible, and reads an `Authorization: Bearer` as an OAuth token —
        // so the list came back `401` and no model could ever be chosen. Fixed
        // in `llm/client::signed`, which now sends `x-api-key` beside the
        // bearer on every keyed request; that is a header, so the endpoints
        // that do not know it ignore it (probed 2026-08-25).
        row(
            "anthropic",
            "Anthropic (Claude)",
            "https://api.anthropic.com/v1",
            "https://platform.claude.com/settings/keys",
        ),
        Provider {
            // No `search` arm: MiniMax's chat completions take user-defined
            // function tools and no built-in search of their own, so there is
            // nothing to switch on (checked 2026-08-25).
            //
            // `api.minimaxi.com` is the mainland host — same API, different
            // account, so it is an edit to `base_url` rather than a second row.
            reasoning: Reasoning::Minimax,
            ..row(
                "minimax",
                "MiniMax",
                "https://api.minimax.io/v1",
                "https://platform.minimax.io/console/access",
            )
        },
        Provider {
            // A broker, admitted knowingly (ADR-0025). `Provider::relays` is
            // what makes the row say so; nothing here has to.
            reasoning: Reasoning::Openrouter,
            // The web plugin, which OpenRouter runs itself and folds into the
            // same completion. Their server-tool form is the newer one and lets
            // the model decide when to search; it is a tool the caller declares
            // and is not needed for a switch that means "search this turn"
            // (checked 2026-08-25).
            search: Search::Openrouter,
            ..row(
                "openrouter",
                "OpenRouter",
                "https://openrouter.ai/api/v1",
                "https://openrouter.ai/keys",
            )
        },
        row("ollama", "Ollama (local)", "http://localhost:11434/v1", ""),
        row(
            "lmstudio",
            "LM Studio (local)",
            "http://localhost:1234/v1",
            "",
        ),
        Provider {
            // A Qwen3 chat template is the usual reason to run vLLM by hand,
            // and it is the case a user cannot discover for themselves.
            reasoning: Reasoning::Qwen,
            ..row("vllm", "vLLM (Qwen3)", "http://localhost:8000/v1", "")
        },
    ]
}

/// Whether this id names a row [`presets`] ships.
///
/// Stated once because two unrelated things ask it and must agree: dialect
/// detection skips a preset (its answer came off the vendor's own docs, and
/// detection is the weaker source), and the pane shows the read-only dialect
/// row only where detection can fill it. If the two definitions drifted, a
/// preset would get a value nothing displays or a hand-made row would show a
/// field nothing ever fills.
pub fn is_preset(id: &str) -> bool {
    presets().iter().any(|one| one.id == id)
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
            update_check: true,
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

        // The one field no preset can fill in is the one field a person types,
        // so it is the one that gets typed wrong. Fixed here rather than
        // refused, and here rather than in a pane, so a hand-edited file and a
        // saved form cannot end up meaning different things.
        for provider in &mut self.api.providers {
            provider.base_url = normalise_base_url(&provider.base_url);
        }
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
        // Container-level `serde(default)` fills a missing field from
        // `Config::default()` rather than from the field type's default — which
        // is the only reason a bool that has to arrive `true` can be one.
        assert!(parsed.update_check);
        assert_eq!(parsed.launcher_hotkey, DEFAULT_LAUNCHER_HOTKEY);
        assert_eq!(parsed.defaults.provider, DEFAULT_PROVIDER_ID);
        assert_eq!(parsed.api.providers, vec![Provider::deepseek()]);
    }

    /// A file that declines the automatic check is honoured, and nothing else
    /// about it moves (ADR-0022).
    #[test]
    fn the_update_check_can_be_declined() {
        let parsed = loaded("update_check = false");
        assert!(!parsed.update_check);
        assert!(parsed.autostart);
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

    /// Both host rules read the same thing: the authority, lowercased, with no
    /// scheme and no path — so a URL that merely *mentions* a host is not that
    /// host.
    #[test]
    fn host_rules_read_the_authority_only() {
        let row = |base_url: &str| Provider {
            base_url: base_url.into(),
            ..Provider::default()
        };
        assert!(row("HTTP://LocalHost:8000/v1").is_local());
        assert!(row("http://172.20.0.5:11434/v1").is_local());
        assert!(!row("https://api.deepseek.com/v1").is_local());
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

    /// Every preset ships with searching off (ADR-0026). A row whose arm can
    /// reach the wire is exactly a row that would start billing per request the
    /// moment a user chose it, so the arm is the capability and this is the
    /// consent.
    #[test]
    fn no_preset_searches_until_it_is_asked_to() {
        for row in presets() {
            assert!(!row.web_search, "{} ships searching", row.id);
        }
    }

    /// The model gate is the vendor's word or silence, never a guess (ADR-0027).
    #[test]
    fn a_model_the_arm_has_not_heard_of_is_not_ruled_out() {
        // The whole endpoint reads the field, so every model behind it does.
        assert_eq!(Search::Xai.supports_model("grok-4.5"), Some(true));
        assert_eq!(
            Search::Openrouter.supports_model("anything/at-all"),
            Some(true)
        );

        // The one documented split: Plus and Flash take the field, Max does not.
        assert_eq!(Search::Dashscope.supports_model("qwen-plus"), Some(true));
        assert_eq!(
            Search::Dashscope.supports_model("qwen3.5-flash"),
            Some(true)
        );
        assert_eq!(
            Search::Dashscope.supports_model("qwen-flash-character"),
            Some(true)
        );
        assert_eq!(Search::Dashscope.supports_model("qwen3.7-max"), Some(false));
        // Something else behind the same host: not documented either way, so
        // the switch stays offered rather than greyed on a guess.
        assert_eq!(Search::Dashscope.supports_model("deepseek-r1"), None);
        assert_eq!(Search::Dashscope.supports_model(""), None);
        // The Max tier is Alibaba's, so the exclusion is too: another vendor's
        // model that happens to carry the word is not something they ruled out.
        assert_eq!(Search::Dashscope.supports_model("minimax-m2"), None);

        // Not a fact about the model — the row already says the endpoint has
        // no field at all.
        assert_eq!(
            Search::None.supports_model("deepseek-v4-flash"),
            Some(false)
        );
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
            web_search: true,
            search: Search::Dashscope,
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

    /// The product decision, as a test: a preset that relays is *disclosed*,
    /// and every hosted one is reached over TLS (ADR-0021, ADR-0025).
    ///
    /// This used to assert that no preset named a broker at all. It now asserts
    /// the weaker, still load-bearing thing — that a broker in the list is one
    /// [`Provider::relays`] recognises — because the disclosure is what makes
    /// admitting one acceptable, and a row that relayed *silently* is the
    /// failure the old ban was really guarding against.
    ///
    /// An inference provider serving somebody else's open weights on its own
    /// GPUs is not a broker and needs no disclosure: a hosted vLLM is the shape,
    /// and Groq and SiliconFlow were both in this list until they were dropped
    /// on scope rather than on this rule.
    #[test]
    fn every_relaying_preset_says_so() {
        let presets = presets();
        assert!(presets.len() > 1);
        for row in &presets {
            assert!(!row.id.trim().is_empty(), "{}", row.label);
            assert!(!row.label.trim().is_empty(), "{}", row.id);
            let host = row.base_url.to_ascii_lowercase();
            for name in BROKERS {
                // Naming a broker is allowed; naming one the row does not admit
                // to is not.
                assert!(
                    !host.contains(name) || row.relays() == Some(name),
                    "{} reaches {name} without disclosing it",
                    row.id
                );
            }
            if row.is_local() {
                // A local server is plain HTTP and needs no key page.
                assert!(row.key_page.is_none(), "{}", row.id);
                assert!(row.relays().is_none(), "{} is local and relays", row.id);
            } else {
                assert!(host.starts_with("https://"), "{} is not TLS", row.id);
                assert!(row.key_page.is_some(), "{} has no key page", row.id);
            }
        }
        // The one relaying row we ship, named rather than merely permitted: if
        // it is ever dropped this assertion is the reminder to drop the
        // disclosure machinery with it.
        let openrouter = presets.iter().find(|one| one.id == "openrouter");
        assert_eq!(openrouter.and_then(Provider::relays), Some("openrouter"));
        // Ids are the credential account, so a duplicate would share a key.
        let mut ids: Vec<&str> = presets.iter().map(|one| one.id.as_str()).collect();
        ids.sort_unstable();
        let count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), count, "duplicate preset id");
    }

    /// The three ways a hand-typed URL goes wrong, each fixed rather than
    /// reported. Every case here is one a user would otherwise debug from a
    /// `404`, a TLS error, or a `401` that names none of them.
    #[test]
    fn a_hand_typed_base_url_is_fixed_rather_than_refused() {
        for (typed, expected) in [
            // A scheme nobody types.
            ("api.example.com/v1", "https://api.example.com/v1"),
            // …except for a local server, where https is the failure.
            ("localhost:11434/v1", "http://localhost:11434/v1"),
            ("127.0.0.1:8000", "http://127.0.0.1:8000"),
            // The URL as it appears in a vendor's quickstart. Without this the
            // request goes to `/chat/completions/chat/completions`.
            (
                "https://api.example.com/v1/chat/completions",
                "https://api.example.com/v1",
            ),
            (
                "https://api.example.com/v1/completions",
                "https://api.example.com/v1",
            ),
            // Trailing slashes and stray whitespace.
            (
                "  https://api.example.com/v1/  ",
                "https://api.example.com/v1",
            ),
            // Already right: normalising is not rewriting.
            ("https://api.deepseek.com", "https://api.deepseek.com"),
            // A scheme is case-insensitive. Read case-sensitively this is a
            // scheme-less URL, gets a second scheme glued on, and `fold_legacy`
            // writes the result back — a row that can no longer reach anything.
            ("HTTP://localhost:1234/v1", "http://localhost:1234/v1"),
            ("HTTPS://Api.Example.com/v1", "https://Api.Example.com/v1"),
            // `models` is deliberately left alone — it could be a compat root.
            (
                "https://gateway.example.com/models",
                "https://gateway.example.com/models",
            ),
            // A blank row is a blank row; a scheme would make it look set up.
            ("", ""),
            ("   ", ""),
        ] {
            assert_eq!(normalise_base_url(typed), expected, "typed {typed:?}");
        }
    }

    /// The fix has to reach a file the user edited by hand, not just a value a
    /// pane sent — which is the whole reason it lives in `fold_legacy`.
    #[test]
    fn folding_normalises_every_row_url() {
        let mut config = Config::default();
        config.api.providers = vec![Provider {
            base_url: "api.example.com/v1/chat/completions".into(),
            ..Provider::deepseek()
        }];
        config.fold_legacy();
        assert_eq!(
            config.api.providers[0].base_url,
            "https://api.example.com/v1"
        );
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
update_check = true
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
