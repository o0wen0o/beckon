//! Which models Beckon offers, and what each one does with thinking mode.
//!
//! ## One table, two consumers
//!
//! [`CATALOG`] is the single source of truth for both the Settings dropdown and
//! `thinking_wire` in [`super::request`]. That is deliberate. A DeepSeek model
//! that always thinks, asked to stop, is a *hard error* there — omitting the
//! field would silently leave thinking on — so a dropdown built from a second,
//! hand-kept list would be a dropdown that offers models the request layer then
//! refuses. Both read this table, so the two cannot drift.
//!
//! ## Where the ids come from
//!
//! DeepSeek's official API reference, checked 2026-08-01:
//!
//! - `GET https://api.deepseek.com/models` documents exactly `deepseek-v4-flash`
//!   and `deepseek-v4-pro` in its example response
//!   (<https://api-docs.deepseek.com/api/list-models>); the pricing page lists
//!   the same two, both 1M context, thinking on by default.
//! - `deepseek-v4-flash-vision-exp` arrived 2026-08-21
//!   (<https://api-docs.deepseek.com/news/news260821/>) and is DeepSeek's
//!   image-reading model. It is experimental, and the docs say nothing about
//!   whether it takes the `thinking` object — hence [`Thinking::Never`] for it,
//!   which refuses `thinking = true` out loud instead of guessing. This table
//!   records no image column: nothing is gated on one (ADR-0016).
//! - The changelog (<https://api-docs.deepseek.com/updates>) records V4-Pro and
//!   V4-Flash arriving 2026-04-24, and the legacy names `deepseek-chat` /
//!   `deepseek-reasoner` being **discontinued on 2026-07-24**. Those two stay in
//!   the table marked [`retired`](CatalogEntry::retired): a config that still
//!   names one has to keep working and be explained, but neither is offered as
//!   a fresh choice.
//!
//! ## The live list versus this one
//!
//! [`options`] prefers the ids the endpoint actually serves, because there is
//! one `base_url` per provider row — pointed at a local or non-DeepSeek
//! endpoint, this table describes nothing that exists there. The documented list
//! is what we fall back to when there is no credential, the fetch fails, or the
//! machine is offline. It is a *fallback*, never an empty dropdown: two ids that
//! are almost certainly right beat nothing to pick.
//!
//! Since ADR-0021 that fallback is offered **only for DeepSeek's own host**
//! ([`Provider::is_deepseek_host`](crate::config::Provider::is_deepseek_host)).
//! Offering `deepseek-v4-flash` as the documented list for somebody's Ollama
//! would be a dropdown of ids that endpoint has never served — worse than the
//! one entry the row already names.

use serde::Serialize;

use crate::config::Language;

/// What a model does with thinking mode — the property [`super::request`] needs
/// in order to put the right `thinking` object on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Thinking {
    /// Takes the documented `thinking: {"type": ...}` object, both directions.
    Switchable,
    /// Always thinks; `thinking = false` cannot be honoured.
    AlwaysOn,
    /// Never thinks; `thinking = true` cannot be honoured.
    Never,
}

/// One officially documented model.
///
/// The description is the one line in this table a person reads, so it is here
/// in both languages rather than in `src/lib/i18n/`: keying a translation off a
/// model id in the frontend would put half of one row in each half of the app,
/// and the live list can name models this table has never heard of.
#[derive(Debug, Clone, Copy)]
pub struct CatalogEntry {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub description_zh: &'static str,
    pub thinking: Thinking,
    /// Withdrawn by the provider. Still recognised — an existing config must not
    /// break — but never offered as a new choice.
    pub retired: bool,
}

pub const CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        id: "deepseek-v4-flash",
        label: "DeepSeek V4 Flash",
        description: "Fast and cheap, 1M context. Thinking can be switched off.",
        description_zh: "快而便宜，100 万上下文。思考模式可关闭。",
        thinking: Thinking::Switchable,
        retired: false,
    },
    CatalogEntry {
        id: "deepseek-v4-pro",
        label: "DeepSeek V4 Pro",
        description: "The stronger V4, 1M context. Thinking can be switched off.",
        description_zh: "更强的 V4，100 万上下文。思考模式可关闭。",
        thinking: Thinking::Switchable,
        retired: false,
    },
    CatalogEntry {
        id: "deepseek-v4-flash-vision-exp",
        label: "DeepSeek V4 Flash Vision (experimental)",
        description: "DeepSeek's image-reading model. Experimental; no thinking mode.",
        description_zh: "DeepSeek 的图片阅读模型。实验性质，没有思考模式。",
        // Undocumented for this model, and a `thinking` object it does not
        // understand is a 400 on every request — so `true` is refused rather
        // than sent hopefully.
        thinking: Thinking::Never,
        retired: false,
    },
    CatalogEntry {
        id: "deepseek-chat",
        label: "DeepSeek Chat (retired)",
        description: "Legacy name for V4-Flash without thinking; withdrawn 2026-07-24.",
        description_zh: "V4-Flash 不带思考的旧名称；已于 2026-07-24 下线。",
        thinking: Thinking::Never,
        retired: true,
    },
    CatalogEntry {
        id: "deepseek-reasoner",
        label: "DeepSeek Reasoner (retired)",
        description: "Legacy name for V4-Flash with thinking; withdrawn 2026-07-24.",
        description_zh: "V4-Flash 带思考的旧名称；已于 2026-07-24 下线。",
        thinking: Thinking::AlwaysOn,
        retired: true,
    },
];

impl CatalogEntry {
    /// The description in the reader's language.
    pub fn description(&self, language: Language) -> &'static str {
        match language {
            Language::En => self.description,
            Language::Zh => self.description_zh,
        }
    }
}

/// Model ids are matched case-insensitively, the way the API treats them.
pub fn find(id: &str) -> Option<&'static CatalogEntry> {
    let id = id.trim();
    CATALOG
        .iter()
        .find(|entry| entry.id.eq_ignore_ascii_case(id))
}

/// The id suggested in an error message when the user's model cannot do what
/// they asked of it. Derived, so retiring a model cannot leave stale advice
/// behind in [`super::request`].
pub fn switchable_suggestion() -> &'static str {
    CATALOG
        .iter()
        .find(|entry| !entry.retired && entry.thinking == Thinking::Switchable)
        .map(|entry| entry.id)
        // The catalog is a constant, so this is unreachable in practice; a
        // literal beats a panic in a message-formatting path.
        .unwrap_or("deepseek-v4-flash")
}

/// Where one dropdown entry came from. The UI needs the distinction:
/// `Configured` is the user's own value that nothing else vouches for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    /// From [`CATALOG`] — the officially documented list.
    Documented,
    /// The endpoint says it serves this.
    Live,
    /// Neither: it is only in the user's config. Kept so the value is never
    /// silently dropped or rewritten.
    Configured,
}

/// One option in the Settings dropdown.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModelOption {
    pub id: String,
    pub label: String,
    /// Empty when we know nothing about the model beyond its id.
    pub description: String,
    /// `None` for a model that is not in the catalog: we do not know, and
    /// guessing is the failure `deepseek` exists to prevent.
    pub thinking: Option<Thinking>,
    pub origin: Origin,
}

/// Build the dropdown.
///
/// `live` is the endpoint's own list when we managed to fetch one, `None`
/// otherwise. `documented` says whether [`CATALOG`] describes *this* endpoint —
/// it is DeepSeek's own list, so it is a fallback only for DeepSeek's own host
/// (ADR-0021). `selected` is every model id the configuration currently names
/// for this provider — the row's own model plus each Action's override — so that
/// a value we cannot vouch for still appears rather than being rewritten out
/// from under the user.
pub fn options(
    live: Option<&[String]>,
    documented: bool,
    selected: &[String],
    language: Language,
) -> Vec<ModelOption> {
    let mut out: Vec<ModelOption> = Vec::new();

    match live {
        // The endpoint told us what it serves; that is the offer, whatever
        // `base_url` happens to point at.
        Some(ids) => {
            for id in ids {
                push_unique(&mut out, id, Origin::Live, language);
            }
            // The provider's order is arbitrary; catalog order is meaningful.
            // A stable sort keeps the provider's order among the ids we do not
            // recognise, which trail the ones we do.
            out.sort_by_key(|option| rank(&option.id));
        }
        // No live list. The documented one stands in where it describes the
        // endpoint, and nowhere else — leaving only `selected`, which is at
        // minimum the model the row already names.
        None if documented => {
            for entry in CATALOG.iter().filter(|entry| !entry.retired) {
                out.push(describe(entry.id, Origin::Documented, language));
            }
        }
        None => {}
    }

    for id in selected {
        push_unique(&mut out, id, Origin::Configured, language);
    }

    out
}

fn push_unique(out: &mut Vec<ModelOption>, id: &str, origin: Origin, language: Language) {
    let id = id.trim();
    if id.is_empty() || out.iter().any(|option| option.id.eq_ignore_ascii_case(id)) {
        return;
    }
    out.push(describe(id, origin, language));
}

fn describe(id: &str, origin: Origin, language: Language) -> ModelOption {
    let id = id.trim();
    match find(id) {
        Some(entry) => ModelOption {
            id: entry.id.to_string(),
            label: entry.label.to_string(),
            description: entry.description(language).to_string(),
            thinking: Some(entry.thinking),
            origin,
        },
        None => ModelOption {
            id: id.to_string(),
            label: id.to_string(),
            description: String::new(),
            thinking: None,
            origin,
        },
    }
}

fn rank(id: &str) -> usize {
    CATALOG
        .iter()
        .position(|entry| entry.id.eq_ignore_ascii_case(id))
        .unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What the dropdown offers with no live list: every non-retired row of the
    /// catalog, in catalog order. Named once, because four tests are about what
    /// happens *around* it rather than about the list itself.
    const DOCUMENTED: [&str; 3] = [
        "deepseek-v4-flash",
        "deepseek-v4-pro",
        "deepseek-v4-flash-vision-exp",
    ];

    fn ids(options: &[ModelOption]) -> Vec<&str> {
        options.iter().map(|option| option.id.as_str()).collect()
    }

    fn strings(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    /// The option *set* is what these tests are about, and it is the same in
    /// both languages; `describes_a_model_in_both_languages` covers the rest.
    /// `documented` is `true` here — these tests are about the DeepSeek row, and
    /// `an_endpoint_the_catalog_does_not_describe_gets_no_documented_list`
    /// covers the other side.
    fn options(live: Option<&[String]>, selected: &[String]) -> Vec<ModelOption> {
        super::options(live, true, selected, Language::En)
    }

    #[test]
    fn describes_a_model_in_both_languages() {
        let entry = find(crate::config::DEFAULT_MODEL).unwrap();
        assert_ne!(
            entry.description(Language::En),
            entry.description(Language::Zh)
        );
        let zh = super::options(None, true, &[], Language::Zh);
        let flash = zh
            .iter()
            .find(|o| o.id == crate::config::DEFAULT_MODEL)
            .unwrap();
        assert_eq!(flash.description, entry.description_zh);
    }

    /// Every documented model carries both descriptions: a row added with only
    /// the English one would read as translated and would not be.
    #[test]
    fn the_catalog_is_translated_throughout() {
        for entry in CATALOG {
            assert!(!entry.description_zh.is_empty(), "{}", entry.id);
            assert_ne!(entry.description, entry.description_zh, "{}", entry.id);
        }
    }

    #[test]
    fn the_default_model_is_a_live_catalog_entry() {
        let entry = find(crate::config::DEFAULT_MODEL).expect("default model is in the catalog");
        assert!(!entry.retired, "the default must not be a retired model");
    }

    #[test]
    fn lookup_is_case_insensitive_and_tolerates_padding() {
        assert_eq!(find(" DeepSeek-V4-Flash ").unwrap().id, "deepseek-v4-flash");
        assert!(find("gpt-4o-mini").is_none());
    }

    #[test]
    fn without_a_live_list_the_documented_models_are_offered() {
        let options = options(None, &[]);
        assert_eq!(ids(&options), DOCUMENTED.to_vec());
        assert!(options.iter().all(|o| o.origin == Origin::Documented));
        assert_eq!(options[0].thinking, Some(Thinking::Switchable));
    }

    #[test]
    fn retired_models_are_never_offered_on_their_own() {
        let options = options(None, &[]);
        let offered = ids(&options);
        assert!(!offered.contains(&"deepseek-chat"));
        assert!(!offered.contains(&"deepseek-reasoner"));
    }

    /// The list is DeepSeek's own, so it stands in only for DeepSeek's own host
    /// (ADR-0021). Everywhere else the row's own model is the whole offer —
    /// which is still not an empty dropdown.
    #[test]
    fn an_endpoint_the_catalog_does_not_describe_gets_no_documented_list() {
        let offered = super::options(None, false, &strings(&["qwen3:8b"]), Language::En);
        assert_eq!(ids(&offered), vec!["qwen3:8b"]);
        assert_eq!(offered[0].origin, Origin::Configured);
        // A live list is the endpoint's own word and needs no vouching either way.
        let live = strings(&["llama3.1:8b"]);
        let offered = super::options(Some(&live), false, &[], Language::En);
        assert_eq!(ids(&offered), vec!["llama3.1:8b"]);
    }

    #[test]
    fn the_dropdown_is_never_empty() {
        // Offline, no credential, nothing configured: still something to pick.
        assert!(!options(None, &[]).is_empty());
        // An endpoint that serves nothing at all is the one case where the
        // configured value is all there is — and it is still offered.
        let empty: Vec<String> = Vec::new();
        assert_eq!(
            ids(&options(Some(&empty), &strings(&["my-model"]))),
            vec!["my-model"]
        );
    }

    #[test]
    fn a_live_list_replaces_the_documented_one() {
        // A non-DeepSeek endpoint must not be described with DeepSeek ids.
        let live = strings(&["llama3.1:8b", "qwen2.5"]);
        let options = options(Some(&live), &[]);
        assert_eq!(ids(&options), vec!["llama3.1:8b", "qwen2.5"]);
        assert!(options.iter().all(|o| o.origin == Origin::Live));
        // Nothing is claimed about a model we have never heard of.
        assert_eq!(options[0].thinking, None);
        assert_eq!(options[0].description, "");
    }

    #[test]
    fn known_live_models_keep_their_catalog_metadata_and_order() {
        let live = strings(&["deepseek-v4-pro", "some-preview", "deepseek-v4-flash"]);
        let options = options(Some(&live), &[]);
        // Catalog order first, provider order preserved among the rest.
        assert_eq!(
            ids(&options),
            vec!["deepseek-v4-flash", "deepseek-v4-pro", "some-preview"]
        );
        assert_eq!(options[0].label, "DeepSeek V4 Flash");
        assert_eq!(options[0].thinking, Some(Thinking::Switchable));
        assert_eq!(options[0].origin, Origin::Live);
    }

    #[test]
    fn a_live_list_may_resurrect_a_retired_model() {
        // If the endpoint says it serves it, it is pickable — the retirement
        // date is a fact about api.deepseek.com, not about every base_url.
        let live = strings(&["deepseek-chat"]);
        let options = options(Some(&live), &[]);
        assert_eq!(ids(&options), vec!["deepseek-chat"]);
        assert_eq!(options[0].thinking, Some(Thinking::Never));
    }

    #[test]
    fn a_configured_model_nobody_vouches_for_is_still_offered() {
        let options = options(None, &strings(&["deepseek-v9-quantum"]));
        assert_eq!(
            ids(&options),
            [DOCUMENTED.as_slice(), &["deepseek-v9-quantum"]].concat()
        );
        let extra = options.last().unwrap();
        assert_eq!(extra.origin, Origin::Configured);
        assert_eq!(extra.thinking, None);
    }

    #[test]
    fn a_configured_retired_model_is_offered_and_explained() {
        let options = options(None, &strings(&["deepseek-chat"]));
        let extra = options.last().unwrap();
        assert_eq!(extra.id, "deepseek-chat");
        assert_eq!(extra.origin, Origin::Configured);
        // Retired, but we still know what it does with thinking, so the
        // request layer and the UI agree about it.
        assert_eq!(extra.thinking, Some(Thinking::Never));
        assert!(extra.description.contains("2026-07-24"));
    }

    #[test]
    fn a_configured_model_that_is_already_offered_is_not_duplicated() {
        let options = options(None, &strings(&["DeepSeek-V4-Flash", "deepseek-v4-pro"]));
        assert_eq!(ids(&options), DOCUMENTED.to_vec());
        assert!(options.iter().all(|o| o.origin == Origin::Documented));
    }

    #[test]
    fn blank_configured_values_are_not_options() {
        // An Action with no override contributes nothing.
        let options = options(None, &strings(&["", "   "]));
        assert_eq!(ids(&options), DOCUMENTED.to_vec());
    }

    #[test]
    fn duplicate_ids_from_the_endpoint_collapse() {
        let live = strings(&["deepseek-v4-flash", "deepseek-v4-flash"]);
        assert_eq!(ids(&options(Some(&live), &[])), vec!["deepseek-v4-flash"]);
    }

    #[test]
    fn the_suggested_model_is_one_we_would_actually_offer() {
        let suggestion = switchable_suggestion();
        let entry = find(suggestion).unwrap();
        assert!(!entry.retired);
        assert_eq!(entry.thinking, Thinking::Switchable);
    }
}
