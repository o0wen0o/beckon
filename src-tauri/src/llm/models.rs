//! Which models Beckon offers, and what each one does with thinking mode.
//!
//! ## One table, one consumer and one enrichment
//!
//! [`CATALOG`] used to *source* the Settings dropdown as well as `thinking_wire`
//! in [`super::request`]. It no longer does. A provider row carries where to
//! fetch and how to connect, never what to run, so what an endpoint serves is
//! the endpoint's own answer and there is no documented list to stand in for it
//! (`docs/register-audit-2026-08-25.md`). What went with that change is
//! `Origin::Documented` and [`options`]'s `documented` parameter.
//!
//! What remains is the half that was never about a dropdown: `Thinking`. A
//! DeepSeek model that always thinks, asked to stop, is a *hard error* in
//! [`super::request`] — omitting the field would silently leave thinking on — so
//! this table is where that answer lives, and it is not optional. The rest of a
//! row still *enriches* an id the endpoint named: [`describe`] looks every id up
//! here, so a live `deepseek-v4-flash` keeps its label and its description, and
//! [`rank`] keeps ordering a live list by the order this table is written in.
//!
//! ## Where the ids come from
//!
//! DeepSeek's official API reference, checked 2026-08-25:
//!
//! - `GET https://api.deepseek.com/models` documents exactly `deepseek-v4-flash`
//!   and `deepseek-v4-pro` in its example response
//!   (<https://api-docs.deepseek.com/api/list-models>); the pricing page
//!   (<https://api-docs.deepseek.com/quick_start/pricing>) carries a third row
//!   for the vision model, all three 1M context, all three "supports both
//!   non-thinking and thinking (default) modes".
//! - `deepseek-v4-flash-vision-exp` arrived 2026-08-21
//!   (<https://api-docs.deepseek.com/news/news260821/>) and is DeepSeek's
//!   image-reading model. Its announcement said nothing about the `thinking`
//!   object, so it was [`Thinking::Never`] until the 2026-08-24 pass found the
//!   pricing table documenting the switch for it like the other two — the
//!   difference between "the docs are silent" and "the docs say no". This table
//!   records no image column: nothing is gated on one (ADR-0016).
//! - The changelog (<https://api-docs.deepseek.com/updates>) records V4-Pro and
//!   V4-Flash arriving 2026-04-24, and the legacy names `deepseek-chat` /
//!   `deepseek-reasoner` being **discontinued on 2026-07-24**. Those two stay in
//!   the table marked [`retired`](CatalogEntry::retired): a config that still
//!   names one has to keep working and be explained, but neither is offered as
//!   a fresh choice.
//!
//! ## The live list, and what happens without one
//!
//! [`options`] offers the ids the endpoint actually serves, because there is one
//! `base_url` per provider row — pointed at a local or non-DeepSeek endpoint,
//! this table describes nothing that exists there.
//!
//! Without a live list there is no fallback, and the dropdown is **empty on
//! purpose**. That used to be an edge case a shipped `model` field papered over;
//! it is now the initial state of every row a user adds, so it is a state the
//! pane renders rather than an accident it hides. Offering `deepseek-v4-flash`
//! for somebody's Ollama was the alternative, and a dropdown of ids that
//! endpoint has never served is worse than saying there are none yet.

use serde::Serialize;

use crate::config::{Language, Search};

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
        description: "DeepSeek's image-reading model, 1M context. Thinking can be switched off.",
        description_zh: "DeepSeek 的图片阅读模型，100 万上下文。思考模式可关闭。",
        // Was `Never` while the launch note was silent about the `thinking`
        // object; the pricing table documents the same switch as the other two
        // rows, so it takes the object both directions (re-checked 2026-08-25).
        thinking: Thinking::Switchable,
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
///
/// There is no `Documented` arm. [`CATALOG`] stopped sourcing this list when a
/// provider row stopped carrying a catalog — see the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
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
    /// Whether this endpoint's search field reaches this model (ADR-0027), from
    /// [`Search::supports_model`] — so the pane can grey a switch the vendor
    /// says would do nothing. `None` is "not documented either way", and the
    /// switch stays offered on it.
    pub search: Option<bool>,
    pub origin: Origin,
}

/// Build the dropdown.
///
/// `live` is the endpoint's own list when we managed to fetch one — or the last
/// one it gave us, from [`crate::models_cache`] — and `None` otherwise.
/// `selected` is every model id the configuration currently names for this
/// provider — the row's own model plus each Action's override — so that a value
/// we cannot vouch for still appears rather than being rewritten out from under
/// the user.
///
/// **This may return an empty list, and that is a state rather than a fault.** A
/// row whose endpoint has never answered and whose `model` is empty has nothing
/// to offer, and inventing something to put there is what the documented
/// fallback used to do. The pane renders the emptiness; `llm/request.rs` refuses
/// the turn.
pub fn options(
    live: Option<&[String]>,
    selected: &[String],
    language: Language,
    search: Search,
) -> Vec<ModelOption> {
    let mut out: Vec<ModelOption> = Vec::new();

    // The endpoint told us what it serves; that is the offer, whatever
    // `base_url` happens to point at.
    if let Some(ids) = live {
        for id in ids {
            push_unique(&mut out, id, Origin::Live, language, search);
        }
        // The provider's order is arbitrary; catalog order is meaningful. A
        // stable sort keeps the provider's order among the ids we do not
        // recognise, which trail the ones we do.
        out.sort_by_key(|option| rank(&option.id));
    }

    for id in selected {
        push_unique(&mut out, id, Origin::Configured, language, search);
    }

    out
}

fn push_unique(
    out: &mut Vec<ModelOption>,
    id: &str,
    origin: Origin,
    language: Language,
    search: Search,
) {
    let id = id.trim();
    if id.is_empty() || out.iter().any(|option| option.id.eq_ignore_ascii_case(id)) {
        return;
    }
    out.push(describe(id, origin, language, search));
}

fn describe(id: &str, origin: Origin, language: Language, search: Search) -> ModelOption {
    let id = id.trim();
    // The endpoint's arm answers this, not the catalog: which models take the
    // search field is a fact about the host, and the catalog is DeepSeek's
    // (ADR-0027).
    let searches = search.supports_model(id);
    match find(id) {
        Some(entry) => ModelOption {
            id: entry.id.to_string(),
            label: entry.label.to_string(),
            description: entry.description(language).to_string(),
            thinking: Some(entry.thinking),
            search: searches,
            origin,
        },
        None => ModelOption {
            id: id.to_string(),
            label: id.to_string(),
            description: String::new(),
            thinking: None,
            search: searches,
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

    fn ids(options: &[ModelOption]) -> Vec<&str> {
        options.iter().map(|option| option.id.as_str()).collect()
    }

    fn strings(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| id.to_string()).collect()
    }

    /// The option *set* is what these tests are about, and it is the same in
    /// both languages; `describes_a_model_in_both_languages` covers the rest.
    fn options(live: Option<&[String]>, selected: &[String]) -> Vec<ModelOption> {
        super::options(live, selected, Language::En, Search::None)
    }

    /// Driven through a *live* list, because that is the only thing that sources
    /// the dropdown now — the catalog's job here is to enrich an id the endpoint
    /// named, and the description is the half a person reads.
    #[test]
    fn describes_a_model_in_both_languages() {
        let entry = find(crate::config::DEFAULT_MODEL).unwrap();
        assert_ne!(
            entry.description(Language::En),
            entry.description(Language::Zh)
        );
        let live = strings(&[crate::config::DEFAULT_MODEL]);
        let zh = super::options(Some(&live), &[], Language::Zh, Search::None);
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

    /// With no live list, the configuration is the whole offer — its own model
    /// plus any Action override, and nothing invented alongside them.
    #[test]
    fn an_endpoint_that_has_not_answered_gets_only_what_the_config_names() {
        let offered = options(None, &strings(&["qwen3:8b"]));
        assert_eq!(ids(&offered), vec!["qwen3:8b"]);
        assert_eq!(offered[0].origin, Origin::Configured);
        // Not even for DeepSeek's own ids: the catalog describes a model, it no
        // longer claims an endpoint serves one.
        let offered = options(None, &[]);
        assert!(offered.is_empty());
        // A live list is the endpoint's own word and needs no vouching either way.
        let live = strings(&["llama3.1:8b"]);
        assert_eq!(ids(&options(Some(&live), &[])), vec!["llama3.1:8b"]);
    }

    /// Was `the_dropdown_is_never_empty`, which claimed an invariant it never
    /// checked — it only ever covered the DeepSeek row and a live list that came
    /// back empty. The invariant is now false *by design*: a row carries no
    /// catalog, so a fresh remote row with no credential has nothing to offer,
    /// and the pane says so rather than a select rendering a blank box.
    #[test]
    fn an_endpoint_with_no_list_and_no_configured_model_offers_nothing() {
        assert_eq!(options(None, &[]), Vec::new());
        // An endpoint that serves nothing at all is the one case where the
        // configured value is all there is — and it is still offered.
        let empty: Vec<String> = Vec::new();
        assert_eq!(
            ids(&options(Some(&empty), &strings(&["my-model"]))),
            vec!["my-model"]
        );
    }

    #[test]
    fn a_live_list_is_the_offer() {
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
        assert_eq!(ids(&options), vec!["deepseek-v9-quantum"]);
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

    /// The endpoint's word wins the entry, so a row that names the same id twice
    /// over does not get it twice — and the origin stays the endpoint's.
    #[test]
    fn a_configured_model_that_is_already_offered_is_not_duplicated() {
        let live = strings(&["deepseek-v4-flash", "deepseek-v4-pro"]);
        let options = options(
            Some(&live),
            &strings(&["DeepSeek-V4-Flash", "deepseek-v4-pro"]),
        );
        assert_eq!(ids(&options), vec!["deepseek-v4-flash", "deepseek-v4-pro"]);
        assert!(options.iter().all(|o| o.origin == Origin::Live));
    }

    #[test]
    fn blank_configured_values_are_not_options() {
        // An Action with no override contributes nothing — and with nothing else
        // to offer, nothing is the whole answer.
        assert_eq!(options(None, &strings(&["", "   "])), Vec::new());
    }

    #[test]
    fn duplicate_ids_from_the_endpoint_collapse() {
        let live = strings(&["deepseek-v4-flash", "deepseek-v4-flash"]);
        assert_eq!(ids(&options(Some(&live), &[])), vec!["deepseek-v4-flash"]);
    }

    /// The dropdown carries the endpoint's answer about each model, so the pane
    /// can grey a switch rather than offer one the vendor says does nothing
    /// (ADR-0027). The arm answers, the catalog does not: these ids are not in
    /// it at all.
    #[test]
    fn each_option_says_whether_this_endpoint_can_search_with_it() {
        let live = strings(&["qwen3.7-max", "qwen3.5-plus", "some-preview"]);
        let offered = super::options(Some(&live), &[], Language::En, Search::Dashscope);
        let by_id = |id: &str| offered.iter().find(|o| o.id == id).unwrap().search;
        assert_eq!(by_id("qwen3.5-plus"), Some(true));
        assert_eq!(by_id("qwen3.7-max"), Some(false));
        // Not documented either way: offered, not ruled out.
        assert_eq!(by_id("some-preview"), None);
        // An endpoint with no field at all answers for every model it serves.
        let offered = super::options(Some(&live), &[], Language::En, Search::None);
        assert!(offered.iter().all(|o| o.search == Some(false)));
    }

    #[test]
    fn the_suggested_model_is_one_we_would_actually_offer() {
        let suggestion = switchable_suggestion();
        let entry = find(suggestion).unwrap();
        assert!(!entry.retired);
        assert_eq!(entry.thinking, Thinking::Switchable);
    }
}
