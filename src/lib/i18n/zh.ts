// 简体中文。`Strings` 由英文目录推导而来，所以少一个键就是一处编译错误 —— 这是
// 两份目录唯一的防漂移机制（见 en.ts）。
//
// 术语跟随 CONTEXT.md：Action 保持原词不译（它同时是 `actions/*.toml` 里的文件
// 名和产品里的专名），Selection 作“选中内容”，Launcher 作“启动器”，Popover 作
// “浮窗”，Direct Hotkey 作“专属热键”，Input Source 作“输入来源”。
import { IS_MAC } from "../platform";
import { kb } from "./units";
import type { Strings } from "./en";

export const ZH: Strings = {
  words: {
    credentialStore: IS_MAC ? "钥匙串" : "Windows 凭据管理器",
    tray: IS_MAC ? "菜单栏" : "通知区域",
    autostart: IS_MAC ? "登录时启动" : "开机时启动",
    systemAppearance: IS_MAC ? "macOS 外观设置" : "Windows 外观设置",
    modifierAdvice: IS_MAC ? "Cmd、Control、Option 或 Shift" : "Ctrl、Alt 或 Shift",
    settings: "设置",
    cancel: "取消",
    dismiss: "关闭",
  },

  inputSource: {
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
    "no-model": "尚未为该端点选择模型",
    "capture-too-large": "截图太大，无法发送",
    "capture-too-many": "已无法再附带更多截图",
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
    copyFailed: "无法复制",

    captureTooltip: (accelerator: string) => `截取屏幕（${accelerator}）`,
    removeCapture: "移除截图",
    captureCancelled: "没有截到任何内容。",
    captureRetry: (accelerator: string) => `按 ${accelerator} 再试一次。`,
    captureNote: "为这些截图写一句说明…",
    captureMeta: (width: number, height: number, bytes: number) =>
      `${width}×${height} · PNG ${kb(bytes)} KB`,
    captureSet: (count: number, bytes: number) =>
      `${count} 张截图 · 共 ${kb(bytes)} KB`,
    viewCapture: "查看这张截图",
    capturePosition: (index: number, total: number) => `第 ${index} 张，共 ${total} 张截图`,
    previousCapture: "上一张截图",
    nextCapture: "下一张截图",
    closeCapture: "关闭截图",
    zoomHint: "点击放大，滚轮缩放",
    zoomOutHint: "点击还原，拖动移动画面",
  },

  settings: {
    nav: {
      label: "设置",
      connection: "连接",
      actions: "Action",
      triggering: "触发",
      appearance: "外观",
      attention: "此分区有需要处理的问题",
      openFolder: "打开文件夹",
      openFolderFailed: "无法打开该文件夹",
    },

    status: {
      notSaved: (message: string) => `未保存 —— ${message}`,
      saving: "保存中…",
      standing: "改动会随输入即时写入磁盘。",
      rawFile: "该文件无法解析，因此用上方按钮保存，而不是随输入即时保存。",
    },

    connection: {
      title: "连接",
      lede: (fallback: string, store: string) =>
        `你保留的端点。每个 Action 发往其中之一；未指定的 Action 发往${fallback}。密钥保存在${store}中，每个端点一个，绝不写入文件。`,
      welcomeLead: "欢迎。",
      welcomeBody:
        " DeepSeek 已配置好，只差一个密钥。也可以改用任何兼容 OpenAI 的端点（直连各家自己的主机）—— 本机端点则完全不需要密钥。",
      setUp: (label: string) => `配置${label}`,
      getKeyFrom: (host: string) => `前往 ${host} 获取密钥`,
      keyPageFailed: "无法打开密钥页面",
      addPreset: "从预设添加…",
      addBlank: "空白",
      defaultTag: "默认",
      defaultForNew: "未指定的 Action 使用它",
      makeDefault: "设为默认",
      usedByNone: "没有 Action 使用",
      usedByOne: (name: string) => `1 个 Action —— ${name}`,
      usedByMany: (count: number) => `${count} 个 Action`,
      staysLocal: "不出本机",
      missingKey: "无密钥",
      edit: (label: string) => `编辑 ${label}`,
      removeLabel: (label: string) => `移除 ${label}`,
      removeHint: "删除此行及其已保存的密钥。",
      removeBlocked: (names: string) => `${names} 将失去端点。请先把它们指向别处。`,
      removeLast: "最后一个端点不能移除。",
      back: "连接",
      unnamed: "（未命名）",

      endpoint: "端点",
      name: "名称",
      nameHint:
        "仅用于显示。旁边的 id 才是 Action 指向它的名字、以及密钥保存的账户 —— 要改请编辑 config.toml，并把密钥一并迁走。",
      baseUrl: "Base URL",
      baseUrlHint:
        "任何兼容 OpenAI 的端点，直连各家自己的主机。请求发往 /v1/chat/completions；结尾的 /v1 可以保留。",
      apiKey: "API 密钥",
      apiKeyHint:
        "已保存的密钥会作为 Bearer 头发送。未保存则不发送该头 —— 这正是本机端点需要的。",
      save: "保存",
      saved: "已保存。",
      remove: "移除",
      removed: "已移除。",
      stored: "已保存 —— 结尾为",
      noKeyYet: "尚未保存密钥。",
      unauthenticated: "无密钥 —— 请求将不带认证发出。",
      readError: (store: string, message: string) =>
        `无法读取${store}：${message}。请重新保存密钥以重建凭据。`,
      reachability: "连通性",
      reachabilityHint: "用该端点自己的密钥，向它发送一个很小的请求。",
      test: "测试连接",
      testing: "测试中…",
      testOk: "已连通 —— 端点有响应。",
      testOkDetected: (dialect: string) => `已连通 —— 端点有响应，使用的是 ${dialect} 形式。`,
      relaysLead: "会转发。",
      relaysBody: (broker: string) =>
        `你的密钥留在 ${broker}；你的文本会继续转发给实际提供下方所选模型的公司。看到这段文本的是两家公司，不是一家。`,

      rowDefaults: "此端点的默认值",
      rowDefaultsNote: "Action 可覆盖其中任一项",
      refresh: "刷新模型",
      loading: "正在加载模型…",
      live: "由该端点列出。",
      cached: "此前由该端点列出。",
      listNotice: (cause: string) => `${cause} —— 现在显示的是此前列出的模型。`,
      noModelsYet: "请先为该端点保存密钥，然后点击刷新模型。",
      noModelsYetLocal: "请先启动该端点，然后点击刷新模型。",
      listUnavailable: "无法获取模型列表",
      listRefreshed: (count: number) => `该端点列出了 ${count} 个模型。`,
      modelAdopted: (model: string) =>
        `${model} 是该端点提供的唯一模型，已设为它的模型。`,

      reasoning: "思考开关形式",
      reasoningName: {
        deepseek: "DeepSeek",
        qwen: "Qwen",
        openai: "OpenAI",
        minimax: "MiniMax",
        openrouter: "OpenRouter",
        none: "无",
      },
      reasoningHint: {
        deepseek: '发送 thinking:{type:"enabled"|"disabled"}。DeepSeek V4 不特别说明就会思考。',
        qwen: "发送 chat_template_kwargs.enable_thinking。适用于 vLLM、SGLang 或 DashScope 上的 Qwen3。",
        openai:
          '关闭思考时发送 reasoning_effort:"none"，开启时什么都不发送。适用于 OpenAI 自己的接口，自 GPT-5.6 起。',
        minimax:
          '发送 thinking:{type:"adaptive"|"disabled"} 和 reasoning_split。适用于 MiniMax 自己的接口；其 M2 系列无论如何都会思考。',
        openrouter: '关闭思考时发送 reasoning:{effort:"none"}，开启时什么都不发送。',
        none: "两个方向都不发送 —— 由端点自己的默认行为决定。绝大多数端点没有这类开关，都属于这一项。",
      },
      thinkingHint: (label: string) => `关闭会显式发给 ${label}，因为它的模型不特别说明就会思考。`,
      thinkingHintNone: (label: string) =>
        `${label} 没有 Beckon 会用的开关，因此两个方向都不发送任何东西。`,
      thisEndpoint: "此端点",
    },

    triggering: {
      title: "触发",
      lede: "如何唤起 Beckon，以及它如何自我更新。每个热键在录制的一刻即注册生效。",
      hotkeyDeadLead: "有热键未生效。",
      hotkeyDeadBody: "请在下方录制另一个组合键；录制的一刻即注册生效。",
      permissionLead: "Beckon 无法读取选中内容。",
      permissionBody:
        " 抓取选中内容意味着向前台程序发送 Cmd+C，而 macOS 只允许在“隐私与安全性 → 辅助功能”中受信任的应用这样做。",
      permissionStillWorks:
        "热键仍会触发，需要手动输入的 Action 也照常可用。请在列表中打开 Beckon，然后回到本窗口。",
      openAccessibility: "打开辅助功能设置",
      openAccessibilityFailed: "无法打开该设置面板",
      summoning: "唤起",
      launcherHotkey: "启动器热键",
      launcherHotkeyHint: "若组合键已被占用，它会变红且不会保存。",
      autostartHint: (tray: string) => `Beckon 常驻在${tray}；随机器一起启动才是它的意义所在。`,
      updates: "更新",
      updateCheck: "自动检查更新",
      updateCheckHint: (tray: string) =>
        `每次启动后 30 秒检查一次，没有新版本就什么都不说。${tray}里的那个条目每次点击都会检查 —— 那不受这个开关管。`,
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
      deleted: (file: string) => `${file} 已删除。`,
      deleteFailed: "无法删除该文件",
      createFailed: "无法创建该 Action",
      rawOpenFailed: "无法读取该文件",
      rawSaved: (file: string) => `${file} 已能解析 —— 返回编辑器。`,

      hotkeyDeadLead: "此 Action 的专属热键未生效。",
      hotkeyDeadBody: "在热键被清除或改掉之前，对此 Action 的任何改动都无法保存。",
      clearHotkey: "清除专属热键",

      definition: "定义",
      definitionHint: "名称、用途，以及它发送的两段提示词。",
      trigger: "触发",
      inputSource: "输入来源",
      sourceHint: {
        prompt: "只用手动输入的内容。任何选中内容都会被忽略。",
        auto: "有选中内容就用它，否则请求手动输入。",
      },
      directHotkey: "专属热键",
      directHotkeyHint: "可选。不设置时，此 Action 只能从启动器运行。",

      overrides: "模型覆盖",
      overridesNote: (label: string) => `未标记的项沿用${label}`,
      provider: "端点",
      providerHint: "此 Action 发往哪里。不标记就跟随默认端点；一旦设定，此 Action 就不再跟随。",
      providerLocal: (label: string) => `${label} —— 本机`,
      strandedModel: (model: string, label: string, fallback: string) =>
        `${model} 不在${label}的列表中。它被保留而不会被改写 —— 恢复默认将使用 ${fallback}。`,
      needsKey: (label: string) =>
        `尚未为${label}保存密钥，此 Action 的请求还没发出就会失败。其他 Action 不受影响。`,
      sends: "此 Action 会发送什么",
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
      clearShort: "清除",
      needsModifier: (advice: string) => `请加上 ${advice} —— 无修饰键的热键会在任何地方触发。`,
    },
    model: {
      label: "模型",
      thinking: "回答前先思考",
      configuredGroup: "由你的配置指定",
      noneChosen: "未选择模型",
      unknown: (model: string) => `${model} 不在端点的模型列表中。因为你的配置指定了它，所以仍然保留。`,
      alwaysThinks: (model: string) => `${model} 始终思考，无法关闭。`,
      neverThinks: (model: string) => `${model} 没有思考模式；两个方向都不会发送任何东西。`,
      noThinkingSwitch: (label: string) =>
        `${label} 没有 Beckon 能使用的思考开关；由它自己的默认行为决定。`,
    },
    field: {
      useDefault: (reading: string) => `使用默认值（${reading}）`,
      on: "开",
      off: "关",
    },
  },
};
