---
status: accepted
---

# Any OpenAI-compatible endpoint, chosen per Action

Beckon spoke to one endpoint. `[api] base_url` named it, `[defaults] model` and `[defaults] thinking`
described what to send it, one credential in the OS store signed every request, and
`llm/deepseek.rs` knew what DeepSeek in particular wants on the wire.

It now keeps a **table** of endpoints, and *which one a turn goes to is a property of the Action*:

```toml
[defaults]
provider = "deepseek"          # what an Action that does not say gets

[[api.providers]]
id = "deepseek"
label = "DeepSeek"
base_url = "https://api.deepseek.com"
model = "deepseek-v4-flash"
thinking = false
reasoning = "deepseek"
temperature = 1.3
key_page = "https://platform.deepseek.com/api_keys"
```

and in an Action:

```toml
[model]
provider = "ollama"            # absent means [defaults] provider
```

DeepSeek is still the default, and a config written before this ADR keeps working untouched.

## No middleman

**A request has to terminate at the company whose key it carries.** No aggregator, no gateway, no
OpenRouter. `every_preset_goes_direct_to_its_own_vendor` in `config.rs` is what keeps that true — a
broker would satisfy the type and nothing else in the codebase would notice.

The line is *termination*, not who trained the weights. Groq, SiliconFlow and a hosted vLLM all serve
somebody else's open models on their own GPUs: the key is theirs, the inference is theirs, nothing is
forwarded, and they belong in the list. A broker is different in kind — it holds keys to *other* APIs
and relays your request to one of them.

This is not a technical constraint; a single proxy is easier to support than eleven hosts. It is what
the product is for. The key in the credential store is issued by one company, the traffic is that
company's to see, and a row in a config file is not the place to quietly insert a third party into
that relationship.

## Why per Action, and not a switch

The first design had `[api] active = "deepseek"` and an Enable button on each row: one endpoint live
at a time, the way every "switch provider" tool works. That is the wrong shape for this product.

An Action is a *prompt with a job*. Translate wants the cheapest fast model. Summarise-this-private-
document wants a model on the machine. Explain-this-code wants the strongest one available. Those are
three jobs with three right answers, and they are one hotkey away from each other — not one settings
trip apart. A global switch makes the common case ("this one thing must never leave the laptop") a
thing you have to remember to do *before* pressing a key, and cannot undo after.

So `provider` joined `model` and `thinking` as an Action override, and the switch became a default.
`ModelOverrides::merge_over` is still the one merge function; it just takes the whole `Config` now,
because overriding the first field moves what the other two inherit.

**What that costs, and how it is paid.** A global switch answered "where does my text go" for free —
one row, highlighted. This design has to earn that answer back, and it does so in three places: every
row on the Connection pane carries its Action count and says `stays on this machine` when it is
local; each Action's editor prints the URL a turn would post to; and the Popover's title bar names the
endpoint beside the model on every turn. That last one is always shown rather than only when it is
surprising — a line that appears only sometimes is a line nobody learns to read.

## Why `reasoning` is a field and not a rule

Every OpenAI-compatible endpoint agrees about `model`, `messages` and `stream`. They disagree about
exactly one thing Beckon needs: **how you say do not think.**

| Endpoint family | To suppress thinking |
| --- | --- |
| DeepSeek | `thinking: {"type": "disabled"}` |
| Qwen3 via vLLM / SGLang / DashScope | `chat_template_kwargs: {"enable_thinking": false}` |
| Everything else | nothing to send |

This cannot be derived from the model id. `deepseek-ai/DeepSeek-V3` served by SiliconFlow speaks the
plain OpenAI dialect; the same weights behind vLLM may take the Qwen form; DeepSeek's own host takes
DeepSeek's. **The dialect belongs to the endpoint**, so the row states it — prefilled by its preset,
which is the one field a person cannot look up, and which is why the preset list lives in Rust beside
the enum that documents the wire.

Nor can it be guessed. An unrecognised field is a `400` on a strict endpoint, not a field politely
ignored, so `Reasoning::None` — send nothing — is both the default and the right answer for most
endpoints, reasoning models included: there is nothing to suppress there either.

There is no `openai` arm, and `reasoning_effort` is why. Its floor is `minimal`, not off — the model
still reasons — so sending it for `thinking = false` would be claiming something untrue, and
suppression is the entire reason this field exists. It is not accepted uniformly either: the o-series
takes low/medium/high and rejects `minimal`, so an `openai` arm would need per-model knowledge, which
is exactly what putting the dialect on the endpoint got rid of.

> **Amended 2026-08-24 — the arm now exists.** GPT-5.6 added `reasoning_effort: "none"`, a real floor
> where the model answers without reasoning, which refutes the first half above: suppression became
> expressible, so `Reasoning::OpenAi` is an arm. The second half was not refuted but fulfilled — the
> arm does carry per-model knowledge, as `EFFORT_NONE_FAMILIES` in `llm/request.rs`, for the same
> reason `Reasoning::Deepseek` consults `CATALOG`: one host serves models that disagree about the
> field. That narrows "never of the model" rather than reversing it — the row still decides *which*
> field could be sent, the model only whether it is, and an unmatched model stays silent rather than
> erroring. The decision this ADR records — the dialect on the row, no host guessing outside
> `Reasoning::guess` — stands unchanged.

## What left, and what did not

**`auth` never existed.** It was in the first draft as `bearer | none` and was cut: the header is sent
when a key is stored for the row and not when there is none. An explicit field can be wrong in two
ways a rule cannot — `none` beside a stored key ignores it silently, `bearer` with no key refuses a
turn the endpoint would have served.

That rule has a consequence ADR-0005 did not have to consider: **nothing stored for a local endpoint
is a working setup, not a fault.** A local server wants no `Authorization` header at all. So "no
credential" is still its own kind, distinct from a store that could not be read and from a key the API
rejected — but it is only *raised* where the host is remote. `Provider::is_local` is the one place
that decides, and it treats a host it cannot place as remote: sending nothing to something that
wanted a key fails as a 401 the user then has to decode.

**`temperature` came back, differently.** ADR-0019 removed it from Action files and pinned 1.3 in the
provider module, on the grounds that DeepSeek's own guidance gives that number and the module gets to
hold an opinion about DeepSeek. Both halves still hold — and neither is a fact about anybody else's
endpoint. It is now an optional value on the row, absent meaning send none and let the endpoint
decide, with 1.3 sitting on the DeepSeek row it came from. ADR-0019's actual decision — that a
*temperature slider* is not a setting this product offers — is untouched: there is still no
per-Action temperature and no control for it anywhere in Settings.

**One hard error survives, and only one.** A model that always thinks, asked to stop, is still a
refused turn: omitting the field there leaves thinking on and adds invisible seconds to every turn,
which is the failure the README wants gone. The opposite — thinking asked for and impossible to
express — is now an amber line in Settings and an omitted field, *not* a refused turn. Refusing it
would break every Action repointed at an endpoint without a switch, which is the move this whole ADR
exists to make easy. The old code refused any unrecognised `deepseek-*` id; that rule was inferring
the dialect from the model, and the row states it now.

**The DeepSeek catalog stayed DeepSeek's.** `llm/models.rs` still describes exactly the models
DeepSeek documents, and it is still the single table both the dropdown and the request layer read. It
is now offered as a fallback **only for DeepSeek's own host**: offering `deepseek-v4-flash` as the
documented list for somebody's Ollama would be a dropdown of ids that endpoint has never served.

## The credential, per row

The account is `provider:{id}` under the same `Beckon` service, so a row's `label` can be renamed
without losing its key and changing its `id` deliberately does not carry one over. Consequences:

- `get_key_statuses` answers for the whole table in one read. The Connection pane draws the inventory
  at once, and N round trips to render one list is N chances to draw it half-answered.
- Deleting a row deletes its credential, in `save_config` rather than in `reload`: it is a consequence
  of that edit and not of every re-read. A row re-added later under the same id would otherwise
  silently inherit the old key.
- The pre-provider account, `api-key`, is copied onto the default row once at startup and then **left
  in place**. Deleting somebody's credential is not a migration's business, and a downgrade to a build
  without the provider table then still works. It costs one dead entry in a store the user can see.

## Migration

`Config::fold_legacy` runs on load. An empty provider table means the file said nothing about
providers, so one row is synthesised from `[api] base_url`, `[defaults] model` and `[defaults]
thinking`; the legacy keys never serialise, so the next write drops them. The file itself is not
rewritten on load — silently rewriting a config is the data loss ADR-0003 warns about.

The row's `reasoning` is the **one host guess in the codebase**: `deepseek.com` folds in as
`deepseek`, `dashscope` as `qwen`, everything else as `none`. It is safe precisely because it runs
once, on a file whose `base_url` defaulted to DeepSeek's own host — and folding a `base_url` pointed
elsewhere in as `deepseek` would start sending a `thinking` object that endpoint has always rejected.
The pinned 1.3 travels the same way, for the same reason.

`fold_legacy` also establishes the two invariants everything downstream leans on: the table is never
empty, and `[defaults] provider` always names a row that is there. A row with a blank `id` cannot be
named by an Action, so it is not a row. `Config::default` calls the same function, so "what a fresh
install has" cannot drift from "what an old file becomes".

## What Settings looks like now

The **Connection** pane stopped being a switch and became an inventory: a card per endpoint, with its
host, its Action count, `stays on this machine` where that is true, and `no key` where that is a
problem. One row carries a `Default` badge, and that badge is where the pane spends its one
inversion — so the card behind it does not also fill, because two claims on one row is no claim. It
is deliberately not an *active* marker: several rows can be in use at once.

A row opens its own screen (ADR-0012) holding the fields above. `name` and `id` share one card —
they are one fact with two halves, the word you read and the word an Action writes — and the id half
is plain selectable text, not a disabled input: editing it there would orphan a stored key and break
every Action naming the row, and a greyed-out box says "not now" where nothing says "not here". Only
a row the user typed themselves shows `reasoning`: a preset carries the right value, and putting that
field on screen there is an invitation to break it.

A preset's `model` is a starting value, not a claim: filled where the vendor publishes a stable id,
and left empty where their ids carry dated `-preview` suffixes that rot. A rotted id is a `400` the
user has to decode; an empty one sends them to the dropdown, which is where the endpoint's own list
lands anyway.

Clicking a nav row always lands on that section's **first layer** — Connection's inventory, Actions'
list — so the nav column is the way back out of both, and returning to a section never resumes a
screen left open minutes ago somewhere else in the window.

A row an Action names **cannot be removed**. Which endpoint those Actions should use instead is not a
decision that button gets to make, and repointing them silently would move where somebody's text goes.

The **Model defaults** pane is gone. Its two fields describe a request, and which of them can be
honoured is a fact about an endpoint, so they moved onto the row. What is left global is one field —
which row is the default — and that belongs in the header sentence of the pane listing the rows, not
in a section of its own.

An Action's `Overrides` group grew a third row and needed nothing new for it: `Field`'s `override`
prop — the dot in the label gutter, the revert control naming the inherited value — already drew
exactly this for `model` and `thinking` (ADR-0011).

## The bug a prototype found

Overriding an Action's provider while a model is pinned leaves that model stranded: it is not in the
new endpoint's list, and `ModelSelect` refuses to write `""` rather than silently rewriting the
model — the right refusal, and a blank select.

The rule everywhere in this codebase is that a configured value is surfaced, never rewritten, so
`get_models` gathers `configured` **per provider**: the row's own model plus every model any Action
resolving to that row names. The stranded value then appears in its own dropdown, in red, with the
revert control beside it and a sentence saying it is kept and not rewritten.

## Consequences

- `llm/deepseek.rs` is now `llm/request.rs`. It is still the only place a wire divergence lives; the
  divergence is just per-endpoint rather than per-vendor.
- `client.rs` takes `Option<&str>` for the key, and `signed()` is the one place "no key means no
  header" is written.
- `ModelParams` carries a provider **id**, not a row. The row is re-read at request time, so a
  `base_url` corrected while a Popover is open reaches the next follow-up, and a row deleted under one
  is reportable rather than a panic.
- `get_models` and `test_connection` take a provider id. There is no such thing as "the" model list.
- First run is still "no key readable" (ADR-0005), asked of the default row only — and never for a
  local one, which wants no key.
