# Testing the macOS build

The macOS half of `src-tauri/src/platform/` was written on a Windows machine and has never been
run. Its *types* are checked — `.github/workflows/ci.yml` compiles both platforms — but a compiler
cannot tell you whether a synthetic Cmd+C reaches Safari, or whether the Popover lands under the
Dock. Everything below needs a Mac and a person.

## Getting a build

Either is fine.

**On CI, without a Mac of your own.** Push the branch, then run the `bundle` job from the Actions
tab (`workflow_dispatch`). It uploads `beckon-macos-latest`, containing `Beckon.app` and a `.dmg`.
The artifact is unsigned, so a downloaded copy is quarantined — see the note at the bottom.

**On the Mac itself.**

```sh
npm install
npm run tauri dev        # or: npm run tauri build
```

Xcode Command Line Tools must be present (`xcode-select --install`); nothing else is needed.

## First launch

macOS will not ask for Accessibility permission until Beckon tries to post a key event, and it
never asks twice. So:

1. Launch Beckon. There should be **no Dock icon and no menu-bar app menu** — only the tray icon
   in the menu bar. A Dock tile means `LSUIElement` did not make it into the bundle.
2. Settings opens by itself (no API key stored yet). Store a DeepSeek key and press
   **Test connection**.
3. Go to **Triggering**. If the Accessibility callout is showing, follow its link, switch Beckon
   on in the list, and come back to the window — the callout should disappear on its own when the
   window regains focus. If it never showed, the permission was already granted from an earlier
   build with the same bundle id.

## The checks that actually need a human

Ordered by how likely they are to be wrong.

| # | What to do | What should happen |
|---|---|---|
| 1 | Select a sentence in **Safari**, press the Translate Action's hotkey | The Popover opens next to the cursor and translates the selected text. This is the whole port in one step: `CGEventPost`, the pasteboard poll, and the Accessibility permission |
| 2 | Repeat in **Notes**, **VS Code**, **Terminal**, and **Slack or Chrome** | Same. Electron and native apps take different paths through the pasteboard |
| 3 | Copy something distinctive first, then run step 1, then press Cmd+V somewhere | Your original clipboard content pastes, not the grabbed Selection (ADR-0002 restores it) |
| 4 | Trigger the Popover near the **bottom-right of the screen**, and again near the **top-left** | It stays fully on screen, never under the Dock and never behind the menu bar. This is `Monitor::work_area` on macOS |
| 5 | Same on a **second display**, especially one above or left of the main one (negative coordinates) | Still on screen, on the display the cursor is on |
| 6 | Same on a **Retina display** | Correct size, not half or double |
| 7 | Press Esc to close the Popover | Focus returns to the app you were reading — the *app*, not just some window of it |
| 8 | Open the Launcher hotkey while an app is fullscreen | The Launcher appears and takes focus |
| 9 | In Settings, click into the API key field and press **Cmd+V** | It pastes. If not, Tauri's default menu is missing and every text field in the app is crippled |
| 10 | With a Beckon window focused, press **Cmd+Q** | The app quits. From the menu bar's Quit item too |
| 11 | Record a new Launcher hotkey using Cmd and Option | The chip draws as glyphs (`⌥⌘T`), and the combination fires |
| 12 | Set an Action's hotkey to something already taken (`Cmd+Space`) | It goes red on the spot and is not saved |
| 13 | Look at the menu-bar icon, then break a hotkey so the error state fires | Two distinguishable icons. If both look like a black blob, the icon is being treated as a template |
| 14 | Switch the theme to **System** and flip macOS between light and dark | All three surfaces follow, live |
| 15 | Turn **Start at login** on, log out, log back in | Beckon is running |
| 16 | Quit Beckon, delete `~/Library/Application Support/Beckon/`, relaunch | The two example Actions are written and Settings opens on Connection |

## What to send back

For anything that fails, the useful report is:

- Which numbered step, and which app you were in.
- macOS version and whether the Mac is Apple Silicon or Intel.
- `~/Library/Logs/Beckon/` if it exists, or the output of `npm run tauri dev` if you built locally.
- For a placement bug (4–6): a screenshot including the whole screen, not just the Popover.
- For a grab bug (1–3): whether the Accessibility callout in Settings was showing at the time.

## Known gaps, so nobody reports them as bugs

- **Nothing is signed or notarized.** A build downloaded from CI is quarantined; run
  `xattr -dr com.apple.quarantine /path/to/Beckon.app` once, or Gatekeeper will refuse it and
  Accessibility permission will not stick.
- **Rebuilding changes the app's identity** as far as Accessibility is concerned. If the grab
  stops working after a rebuild, remove Beckon from the Accessibility list and add it again.
- The Mac App Store is foreclosed by `macOSPrivateApi` (ADR-0013). Direct distribution is not.

## The one thing to re-check on Windows

The port changed exactly one thing that runs on Windows: `platform::cursor` now reads the cursor
and the monitor's work area from Tauri instead of from `GetCursorPos` and `MONITORINFO.rcWork`
directly, so that macOS and Windows share one coordinate space (ADR-0013). Everything else under
`platform/windows/` is byte-for-byte what it was.

That makes Popover placement the Windows regression check, and it is 30 seconds:

1. Put the cursor a few pixels above the **taskbar** and fire an Action. The Popover must sit
   clear of the taskbar, not under it.
2. Fire one near the bottom-right corner. It must flip above/left of the cursor and stay on
   screen.
3. On a multi-monitor setup, fire one on the secondary display — including one positioned above
   or to the left of the primary, where coordinates go negative.
4. On a display at 150% scaling, check the window is the right size.

The unit tests cover the arithmetic (`platform::tests`); what they cannot cover is whether the
work area handed to it excludes the taskbar.
