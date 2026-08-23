# Beckon

[English](./README.md) | 简体中文

按下热键。刚刚选中的文字连同你自己写的提示词一起发给 LLM，回答以流式方式出现在光标旁的浮窗里。
然后它就退开。

Beckon 常驻通知区域，随机器启动，什么都不保存。名字取自 "beckon"（招手示意）—— 你一招手，它就过来。

**支持 Windows 与 macOS。** 自带密钥：任何兼容 OpenAI 的端点都能用，默认 DeepSeek，始终直连各家自己的
主机，绝不经过聚合中转。

> 本文档是 [README.md](./README.md) 的中文版。两者内容一致；若有出入，以英文版为准。

<!-- screenshot: the Popover mid-stream, over a browser -->

## 安装

从 [release](https://github.com/o0wen0o/beckon/releases) 下载：

| 平台 | 文件 | 说明 |
| --- | --- | --- |
| Windows | `Beckon_x.y.z_x64-setup.exe` | 推荐 —— 只有这个能自我更新 |
| Windows | `Beckon_x.y.z_x64_en-US.msi` | 能正常安装，但无法自我更新 |
| macOS | `Beckon_x.y.z_universal.dmg` | Intel 与 Apple Silicon 通用 |

没有做代码签名，所以**第一次**安装会遇到 SmartScreen 或 Gatekeeper。之后的每次更新都改由 Beckon 自有的
签名密钥校验。

在 macOS 上请授予**辅助功能**权限 —— 没有它，抓取选中内容会静默地什么都拿不到。设置会直接读取该权限，
并附上前往对应面板的链接。

## 首次运行

设置窗口会自己打开，因为还没有 API 密钥。

1. **端点** —— 默认那一行是 DeepSeek。粘贴密钥，点**测试连接**。本机端点（Ollama、LM Studio）完全不需要
   密钥。
2. **Action** —— 首次启动会为你写入两个示例。改它们，或者自己新建。
3. 按 `Ctrl+Shift+Space`（macOS 上是 `Cmd+Shift+Space`）打开启动器。

## 怎么用

一个 **Action** 就是一段存下来的提示词：系统提示词、模型，以及它从哪里取输入。你可以从启动器里挑一个，
也可以给它绑一个专属热键，彻底跳过启动器。

- **启动器** —— 全局热键唤起，输入即筛选，回车运行。
- **专属热键** —— 每个 Action 一个，按下即直达结果，零交互。
- **选中内容** —— 通过模拟平台自身的复制快捷键抓取，之后把剪贴板原有内容放回去。没有选中不是错误：
  浮窗直接给你一个输入框。
- **浮窗** —— 获取焦点、流式输出、支持追问。拖任意边或角可以改大小，下次打开就是这个尺寸。`Esc` 先取消
  正在进行的请求，再关闭。
- **截图** —— 浮窗里的按钮会调用系统自带的截图工具并把结果附上，一轮最多四张。它是先附上、再发送，绝不
  "截完即发"。模型能不能读图，是你和你的端点之间的事。

Beckon 只有一个进程。它已经驻留时再启动一次，打开的是设置窗口，而不是第二个副本。macOS 上它只待在菜单栏、
没有 Dock 图标，本来就没有可双击的东西 —— 重新打开只会把已经在跑的那一份交还给你。

界面全部双语 —— 英文与简体中文，三个窗口加托盘菜单一起切换，在设置里改。默认是 `en`，而且没有"跟随系统"
这一档：猜错区域设置会替换掉产品里的每一个词，包括那些说明如何改回来的词。你自己的 Action 在任何方向上
都不会被翻译。

## 配置

设置窗口能改全部内容，但这些文件也是你的 —— 监视器会在改动后重新加载。

```
%APPDATA%\Beckon\                        # Windows
~/Library/Application Support/Beckon/    # macOS
├── config.toml
└── actions/
    ├── translate.toml
    └── ask.toml
```

整个目录可以在两个平台之间原样复制：`Ctrl`、`Alt`、`Shift` 和 `Cmd`／`Super` 在两边都能解析，不同的只是
默认值。

**API 密钥不在这些文件里。** 它们保存在操作系统的凭据存储中 —— Windows 凭据管理器，或登录钥匙串 ——
服务名为 `Beckon`，每个端点一个账户。磁盘上任何地方都没有明文密钥。

### config.toml

```toml
launcher_hotkey = "Ctrl+Shift+Space" # macOS 默认值是 "Cmd+Shift+Space"
autostart = true
update_check = true             # 每次启动检查一次
theme = "light"                 # light | dark | system
language = "en"                 # en | zh

[defaults]
provider = "deepseek"           # 未指定 provider 的 Action 使用它

[popover]
width = 620.0                   # 由拖动窗口写入，不用手改
height = 500.0

# 每个端点一张表。放在最后：数组表会吞掉它之后的一切表头。
# `id` 既是 Action 指向它的名字，也是凭据账户名。
[[api.providers]]
id = "deepseek"
label = "DeepSeek"              # 仅用于显示
base_url = "https://api.deepseek.com"
model = "deepseek-v4-flash"
thinking = false                # DeepSeek 默认思考；关掉能省好几秒
reasoning = "deepseek"          # deepseek | qwen | none —— 如何告诉此端点*不要*
                                # 思考。这是端点的属性而非模型的属性，由预设填好。
temperature = 1.3               # 可选；省略即交给端点自己决定
key_page = "https://platform.deepseek.com/api_keys"

[[api.providers]]
id = "ollama"
label = "Ollama（本机）"
base_url = "http://localhost:11434/v1"
model = "qwen3:8b"
thinking = false
reasoning = "qwen"
# 没有 key_page，也不需要密钥：本机端点不会收到 Authorization 头。
```

可以有两个端点同时在用 —— 翻译走快而便宜的云端模型、总结走不出本机的模型，二者只隔一个热键，而不是隔
一趟设置。模型是从列表里选而不是手动输入：请求成功时列表就是该端点自己的 `/v1/models`，否则就是你的配置
已经写明的模型 —— 标记出来，而不是改写掉。旧格式的配置文件仍然可用，加载时会折叠成一行。

### 一个 Action

一个 Action 的**身份是它的文件名**；`name` 只用于显示。

```toml
name = "Translate"
description = "Chinese <-> English"
input_source = "auto"           # auto | prompt
hotkey = "Ctrl+Alt+T"           # 可选；省略时只能从启动器调用

[prompt]
system = """
You are a translation engine. Translate Chinese input into English; translate any other language into Chinese.
Output only the translation — no explanation, no quotes, no prefix or suffix of any kind.
"""
# user 可以省略，默认为 "{{input}}"

# [model] 整块都可以省略。每个键缺省即表示"继承"：
#   provider  [defaults] provider 所指的那一行
#   model     该行的 model
#   thinking  该行的 thinking
# 因此覆盖 `provider` 会同时改变另外两项继承的来源。
[model]
provider = "ollama"
thinking = true
```

## 更新

Beckon 每次启动检查一次，启动后 30 秒执行，没有新版本就什么都不说。托盘菜单是主动路径：平时是
`检查更新…`，有新版本时变成 `更新到 0.2.0…`。自动检查是"设置 → 触发"里的一个开关；托盘那一项则是点一次
就检查一次。

浮窗打开时会拒绝更新，并说明原因 —— 安装会结束进程，而对话从来不在磁盘上，回不来。

## 明确不做

| | 原因 |
| --- | --- |
| 历史记录、检索、保存对话 | Exchange 随窗口一起消失，这是故意的（[ADR-0004](./docs/adr/0004-exchanges-are-never-persisted.md)）|
| Action 分类或标签 | 十几个 Action 的规模下模糊搜索已经够用；将来用子目录实现也没有成本 |
| 需要现场填空的 Action | 破坏了"按下就等结果"；交给系统提示词自己判断即可 |
| 就地替换选中的文字 | [ADR-0002](./docs/adr/0002-selection-via-simulated-ctrl-c.md) |
| 选中文本后自动浮出小图标 | 需要全局轮询选区 —— 费电、容易误触，而且与复制快捷键抓取的思路冲突 |
| 显示 token 用量 | 一次快速翻译的成本不到一分钱；这个数字改变不了任何决定 |
| Linux | `platform/fallback.rs` 里的桩代码只是让 crate 仍能编译，不是承诺 |
| 代码签名与公证 | 两个平台都没有配置（[ADR-0013](./docs/adr/0013-support-macos-alongside-windows.md)）|

## 自己构建

Tauri v2 —— Rust 后端 + React webview。需要 Node 和 Rust 工具链。

```bash
npm install
npm run tauri dev             # 真正的应用，带托盘
npm run tauri build           # 安装包；需要更新签名密钥
```

`npx tauri signer generate` 生成的是一把一次性密钥，能打包，但签出来的东西别人不会接受。CI 强制的四道
关卡见 [CLAUDE.md](./CLAUDE.md)。

## 延伸阅读

- [CONTEXT.md](./CONTEXT.md) —— 术语表，一个概念一个名字，中英对照。
- [docs/adr/](./docs/adr/) —— 22 条决策，每条都写了被否掉的方案和原因。
- [docs/macos-testing.md](./docs/macos-testing.md) —— 编译器查不到的行为。
