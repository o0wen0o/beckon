# Build on Tauri v2, and support only Windows in the first release

This is a tool that starts with the system and sits in the background all day, so memory footprint and startup speed decide whether it can be tolerated long-term — Electron's resident 150–300MB is too heavy for this role. Tauri v2 uses the system WebView, sits at around 30MB resident with an installer of a few MB, and its official global-shortcut, tray, clipboard, and autostart plugins cover nearly every native need this project has.

## Considered Options

**C# WinUI 3** is technically the more "correct" choice given a Windows-only scope: Win32 hotkeys, UI Automation, and the tray are all first-class citizens. It was rejected because it welds the cross-platform door shut for good; Tauri keeps a future macOS release possible, and the only cost is a bit of a Rust learning curve (this project has very little Rust code).

**Python + PySide6** would be the fastest to write, but its bundle size, startup speed, and distribution experience are all wrong for a permanently resident tool.

## Consequences

Locking the platform to Windows is a **scope decision, not a technical constraint**: platform-specific code — text grabbing, hotkeys, DPAPI — should be concentrated in a handful of modules rather than scattered through business logic. Otherwise we throw away the portability Tauri bought us when the time comes to ship on macOS.
