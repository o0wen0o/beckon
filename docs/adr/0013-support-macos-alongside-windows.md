---
status: accepted
---

# Ship on macOS as well as Windows

[ADR-0001](0001-tauri-v2-on-windows-only.md) picked Tauri v2 over WinUI 3 precisely so that a
macOS release would stay possible, and said the platform lock was "a scope decision, not a
technical constraint" — on the condition that platform-specific code stay concentrated in a
handful of modules. That bill is now being paid, and the condition held: the port is three files
under `src-tauri/src/platform/macos/`, one `Paths` lookup, and a set of labels. No business logic
gained a `#[cfg]`.

**Everything ADR-0001 decided about the stack stands.** What changes is the scope sentence, and
the three ADRs that named Windows in their titles because Windows was the only platform:
[ADR-0002](0002-selection-via-simulated-ctrl-c.md) becomes "simulate the platform's copy
shortcut", and [ADR-0005](0005-api-key-in-windows-credential-manager.md) becomes "the OS
credential store". Neither decision changes — the mechanism was never the point, the property was.

## What is genuinely different, and what only looks it

Most of the port is substitution: `CGEventPost` for `SendInput`, `NSPasteboard` for the Win32
clipboard, `NSWorkspace` for `GetForegroundWindow`, the login Keychain for the Credential Manager.
Those are not decisions. Four things are.

**The grab needs a permission, and macOS refuses it silently.** `CGEventPost` reaches another
application only for a process the user has trusted under Privacy & Security → Accessibility. An
untrusted process gets no error and no event: the pasteboard simply never changes, the poll times
out, and the Action reports an empty Selection. That is indistinguishable from "nothing was
selected", which is a normal outcome the product is built to absorb — so the failure would be
invisible. `platform::permission::input_permission` therefore reads the state directly
(`AXIsProcessTrusted`) rather than inferring it, and Settings carries a `denied` callout with a
link to the pane. It is re-read whenever the Settings window regains focus, because the switch is
thrown outside Beckon and nothing tells us when.

Windows answers `not-required`, which is deliberately *not* `granted`: there is nothing to grant
there, so the UI says nothing at all rather than reporting a permission the user has never heard
of.

**The unit of focus is an application, not a window.** Windows remembers an `HWND` and restores
it; macOS remembers a pid and activates the `NSRunningApplication`. This is why
`focus::window_handle` returns *our own pid* for every one of our windows on macOS — "one of our
windows was in front" and "we were the active app" are the same statement there, which is exactly
what `is_ours` needs to know.

**Cursor placement stopped being per-platform at all.** The Windows implementation read
`GetCursorPos` and `MONITORINFO.rcWork`. macOS's own equivalents are in a bottom-left, Y-up space
that would have to be flipped into the top-left one `set_position` takes — and tao already does
that flip. Deriving it a second time in `platform/macos/` would be a second place for it to be
wrong, so `platform::cursor` now asks Tauri (`cursor_position`, `monitor_from_point`,
`Monitor::work_area`) on both platforms and the Windows path was retired. `place_near_cursor` is
untouched and still the tested part; it gained one test, because a macOS work area starts below
the menu bar and nothing in it may assume `area.y == 0`.

**Cmd+Q quits.** Tauri fits every macOS app with a default menu, which is the only reason Cmd+C
and Cmd+V work in Settings' text fields — and that menu owns Cmd+Q. Beckon's rule is that it quits
only from the tray, enforced by refusing every exit request that carries no code. On macOS every
such request is a person asking to quit (Cmd+Q, the Dock, logging out): there is no window-count
exit to guard against, because every window refuses to close. So the refusal is skipped there, and
Cmd+Q means what it says.

## Consequences

- **The default Launcher hotkey differs per platform**, because the conflicts do: macOS ships
  Ctrl+Option+Space as "select the next input source", so the Windows default would fail to
  register on a stock Mac and the first thing a new user would see is the tray's error icon. It is
  `Cmd+Shift+Space` there. Nothing else about the format changes — `Ctrl`, `Alt`, `Shift` and
  `Cmd`/`Super` all parse on both platforms, so a `config.toml` stays portable.
- **`macOSPrivateApi` is on.** The Launcher and the Popover are frameless cards over a transparent
  `<body>`, and a transparent window is private API on macOS. This forecloses the Mac App Store,
  which was never a target; direct distribution is unaffected.
- **The tray icon is not a template image**, which is the platform's default for a menu-bar item.
  A template is rendered from alpha alone, and Beckon's two icon states are one silhouette that
  differs only in accent colour — the state the README insists must never be silent is exactly the
  thing template mode would erase.
- **The app is an accessory** — `LSUIElement` in `Info.plist`, and `ActivationPolicy::Accessory`
  at startup. Both, not either: the policy call is what a `cargo run` gets, the plist is what stops
  a Dock tile existing for the instant before `setup` runs.
- **The `#[cfg(not(any(windows, target_os = "macos")))]` stubs stay.** They are not a promise of a
  Linux build; they are what makes the isolation testable. A Win32 or AppKit call that leaked into
  business logic breaks `platform/fallback.rs` first.
- **CI now builds both platforms** (`.github/workflows/ci.yml`). Half the platform layer cannot be
  compiled on a Windows machine and half cannot be compiled on a Mac, so a green build on one is
  no longer evidence. The workflow is the compiler for whichever half the contributor does not
  have.
- **Signing and notarization are not set up.** An unsigned `.app` is quarantined by Gatekeeper on
  first launch, and a quarantined app cannot be granted Accessibility reliably. Shipping to anyone
  but the developer needs a Developer ID and `notarytool` in the bundle job; running a locally
  built `.app` needs the quarantine attribute removed once.
