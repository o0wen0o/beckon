// 简体中文。`Strings` 由英文目录推导而来，所以少一个键就是一处编译错误 —— 这是
// 两份目录唯一的防漂移机制（见 en.ts）。
//
// 术语跟随 CONTEXT.md：Action 保持原词不译（它同时是 `actions/*.toml` 里的文件
// 名和产品里的专名），Selection 作“选中内容”，Launcher 作“启动器”，Popover 作
// “浮窗”，Direct Hotkey 作“专属热键”，Input Source 作“输入来源”。
import { IS_MAC } from "../platform";
import type { Strings } from "./en";

export const ZH: Strings = {
  words: {
    credentialStore: IS_MAC ? "钥匙串" : "Windows 凭据管理器",
    tray: IS_MAC ? "菜单栏" : "通知区域",
    autostart: IS_MAC ? "登录时启动" : "开机时启动",
    systemAppearance: IS_MAC ? "macOS 外观设置" : "Windows 外观设置",
    modifierAdvice: IS_MAC ? "Cmd、Control、Option 或 Shift" : "Ctrl、Alt 或 Shift",
    emptyGrabCause: IS_MAC
      ? "没有辅助功能权限时什么都读不到 —— 若是这个原因，设置里会说明。"
      : "以管理员权限运行的窗口完全无法读取。",
    settings: "设置",
    cancel: "取消",
  },

  inputSource: {
    selection: "选中内容",
    prompt: "手动输入",
    auto: "自动",
    cell: (label: string) => `输入来源：${label}`,
    repair: "修复",
  },

  failure: {
    auth: "API 拒绝了该密钥",
    network: "无法连接 API",
    http: "API 拒绝了这次请求",
    "no-credential": "尚未保存 API 密钥",
    "read-error": `无法读取${IS_MAC ? "钥匙串" : "Windows 凭据管理器"}`,
    interrupted: "回答提前中断",
    empty: "端点没有列出任何模型",
    config: "Beckon 的配置不支持这样做",
    "capture-too-large": "截图太大，无法发送",
    "capture-unreadable": "无法读取这张截图",
    fallback: "失败",
  },

  launcher: {
    searchPlaceholder: "搜索 Action…",
    searchLabel: "搜索 Action",
    listLabel: "Action 列表",
    escape: "Esc",
    noActions: "还没有 Action。",
    addInSettings: "在设置中新建",
    nothingMatches: "没有匹配 {query} 的 Action。",
    widen: "按退格键放宽搜索。",
    selected: (characters: number) => `已选中 ${characters} 个字符`,
    noSelection: "没有选中内容",
    move: "移动",
    run: "运行",
    settingsTitle: (accelerator: string) => `设置（${accelerator}）`,
  },

  popover: {
    close: "关闭",
    thinking: "思考",
    nothingToShow: "没有可显示的内容。",
    needsSelection: "{name} 需要选中内容，但当前没有选中任何文本。",
    selectAndRetry: "选中一段文本后再按一次热键。",
    typeYourInput: "输入要发送给 {name} 的内容。",
    waiting: "正在等待第一个 token",
    runningWaiting: "等待中",
    runningStreaming: "输出中",
    stop: "停止",
    firstInput: "输入内容…",
    followUp: "继续追问…",
    send: "发送",
    showAll: "展开",
    showLess: "收起",
    showThinking: "查看思考过程",
    hideThinking: "隐藏",
    failed: "失败",
    interrupted: "已中断",
    interruptedEmpty: "尚无输出即中断",
    cancelled: "已取消。",
    retry: "重试",
    openSettings: "打开设置",
    copy: "复制",
    copied: "已复制",

    capture: "截图",
    captureTooltip: (accelerator: string) => `截取屏幕（${accelerator}）`,
    removeCapture: "移除截图",
    captureCancelled: "没有截到任何内容。",
    captureRetry: (accelerator: string) => `按 ${accelerator} 再试一次。`,
    captureNote: "为这张截图写一句说明…",
    captureMeta: (width: number, height: number, kilobytes: number) =>
      `${width}×${height} · PNG ${kilobytes} KB`,
  },

  settings: {
    nav: {
      label: "设置",
      connection: "连接",
      actions: "Action",
      triggering: "触发",
      appearance: "外观",
      defaults: "模型默认值",
      attention: "此分区有需要处理的问题",
      openFolder: "打开文件夹",
    },

    status: {
      notSaved: (message: string) => `未保存 —— ${message}`,
      saving: "保存中…",
      standing: "改动会随输入即时写入磁盘。",
      rawFile: "该文件无法解析，因此用上方按钮保存，而不是随输入即时保存。",
    },

    connection: {
      title: "连接",
      lede: (store: string) =>
        `请求发往哪里，以及带上哪个凭据。密钥保存在${store}中，绝不写入文件。`,
      welcomeLead: "欢迎。",
      welcomeBody: " Beckon 需要一个 DeepSeek API 密钥才能开始工作。",
      getKey: "前往 platform.deepseek.com 获取密钥",
      credential: "凭据",
      apiKey: "API 密钥",
      save: "保存",
      saved: "已保存。",
      remove: "移除",
      removed: "已移除。",
      stored: "已保存 —— 结尾为",
      noKeyYet: "尚未保存密钥。",
      readError: (store: string, message: string) =>
        `无法读取${store}：${message}。请重新保存密钥以重建凭据。`,
      endpoint: "端点",
      baseUrl: "Base URL",
      baseUrlHint: "任何兼容 OpenAI 的端点。请求发往 /v1/chat/completions。",
      reachability: "连通性",
      reachabilityHint: "用已保存的密钥发送一个很小的请求。",
      test: "测试连接",
      testing: "测试中…",
      testOk: "密钥与 Base URL 均可用。",
    },

    triggering: {
      title: "触发",
      lede: "如何唤起 Beckon。每个热键在录制的一刻即注册生效。",
      hotkeyDeadLead: "有热键未生效。",
      hotkeyDeadBody: "请在下方录制另一个组合键；录制的一刻即注册生效。",
      permissionLead: "Beckon 无法读取选中内容。",
      permissionBody:
        " 抓取选中内容意味着向前台程序发送 Cmd+C，而 macOS 只允许在“隐私与安全性 → 辅助功能”中受信任的应用这样做。",
      permissionStillWorks:
        "热键仍会触发，需要手动输入的 Action 也照常可用。请在列表中打开 Beckon，然后回到本窗口。",
      openAccessibility: "打开辅助功能设置",
      summoning: "唤起",
      launcherHotkey: "启动器热键",
      launcherHotkeyHint: "若组合键已被占用，它会变红且不会保存。",
      autostartHint: (tray: string) => `Beckon 常驻在${tray}；随机器一起启动才是它的意义所在。`,
    },

    appearance: {
      title: "外观",
      lede: "同时作用于启动器、浮窗和本窗口。",
      theme: "主题",
      light: "浅色",
      dark: "深色",
      system: "跟随系统",
      themeHint: (appearance: string) =>
        `除非另行指定，Beckon 以浅色启动。“跟随系统”是唯一读取${appearance}的选项，并会实时跟随它。`,
      language: "语言",
      english: "English",
      chinese: "中文",
      languageHint:
        "同时作用于所有窗口以及托盘菜单。除非另行指定，Beckon 以英文启动；你的 Action 是你自己的文字，永远不会被翻译。",
    },

    defaults: {
      title: "模型默认值",
      lede: "每个 Action 在自身 {table} 表未另行指定时继承的设置。",
      catalogNotice: (cause: string) => `${cause} —— 现在显示的是文档记载的模型。`,
      catalogFallback: "无法获取模型列表",
      model: "模型",
      thinking: "回答前先思考",
      thinkingHint:
        "DeepSeek 默认会思考。对翻译这类 Action，开着它只会多出几秒延迟 —— 所以除非明确需要，此项保持关闭。",
      temperature: "温度",
      temperatureHint:
        "模型措辞的自由度。低值直白、可复现 —— 翻译或改写要的就是这一端；高值多变，也更容易跑偏。范围 0 到 2。",
      catalog: "模型目录",
      modelList: "模型列表",
      modelListHint: "刷新会向端点索取它自己的列表；文档记载的目录是兜底。",
      refresh: "刷新模型",
      loading: "正在加载模型…",
      live: "由你的 Base URL 所指端点列出。",
    },

    actions: {
      title: "Action",
      lede: "一个 Action 就是一段提示词，各自存为一个文件。文件名是它的身份；名称只是给你看的。",
      create: "新建 Action",
      count: (count: number) => `${count} 个 Action`,
      willNotParse: "无法解析",
      empty: "还没有 Action。一个 Action 就是一段提示词，各自存为一个文件。",
      gone: "该 Action 已不存在。",
      rawUnder: "无法解析 —— 以纯文本编辑",
      back: "返回 Action 列表",
      backTo: (name: string) => `返回 ${name}`,
      deleteTitle: (name: string) => `删除“${name}”？`,
      deleteBody: "文件 {file} 将从磁盘删除。此操作无法撤销。",
      deleteConfirm: "删除文件",

      hotkeyDeadLead: "此 Action 的专属热键未生效。",
      hotkeyDeadBody: "在热键被清除或改掉之前，对此 Action 的任何改动都无法保存。",
      clearHotkey: "清除专属热键",

      definition: "定义",
      definitionHint: "名称、用途，以及它发送的两段提示词。",
      trigger: "触发",
      inputSource: "输入来源",
      sourceHint: {
        selection: "只用选中内容。没有抓到内容时给出提示，不发送请求。",
        prompt: "只用手动输入的内容。任何选中内容都会被忽略。",
        auto: "有选中内容就用它，否则请求手动输入。",
      },
      directHotkey: "专属热键",
      directHotkeyHint: "可选。不设置时，此 Action 只能从启动器运行。",

      overrides: "模型覆盖",
      overridesNote: "未标记的项沿用模型默认值",
      thinkingHint: "会在第一个字出现前多花几秒。需要模型推理时值得，只是改写格式时不值得。",

      thisFile: "此文件",
      deleteLabel: "删除此 Action",
      deleteHint: (file: string) => `从磁盘删除 ${file}。此操作无法撤销。`,
      deleteButton: "删除 Action",

      name: "名称",
      nameWarning: "没有名称时，此 Action 在启动器中显示为它的文件名。",
      description: "描述",
      descriptionHint: "在启动器中显示于名称下方，并参与搜索。",
      systemPrompt: "系统提示词",
      systemPromptHint: "模型应当如何表现。在每次输入之前发送。",
      userTemplate: "用户模板",
      userTemplateHint: "{{input}} 会被选中内容或手动输入替换。留空表示只发送输入本身。",
      templateWarning: "此模板从未包含输入内容。",
      templateWarningShort: "用户模板从未包含输入内容。",

      saveFile: "保存文件",
      reloadsWhenItParses: "一旦能够解析，它就会立刻重新加载。",
    },
  },

  controls: {
    hotkey: {
      record: "录制…",
      change: "更改…",
      recording: "请按下按键…",
      clear: "清除专属热键",
      needsModifier: (advice: string) => `请加上 ${advice} —— 无修饰键的热键会在任何地方触发。`,
    },
    temperature: {
      low: "0 · 精确",
      mid: "1",
      high: "2 · 发散",
    },
    model: {
      configuredGroup: "由你的配置指定",
      unknownLive: "不在端点的模型列表中",
      unknownCatalog: "不是 Beckon 已知的模型",
      unknown: (model: string, why: string) => `${model} ${why}。因为你的配置指定了它，所以仍然保留。`,
      alwaysThinks: (model: string) => `${model} 始终思考，无法关闭。`,
      neverThinks: (model: string) => `${model} 无法思考，这样的请求会被拒绝。`,
    },
    field: {
      useDefault: (reading: string) => `使用默认值（${reading}）`,
      on: "开",
      off: "关",
    },
  },
};
