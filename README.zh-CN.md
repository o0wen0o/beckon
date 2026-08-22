# Beckon

[English](./README.md) | 简体中文

一个常驻后台的 LLM 快捷工具，支持 Windows 与 macOS：按下热键唤起，把预设提示词连同当前输入一起发给 DeepSeek，结果以流式方式显示在光标旁的浮窗里。名字取自 "beckon"（招手示意）—— 你一招手，它就过来。

术语见 [CONTEXT.md](./CONTEXT.md)；架构决策见 [docs/adr/](./docs/adr/)。

> 本文档是 [README.md](./README.md) 的中文版。两者内容一致；若有出入，以英文版为准。

## MVP 范围

**包含**

- 常驻通知区域（macOS 上是菜单栏），随机器一起启动
- 全局热键唤起启动器（可搜索的 Action 列表）
- 一个 Action 还可以绑定专属热键 —— 按下即直达结果，零交互
- 通过模拟平台自身的复制快捷键抓取选中内容 —— Windows 上是 Ctrl+C，macOS 上是 Cmd+C —— 之后恢复剪贴板原有内容
- 光标附近的浮窗：获取焦点、流式输出、支持追问、按 Esc 关闭
- 浮窗里的截图按钮：调用系统自带的截图工具，把结果作为截图附上，与一起输入的文字作为同一轮发送（[ADR-0016](./docs/adr/0016-captures-from-the-os-snip-tool-via-the-clipboard.md)）。图片会发给该动作指定的模型；该模型能否读取图片，由服务端自己回答
- 完整的设置窗口：API 密钥、全局热键、主题、语言、全局模型默认值，以及 Action 本身 —— 一个 Action 分区列出全部 Action，每一个都可打开自己的编辑界面
- 英文与简体中文，在设置中切换（[ADR-0015](./docs/adr/0015-english-and-chinese-from-one-config-field.md)）
- Action 以 TOML 文件存储，文件监视器会在外部改动后自动重新加载
- 兼容 OpenAI 的 API，`base_url` 可配置

**明确不做**

| 不做的事 | 原因 |
| --- | --- |
| 持久化 Exchange／历史记录／检索 | [ADR-0004](./docs/adr/0004-exchanges-are-never-persisted.md) |
| Action 分类／标签 | 十几个 Action 的规模下，模糊搜索已经够用；将来用子目录实现也没有迁移成本 |
| 参数化 Action（调用时选择／填写变量） | 破坏了“按下就等结果”；双向翻译交给系统提示词里的模型自己判断即可 |
| “替换原文”式回写 | [ADR-0002](./docs/adr/0002-selection-via-simulated-ctrl-c.md) |
| 选中文本后自动弹出小图标（PopClip 那种） | 需要全局轮询选区 —— 费电、容易误触，而且与 Ctrl+C 抓取的思路冲突 |
| 显示 token 用量 | 一次快速翻译的成本不到一分钱；看到这个数字改变不了任何决定 |
| Linux | [ADR-0013](./docs/adr/0013-support-macos-alongside-windows.md) 把平台层移植到了 macOS；第三个平台没人提出需求。`platform/fallback.rs` 里的桩代码只是让 crate 在别处仍能编译，不是承诺 |
| 代码签名与公证 | 两个平台都没有配置。在 macOS 上，这是“只能在打包它的那台机器上运行”和“别人也能运行”之间的区别 —— 见 ADR-0013 |

## 已定行为

**触发与抓取文本**

- 全局热键在 Windows 上默认为 `Ctrl+Shift+Space`，macOS 上默认为 `Cmd+Shift+Space`：两边都是「Space 加两个修饰键」，只有第一个修饰键不同 —— 因为平台自己的启动器就是这样，Spotlight 是 `Cmd+Space`。两个默认值都避开了输入法组合键（微软拼音的 `Ctrl+Space` 中英文切换、macOS 的“上一个输入法”，`Shift+Space` 全角半角，`Win+Space` 切换输入法）、`Alt+Space`（系统窗口菜单，且常被 PowerToys Run／uTools 占用），以及任何 `Ctrl+Alt` 组合 —— 它在 ISO 键盘上就是 `AltGr`，既会参与字符合成，也正是抓取动作在复制之前必须先释放的修饰键状态。
- 如果抓取回来是空的，按 Action 的 `input_source` 处理 —— 这**不是错误**：`selection` 给出提示且不发送请求，`auto` 落回到输入框。
- 浮窗总是获取焦点。在显示之前记住前台是谁 —— Windows 上是窗口，macOS 上是应用 —— 关闭时把焦点交还。
- 在 macOS 上，抓取需要辅助功能权限，而系统会**静默**拒绝：选中内容只是回来是空的。设置直接读取该权限并说明情况，并附上前往对应面板的链接；热键本身无论如何都会触发。

**失败与等待**

- 不设超时。网络不通时 HTTP 层会立即报错，浮窗显示错误而不是空转。
- 界面必须区分“正在等待第一个 token”和“正在流式输出” —— 否则无从判断请求是否还活着。
- 请求失败 → 在浮窗内联显示错误，并给出重试按钮；不发系统通知。
- 截图先附上、再发送，绝不"截完即发"。取消截图**不是错误**：什么都没截到，也什么都没发出，浮窗如实说明。截到了但发不出去 —— 超过 Beckon 的 8 MB 上限，或字节无法解码 —— 会作为独立的原因说明，与取消区分开。
- 流中途断开 → 保留已经输出的内容，并在下方标记“已中断”。
- Esc 随时可以取消请求。

**配置**

- 在设置窗口录制热键时，**立即尝试注册**；若已被占用，当场标红，并拒绝保存一个注册不了的热键。
- 启动时的热键注册失败绝不静默：托盘图标切换到错误状态，外加一次性的气泡通知，点击即可打开设置。
- 模型是**从列表中选择，而不是手动输入**。已保存密钥且请求成功时，列表就是端点自己的 `/v1/models` 响应，否则是官方文档记载的 DeepSeek 模型 —— 没有凭据、密钥被拒或网络不通都会让列表降级并说明原因，但绝不会让它变空。`config.toml` 或某个 Action 里已经写明的模型始终可选，即使没有任何来源为它背书 —— 标记出来，而不是改写掉。
- 首次运行时（凭据存储里读不到密钥），直接打开设置窗口，其中带有“测试连接”按钮 —— 它会发送一个最小请求，当场验证密钥和 `base_url`。
- 首次运行时，若 `actions/` 不存在，写入两个示例 Action（一个 `selection` 类型、一个 `prompt` 类型），覆盖两条主要路径。一旦删除，不会再生成。
- 主题是 `light`、`dark` 或 `system`，同时作用于三个界面。**默认是 `light`**，即使机器的系统外观是深色也一样：系统偏好只有在 `theme = "system"` 时才会被读取，而这需要主动选择。
- 语言是 `en` 或 `zh`，同时作用于三个界面**以及**托盘菜单。**默认是 `en`**，在中文机器上也是如此，而且没有 `system` 这一档：系统区域设置是对**读者**的猜测，而不是一项设置，猜错了就会替换掉产品里的每一个词 —— 包括那些说明如何改回来的词（[ADR-0015](./docs/adr/0015-english-and-chinese-from-one-config-field.md)）。你的 Action 在任何方向上都不会被翻译：那是你自己的文字，在你自己的文件里。

## 配置文件布局

```
%APPDATA%\Beckon\                        # Windows
~/Library/Application Support/Beckon/    # macOS
├── config.toml        # 全局热键、开机自启、主题、语言、base_url、全局模型默认值
└── actions/
    ├── translate.toml
    └── ask.toml
```

API 密钥**不在这里** —— 它保存在操作系统的凭据存储中（Windows 凭据管理器，或 macOS 的登录钥匙串），服务名为 `Beckon`（[ADR-0005](./docs/adr/0005-api-key-in-windows-credential-manager.md)）。磁盘上任何地方都没有明文密钥文件。

配置目录可以在两个平台之间原样复制：`Ctrl`、`Alt`、`Shift` 和 `Cmd`／`Super` 在两边都能解析。不同的只是*默认值*，而且只在原装机器会拒绝注册的地方才不同。

一个 Action 的**身份是它的文件名**；`name` 字段只用于显示。

### config.toml

```toml
launcher_hotkey = "Ctrl+Shift+Space" # macOS 默认值是 "Cmd+Shift+Space"
autostart = true
theme = "light"                 # light | dark | system
language = "en"                 # en | zh

[api]
base_url = "https://api.deepseek.com"

[defaults]
model = "deepseek-v4-flash"
thinking = false
temperature = 1.3
```

### actions/translate.toml

```toml
name = "Translate"
description = "Chinese <-> English"
input_source = "selection"      # selection | prompt | auto
hotkey = "Ctrl+Alt+T"           # 可选；省略时只能从启动器调用

[prompt]
system = """
You are a translation engine. Translate Chinese input into English; translate any other language into Chinese.
Output only the translation — no explanation, no quotes, no prefix or suffix of any kind.
"""
# user 可以省略，默认为 "{{input}}"

[model]
temperature = 1.3               # 这三项都可以省略，省略时回落到 config.toml 里的默认值
```

### actions/ask.toml

```toml
name = "Quick ask"
input_source = "prompt"

[prompt]
system = "Answer concisely. Unless asked, do not enumerate bullet points and do not preamble at length."

[model]
thinking = true
```

## 技术栈

Tauri v2（Rust + web UI）。取舍理由和被否掉的方案见 [ADR-0001](./docs/adr/0001-tauri-v2-on-windows-only.md)；macOS 移植及其带来的改动见 [ADR-0013](./docs/adr/0013-support-macos-alongside-windows.md)。

两个平台在每次 push 时都由 [.github/workflows/ci.yml](./.github/workflows/ci.yml) 构建。`src-tauri/src/platform/` 有一半无法在 Windows 机器上编译，另一半无法在 Mac 上编译，所以一边构建通过并不能说明另一边。编译器查不到的部分记在 [docs/macos-testing.md](./docs/macos-testing.md) 里，连同这次移植所触及的那一处 Windows 行为。

DeepSeek 通过 `https://api.deepseek.com` 上兼容 OpenAI 的格式访问。当前模型为 `deepseek-v4-flash` ／ `deepseek-v4-pro`，100 万上下文，且**思考模式默认开启** —— 这正是全局默认值里有 `thinking = false` 的原因：翻译类 Action 若保持思考开启，只会多出好几秒延迟和一堆推理 token，毫无收益。旧名称 `deepseek-chat` ／ `deepseek-reasoner` 已于 2026-07-24 下线；Beckon 仍然认得它们，好让旧配置继续可用，但不再提供为可选项。
