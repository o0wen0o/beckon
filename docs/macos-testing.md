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
| 7a | In the Popover, press the screenshot button (or Cmd+Shift+S), drag a region | Beckon's windows vanish while the snip runs — the Popover must not be in the shot — then the Popover comes back with a thumbnail above the box and the caret still in it. This is `screencapture -i -c` plus the pasteboard read (ADR-0016) |
| 7b | Repeat, and press **Esc** in the snip overlay instead of dragging | The Popover comes back saying nothing was captured. Nothing is sent, and any previously attached screenshot is still attached |
| 7c | Type a note beside an attached screenshot, then Send, with the Action's model set to `deepseek-v4-flash-vision-exp` | The image and the note go as one turn; the answer streams as usual |
| 7d | Same, with the model left at `deepseek-v4-pro` | The request is sent anyway; the provider's own error is shown verbatim (Beckon gates nothing on the model, ADR-0016) |
| 7e | Attach a screenshot, take a **second** one without sending the first | Both are attached, oldest tile first, and the note being typed survives (ADR-0017) |
| 7f | Take a **fifth** screenshot without sending | It is refused with "there is no room for another screenshot"; the four already attached stay attached |
| 7g | Click a tile in the rail, then press ← / → , then Esc | The screenshot opens full size over the whole window on a grey ground, the arrows walk the set and wrap, Esc closes the preview only — the Exchange and the draft are still there |
| 7g1 | With a preview open, click the grey space beside the image | It closes. A click on the image itself does not close it — that one zooms (ADR-0017) |
| 7g2 | Preview a **full-screen** snip | The whole image is on screen, no edge cut off, both arrows still in their places (ADR-0017) |
| 7g3 | In the preview, **scroll the wheel** up and down over the image | It zooms continuously, the title bar reads the percentage while zoomed, and scrolling back down stops at fit rather than shrinking past it. A trackpad's two-finger scroll is the same gesture |
| 7g4 | **Click** the image, then drag it, then click again | The first click goes to the image's own pixels, dragging pans, the second click returns to fit — and the click that ends a drag does *not* toggle it |
| 7g5 | Zoom in, then drag to each **corner** of the image | All four corners are reachable; nothing is stranded off the top or left edge (ADR-0017) |
| 7g6 | Zoom in, then press ← or → | The next Capture opens fitted, not at the previous one's zoom |
| 7g7 | Zoom in and look at the edges of the image area | No scrollbar appears on either axis, and the image does not jump smaller the moment it is zoomed (ADR-0017) |
| 7g8 | Zoom in, then drag the image and **release with the pointer outside the image**, over the grey | It pans and stays open — the release does not close the preview (ADR-0017) |
| 7h | Click a screenshot inside a *sent* turn's card | The same preview, walking that turn's images rather than the composer's |
| 7i | Send two screenshots with a note, then ask a follow-up | The follow-up carries no image of its own, and the answer can still refer to both (the history is resent, ADR-0004) |
| 7j | Drag each **edge and corner** of the Popover | It resizes from all eight, and cannot go below 380×200. An undecorated NSWindow has no border of its own, so this is `startResizeDragging` and nothing else (ADR-0018) |
| 7k | Resize the Popover, close it, then trigger any Action again | It opens at the size you left it, still next to the cursor. `[popover]` in `config.toml` holds it |
| 7l | Resize the Popover to an odd size on a **scaled** display, then trigger an Action several times over | `[popover]` in `config.toml` still reads what you dragged to: our own `set_size` is not written back as though it were a drag, so the remembered size never walks a pixel at a time (ADR-0018) |
| 8 | Open the Launcher hotkey while an app is fullscreen | The Launcher appears and takes focus |
| 9 | In Settings, click into the API key field and press **Cmd+V** | It pastes. If not, Tauri's default menu is missing and every text field in the app is crippled |
| 10 | With a Beckon window focused, press **Cmd+Q** | The app quits. From the menu bar's Quit item too |
| 11 | Record a new Launcher hotkey using Cmd and Option | The chip draws as glyphs (`⌥⌘T`), and the combination fires |
| 12 | Set an Action's hotkey to something already taken (`Cmd+Space`) | It goes red on the spot and is not saved |
| 13 | Look at the menu-bar icon, then break a hotkey so the error state fires | Two distinguishable icons. If both look like a black blob, the icon is being treated as a template |
| 14 | Switch the theme to **System** and flip macOS between light and dark | All three surfaces follow, live |
| 15 | Turn **Start at login** on, log out, log back in | Beckon is running |
| 16 | Quit Beckon, delete `~/Library/Application Support/Beckon/`, relaunch | The two example Actions are written and Settings opens on Connection |
| 17 | With no newer release published, open the menu-bar menu and click **Check for Updates…** | A notification says Beckon is up to date and names the running version. The item still reads "Check for Updates…" (ADR-0022) |
| 18 | Publish a release one version higher, wait for the check (or click the item), then click **Update to X…** with a Popover open | It refuses, saying to close the Popover first: installing ends the process and the Exchange is not saved (ADR-0004) |
| 19 | Close the Popover and click **Update to X…** again | The `.app` is replaced and Beckon relaunches itself, still in the menu bar, now reporting the new version. This is the macOS half of ADR-0022 — `restart` has to run on AppKit's thread and the bundle swap is not something a compiler can check |
| 19a | Repeat on an **Intel** Mac | Same. The release ships one universal `.dmg`, so an arm64-only artifact would surface here as "up to date" on a Mac that is not |
| 19b | Point `plugins.updater.endpoints` at a manifest signed with a **different** key, then check | It fails with a signature error rather than installing. The trust anchor is the key compiled into the running binary, and this is the only test that proves it |
| 19c | Turn **Check for updates automatically** off in Settings → Triggering, then restart Beckon with a newer release published | Nothing is said, and no request leaves. The menu-bar item still checks when clicked, and still finds it |

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
- **The automatic update check runs once per launch** — thirty seconds in, one HTTP GET to GitHub —
  unless it is turned off in Settings → Triggering. The menu-bar item asks either way (ADR-0022).

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

And one thing that is not the port's: the Windows update path (ADR-0022) has never been run either.
Install from the `.exe`, publish a newer release, and take the tray menu's **Update to X…** — the NSIS
installer should replace Beckon and bring it back to the tray without a wizard. The `.msi` is not part
of that check; it cannot update itself by design.

The unit tests cover the arithmetic (`platform::tests`); what they cannot cover is whether the
work area handed to it excludes the taskbar.
