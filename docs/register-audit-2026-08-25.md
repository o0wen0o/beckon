# Model register audit — 2026-08-25

One line per row, per `.claude/skills/model-register/audit.md`. A row with no line is unchecked, not
fine. Verdicts are the ones that file names: **Current**, **Moved**, **Live but superseded**, **Alias
that has drifted**, **Gone**, **Unverified**.

This is the first audit written to `docs/`. The six code comments and ADR-0024 that cited
`docs/register-audit-2026-08-24.md` cited a file that was never committed — that pass happened and
its conclusions are live in the code, but its per-row table does not exist. Those citations now point
here.

## What was checked

- Every `key_page` and every `{base_url}/models` fetched anonymously; status codes below are
  observed, not inferred. Key pages were re-fetched with a browser `User-Agent` where a bare `curl`
  drew a bot block, and both results are recorded.
- Each row's `reasoning` and `search` arm re-read against the vendor's current docs.
- `Search::supports_model`, `EFFORT_NONE_FAMILIES`, `ALWAYS_THINKING_MINIMAX` and `CATALOG` re-read
  against the same pages.
- **No authenticated listing check.** No API key for any vendor was available in this environment, so
  the `auth` column of the two-request check in SKILL.md could not be filled for any row. Every
  non-local row's anonymous result is recorded; whether the endpoint answers *with* a key is carried
  forward from the row's own history, not re-verified today.

## URL column (mechanical)

| Row | `key_page` | anon | with browser UA | Verdict |
| --- | --- | --- | --- | --- |
| `deepseek` | `platform.deepseek.com/api_keys` | 403 | 202 | Current — bot block, not drift |
| `openai` | `platform.openai.com/api-keys` | 403 | 200 | Current — bot block, not drift |
| `xai` | `console.x.ai` | 403 | 307 → `console.x.ai/home` (200) | Moved, not carried — see note |
| `moonshot` | `platform.kimi.com/console/api-keys` | 200 | — | Current |
| `zhipu` | `bigmodel.cn/usercenter/proj-mgmt/apikeys` | 200 | — | Current |
| `gemini` | `aistudio.google.com/apikey` | 302 → Google sign-in | — | Current — login wall, `continue` returns to the same path |
| `dashscope` | `bailian.console.aliyun.com` | 200 | — | Current |
| `anthropic` | `platform.claude.com/settings/keys` | 200 | — | Current — and the vendor's own OpenAI-SDK page links this exact URL |
| `minimax` | `platform.minimax.io/console/access` | 200 | — | Current |
| `openrouter` | `openrouter.ai/keys` | 307 → `openrouter.ai/workspaces/default/keys` | 307 | Moved, not carried — see note |
| `ollama`, `lmstudio`, `vllm` | none | — | — | n/a, local rows carry no key page |

**`xai` — Moved, deliberately not carried.** `console.x.ai` redirects to `console.x.ai/home`, which is
the console's own landing page. Carrying `/home` names an app route instead of the console and gains
the user nothing. The actual keys path, `console.x.ai/team/default/api-keys`, is login-gated
(307 → `/login?return_to=…`) and team-scoped, so it is not a stable link to ship either. The bare host
lands in the console for every user; left as written.

**`openrouter` — Moved, deliberately not carried.** `/keys` redirects to
`/workspaces/default/keys`, which then redirects again to sign-in. The destination is
**workspace-scoped**: shipping it would name one user's default workspace in everybody's config.
`/keys` is the stable public alias that resolves correctly per user; left as written. Note the
direction — `/settings/keys` also 307s to the same workspace path, so `/keys` is the alias OpenRouter
still maintains.

## Listing endpoints (anonymous)

`fetch_model_ids` calls `require_api_key` before any HTTP, so a non-local row with no key never asks.
This column records whether the endpoint *would* have answered.

| Row | `{base_url}/models` | anon | Reading |
| --- | --- | --- | --- |
| `deepseek` | `api.deepseek.com/v1/models` | 401 | Answers only with a key; the current gate is correct for it. `api.deepseek.com/models` also 401s, so the `api_url` `/v1` insertion is not the difference |
| `openai` | `api.openai.com/v1/models` | 401 | Only authenticated |
| `xai` | `api.x.ai/v1/models` | 401 | Only authenticated |
| `moonshot` | `api.moonshot.cn/v1/models` | 401 | Only authenticated. `api.moonshot.ai/v1/models` — the international host the row's comment names — also 401s, so both halves of that note still hold |
| `zhipu` | `open.bigmodel.cn/api/paas/v4/models` | 401 | Only authenticated, and on the `v4` path `api_url` builds. This is the row `api_url`'s any-segment version rule exists for; it is still correct |
| `gemini` | `…/v1beta/openai/models` | 404 | **Auth-shaped, not a missing route.** Google answers unauthenticated requests with `{"code": 404, "message": "Requested entity was not found."}`; the same URL with a junk bearer answers `400`. The path reacts to the credential, so it exists. Not verifiable further without a key |
| `dashscope` | `dashscope.aliyuncs.com/compatible-mode/v1/models` | 401 | Only authenticated |
| `anthropic` | `api.anthropic.com/v1/models` | 401 | Only authenticated. The `x-api-key`-beside-the-bearer fix in `llm/client::signed` is what makes this row fillable at all; unchanged today and not re-probed without a key |
| `minimax` | `api.minimax.io/v1/models` | 401 | Only authenticated |
| `openrouter` | `openrouter.ai/api/v1/models` | **200** | **Both answer.** The one row whose dropdown could be filled before a key is stored, if Beckon asked. It does not — `require_api_key` runs first for every non-local row. Recorded, not changed: whether that gate should learn an exception is a product question, not a register one |

## `api_url` version rule

Every `base_url` in `presets()` run against `has_version_segment`, and its `providers.ts` mirror
`chatUrl` read beside it. Both find a `/^v\d/i` segment in `zhipu`'s `/api/paas/v4`, `gemini`'s
`/v1beta/openai/`, `dashscope`'s `/compatible-mode/v1` and `openrouter`'s `/api/v1`, and none in
`api.deepseek.com`. No row builds a wrong URL. The two implementations still read as one rule — the
Rust one returns a URL with its scheme and the TypeScript one without, which is a display difference
in the pane and not a divergence in the rule.

## Per-row verdicts

| Row | `reasoning` | `search` | Verdict |
| --- | --- | --- | --- |
| `deepseek` | `Deepseek` | `None` | **Current.** Pricing page still documents all three live ids as "Supports both non-thinking and thinking (default) modes". No search field on `/chat/completions` |
| `openai` | `OpenAi` | `None` | **Current.** `gpt-5.6` documents `none, low, medium, high, xhigh, max`; `gpt-5.5` documents `none, low, medium (default), high, xhigh` — both still real `none` floors. Chat-completions web search is still the search-specialised models (`gpt-5-search-api`), which "always retrieve from the web" with no field to ask them to; `web_search_options` is a `user_location` config on those models, not an on-switch |
| `xai` | `None` | `Xai` | **Current, and now on the vendor's explicit word.** The reasoning guide states "Reasoning cannot be disabled" — `grok-4.6`/`grok-4.5` take `reasoning_effort` of `low, medium, high, xhigh` with no `none`. `search_parameters` is confirmed on the chat-completions schema with `mode` of `off`, `on` (default), `auto`, exactly as the arm's doc comment describes, including why it sends `auto` rather than `on` |
| `moonshot` | `None` | `None` | **Alias that has drifted — the reasoning arm is now wrong.** See below |
| `zhipu` | `None` | `None` | **Unverified, TODO stands.** See below |
| `gemini` | `None` | `None` | **Current, verbatim on both halves.** The compatibility page still says reasoning "cannot be turned off for Gemini 2.5 Pro or 3 models", and still documents Grounding with Google Search under *image generation* rather than chat completions. Base URL on the page matches the row including its trailing slash |
| `dashscope` | `Qwen` | `Dashscope` | **Current wire, stale `supports_model`.** See below |
| `anthropic` | `None` | `None` | **Current, verbatim.** The compatibility table still lists `reasoning_effort` as `Ignored`, and carries no web-search field of any kind. Thinking is reachable only as the native `thinking` object through `extra_body`, which is the Messages API's shape and not something `Reasoning` has an arm for |
| `minimax` | `Minimax` | `None` | **Current.** MiniMax still documents that for M2.x models thinking cannot be disabled — `thinking: {"type": "disabled"}` is accepted and ignored. Chat completions still carry user-defined function tools and no built-in search |
| `openrouter` | `Openrouter` | `Openrouter` | **Current, verbatim.** The web plugin is still `plugins: [{ "id": "web" }]`, still billed per request, and the docs still name no excluded model — which is what `supports_model`'s `Some(true)` claims |
| `ollama` | `None` | `None` | **Current by construction.** A local row; nothing to fetch and nothing a vendor can move |
| `lmstudio` | `None` | `None` | **Current by construction.** Same |
| `vllm` | `Qwen` | `None` | **Current by construction.** The arm states the case a user running a Qwen3 chat template by hand cannot discover; no vendor page governs it |

## `CATALOG`

| Id | `thinking` | `retired` | Verdict |
| --- | --- | --- | --- |
| `deepseek-v4-flash` | `Switchable` | no | **Current.** Pricing page: "Supports both non-thinking and thinking (default) modes", 1M context |
| `deepseek-v4-pro` | `Switchable` | no | **Current.** Same wording, 1M context. Changelog records GA on 2026-08-13 |
| `deepseek-v4-flash-vision-exp` | `Switchable` | no | **Current.** The 2026-08-24 correction holds — the pricing table documents the same switch as the other two, which is why this is `Switchable` and not the `Never` its launch note's silence once implied. Changelog confirms the 2026-08-21 release date |
| `deepseek-chat` | `Never` | yes | **Current.** Changelog still carries the sunset: discontinued 2026-07-24, pointed at V4-Flash's non-thinking mode during the window |
| `deepseek-reasoner` | `AlwaysOn` | yes | **Current.** Same notice, thinking mode side |

The list endpoint's documented example response still names exactly `deepseek-v4-flash` and
`deepseek-v4-pro`; the vision model comes from the pricing page and the changelog. No id was added,
renamed or withdrawn since the last pass.

## Per-model lists inside a dialect

| List | Polarity | Verdict |
| --- | --- | --- |
| `EFFORT_NONE_FAMILIES` = `["gpt-5.6", "gpt-5.5"]` | allow | **Current.** Both families still document a real `none`. OpenAI ships no `gpt-5.5-mini`, so the prefix still cannot reach a sibling that disagrees. Unmatched stays `Omit`, so the list falling behind stays cheap |
| `ALWAYS_THINKING_MINIMAX` = `["minimax-m2"]` | deny | **Current.** MiniMax's own wording is about **M2.x**, and the prefix covers M2, M2.1, M2.5 and M2.7 as the comment claims. `reasoning_split` is confirmed as a formatting control that does not stop reasoning, which is why it rides in both directions |
| `Search::supports_model`, `Xai` / `Openrouter` → `Some(true)` | — | **Current.** Neither vendor documents an excluded model |
| `Search::supports_model`, `Dashscope` | — | **Stale in the cheap direction.** See below |
| `Search::supports_model`, `None` → `Some(false)` | — | **Current by construction.** Not a claim about a model; the row has no field for any of them |

## The three things this audit changed or opened

### `moonshot` — `Reasoning::None` is no longer true

Kimi's chat API now documents a thinking switch, so the row's `None` claims an absence the vendor has
filled. This is exactly the drift audit.md step 3 is for: nothing fails, no gate can see it, and the
cost is a silent thinking turn on every Action that asked for `thinking = false`.

What the vendor documents today, per model:

- `kimi-k2.6` and `kimi-k2.5` — `thinking: {"type": "enabled" | "disabled"}`. A real off-switch, and
  byte-for-byte the shape `Reasoning::Deepseek` already sends.
- `kimi-k2.7-code` — `thinking: {"type": "enabled"}` only; disabling is not supported.
- `kimi-k3` — no `thinking` parameter at all; it always reasons, and takes `reasoning_effort` of
  `low`, `high`, `max` — with no `none`.

So the arm cannot be a plain reuse of `Deepseek`: one host serves three models that disagree, one of
which must be *refused* rather than sent to, the way `ALWAYS_THINKING_MINIMAX` refuses. That makes it
a `Reasoning` arm **with a per-model list**, which SKILL.md places outside an audit's authority —
ADR-0021 named per-model knowledge as the thing putting the dialect on the row got rid of, so each
further list is a narrowing that gets recorded against 0021 or in a new ADR.

Left as `None` and marked `TODO(register):` at the row. Adding the arm is the
[dialect.md](../.claude/skills/model-register/dialect.md) branch and its eight sites, not this pass.

`Search::None` still stands for the row, but for a weaker reason than the comment gives: the current
Kimi chat API page documents no web search at all — not the `$web_search` `builtin_function` the
comment describes. The conclusion is unchanged and the arm is right either way; the *cited mechanism*
is now unverified on the page it was read from. Noted at the site.

### `dashscope` — `supports_model` over-offers two families

`enable_search: true` is confirmed as the field, on the compatible-mode chat endpoint, and the Max
exclusion the code claims is confirmed in Alibaba's own words: the Max tiers take web search through
the Responses API, which Beckon does not post to. `Some(false)` for `qwen*max*` is **correct**.

But the same page marks **Qwen3.6-Plus and Qwen3.6-Flash** as "Supported only by the Responses API"
— and `supports_model` answers `Some(true)` for anything containing `plus` or `flash`. Those two
families get an offered switch whose field reaches nothing.

Deliberately **not** narrowed today, for two reasons:

1. The error direction is the cheap one. Over-offering costs the feature silently (ADR-0026: a model
   ignoring its host's search field costs the feature, not the turn). Narrowing wrongly costs the
   expensive one ADR-0027 was written about — a greyed switch and Settings telling the user, on
   Beckon's word, that their model cannot search.
2. The sources disagree. Alibaba's web-search page carries the Responses-API-only note; third-party
   aggregations list `qwen3.6-plus` and `qwen3.6-flash` as `enable_search` models. Neither is settled
   without a real DashScope key, and SKILL.md ranks the vendor first but audit.md forbids a guess.

Marked `TODO(register):` at `Search::supports_model` with what would settle it.

### `zhipu` — the open search TODO survives, with one new fact

The mainland API reference confirms a server-side `web_search` tool with a **required** `search_engine`
field, and names `search_std`, `search_pro`, `search_pro_sogou`, `search_pro_quark` — with
`search_std` as the default when unspecified. z.ai still documents `search_pro_jina` as its only
supported value. The disagreement the TODO records is unchanged, so the TODO stands.

Two things the re-check settles for whoever picks it up:

- **`search_engine` has a documented default on the mainland host**, so "required" overstates it there.
  That weakens but does not remove the 400 risk, because the *international* half of the split is the
  one whose value set is disjoint.
- **The shape is a `tools` array entry, not a top-level field.** Under ADR-0026's own rule — the named
  arms are exactly the endpoints whose search is one field and one round trip — a `tools` entry is not
  the shape a `Search` arm carries today, whatever its engine id turns out to be. Zhipu runs it
  server-side in one round trip, so the round-trip half is satisfied and the field half is not. That
  is a question for `Search`'s definition, not for the `zhipu` row alone.

## Not verifiable in this environment

- **Every `auth` listing result.** No vendor API key was present. Ten non-local rows have an anonymous
  result recorded above and an authenticated one that was not run.
- **Whether `llm/client::signed`'s `x-api-key`-beside-bearer still fixes `anthropic`'s native
  `/v1/models`.** That fix was probed on 2026-08-25 with a key; today's pass has no key and only
  confirms the endpoint still rejects an anonymous request the same way.
