# Beckon

Beckon is a tool that lives in the background on Windows: press a hotkey to summon it, send a preset prompt plus your current input to an OpenAI-compatible endpoint — DeepSeek by default — and get the result streamed back in a popover next to your cursor. The name comes from "beckoning" — you wave, it comes.

## Language

### Core concepts

**Action**:
A preset prompt together with how it is triggered. This is the basic unit users configure and invoke.
_Avoid_: Skill, Command, Preset, feature

**Input Source**:
A property of an Action declaring where its input comes from — `auto` (use the Selection if there is one, otherwise ask for typed input) or `prompt` (typed input only; the Selection is ignored). Two values, not three: `selection` was retired by ADR-0020 and still loads as `auto`.
_Avoid_: input mode, mode

**Provider**:
One endpoint requests can go to: a row of `[[api.providers]]` holding a `base_url`, a model, how that
endpoint is told not to think, and the id its stored key is filed under. Every Action names one or
inherits `[defaults] provider`, so several can be in use at once — which is why there is no "active"
provider and no global switch to describe (ADR-0021). A row is always the user's own file: never a
Rust enum of vendors, and never an aggregator standing in for one.
_Avoid_: active provider, current provider, backend, vendor, API (the API is the protocol, a Provider
is one host that speaks it), model (a Provider serves models; it is not one)

**Web Search**:
An Action asking its endpoint to read the live web before answering, off unless the Action says
otherwise (ADR-0026). A switch on a turn, never a step Beckon performs: Beckon issues no search of
its own and reads no page — it sets the field that endpoint documents, and the endpoint searches.
Where a row has no such field, the switch reaches nothing and the pane says so.
_Avoid_: browsing, grounding, online mode, RAG, retrieval, live search (xAI's name for their own
field, not the name of the switch)

**Selection**:
The text the user has highlighted in a program outside Beckon, obtained by simulating Ctrl+C.
_Avoid_: highlighted text, selected region, text grab

**Capture**:
An image of part of the screen, taken with the platform's own snip tool from the Popover and read
off the clipboard. A peer of the Selection, not a kind of it: the Selection is text the user had
already highlighted, a Capture is made on request. A turn carries up to four of them, in the order
they were taken (ADR-0017). In UI strings it is called a *screenshot* — "Capture" is the word for
code, comments and commit messages.
_Avoid_: snip, screen grab, attachment, image (all three are what it is made of, not what it is)

### Triggering

**Launcher**:
The searchable list of Actions summoned by the global hotkey. It is the universal entry point to every Action.
_Avoid_: panel, main window, palette

**Direct Hotkey**:
A dedicated hotkey bound to a single Action. Its only reason to exist is zero interaction — once pressed, the user waits for the result and makes no choices.
_Avoid_: hotkey, shortcut (both are ambiguous and may refer to the global hotkey)

### Execution and presentation

**Popover**:
The lightweight floating window that pops up near the cursor. It hosts the input box, the streamed result, and any follow-up questions. Closing it destroys it.
_Avoid_: result window, floating window, toast, panel

**Exchange**:
The multi-turn conversation opened by one Action trigger, with a lifetime equal to that of the Popover. Discarded when the window closes; never persisted.
_Avoid_: session, Session, Conversation, history

### The same words in Chinese

Beckon ships in English and Simplified Chinese (ADR-0015), so each term above has exactly one
Chinese form, and `src/lib/i18n/zh.ts` is where it is kept. A second rendering of one of these is
the same failure as a synonym in English.

| Term | 中文 | Note |
| --- | --- | --- |
| Action | Action | Untranslated on purpose: it is also the file in `actions/`, and the filename is the identity |
| Input Source | 输入来源 | Its two values are 自动 / 手动输入 — the *values* stay `auto` / `prompt` on disk |
| Selection | 选中内容 | _Avoid_: 选区, 选中文本 |
| Capture | 截图 | _Avoid_: 屏幕截图, 抓图, 图片 |
| Launcher | 启动器 | |
| Direct Hotkey | 专属热键 | _Avoid_: 快捷键 alone, which is any hotkey |
| Popover | 浮窗 | _Avoid_: 弹窗, which is a dialog |
| Exchange | 对话 | Only ever the one a Popover holds |
| Web Search | 联网搜索 | The switch on an Action. _Avoid_: 网络搜索, 联网, 搜索 alone |
| Provider | 端点 | The row, and what Settings calls the section. _Avoid_: 服务商 (a company, not a row — two rows can point at one company), 后端, 供应商 |
