---
status: accepted
---

# One process, and a second launch opens Settings

Beckon claims a single-instance lock at startup. A launch that finds the lock already held does not
start a second tray process: it tells the running one to open Settings, and exits. Implemented with
`tauri-plugin-single-instance`, registered as the first plugin in `main`.

Before this, every launch started another copy. Nothing said so — Beckon has no window on startup and
its tray icon is one silhouette — so the second, third and fourth copies were invisible, and the way
you found out was Task Manager.

## Why a second copy is not merely wasteful

The cost is not the memory. Every one of Beckon's resident mechanisms assumes it is the only one
holding it:

- **Hotkeys are exclusive.** `RegisterHotKey` and its macOS equivalent hand a chord to one process.
  The second copy loses every registration it asks for, so `reload::apply_hotkeys` reports failures,
  `tray::set_error` fires, and the balloon the README promises is never silent tells the user their
  hotkeys are broken — about a process they did not know they had started. The hotkeys still work;
  they belong to the first copy.
- **The filesystem is the source of truth (ADR-0003), and now there are two writers.** Two watchers
  see each other's saves. `SelfWrites` suppresses the echo of *this* process's write, which is
  precisely the wrong shape when the other writer is a different process: its debounced save arrives
  as a genuine external edit, gets adopted, and the two Settings windows can walk a value backwards
  between them.
- **The credential store and the update channel are per-machine, not per-process.** Two copies check
  for updates on the same launch, and two can decide to install one. ADR-0022 already refuses an
  install while a Popover is open; it has no way to refuse an install because another copy of Beckon
  is mid-download.
- **Two tray icons.** Quitting from one leaves the other, still holding the hotkeys, and the tray is
  the only way Beckon quits.

None of that is fixable by making the second copy better behaved. The fix is that there is no second
copy.

## Why the second launch opens Settings

Something has to happen, or the user double-clicks Beckon and gets nothing at all — which is the
same non-answer as before, just cheaper.

Settings is the right target because it is the only surface a *launch* can be asking for. The other
two are summoned, not opened: the Launcher answers a hotkey next to the cursor, and the Popover is
where an Action's answer lands. Neither is a thing you double-click an icon to get, and opening
either from a Start-menu click would put a floating card on screen with no Action behind it.

A person launching an app that is already resident is nearly always asking one of "is this thing
running?", "where do I change something?", or "how do I quit it?". Settings answers the first two by
appearing, and the tray menu — which the act of launching also brings to their attention — answers
the third. It is also exactly what `setup` already does on first run when no key is readable, so the
path is one that existed, not one this ADR invents.

## The lock is released before the updater restarts

`AppHandle::restart` spawns the successor process and *then* exits the current one, and
`cleanup_before_exit` does not reach plugin `RunEvent::Exit` handlers. So during the update install
of ADR-0022 there is a window in which the new copy could ask "is Beckon already running?" and be
answered yes by the copy that is in the middle of dying — and then exit, leaving the user with an
updated Beckon that is not running.

Booting a Tauri app takes orders of magnitude longer than the old process's remaining `exit(0)`, so
this is a race Beckon would win nearly every time. Nearly every time is not a property worth
shipping when the plugin exports `destroy` for exactly this, so `update::install` calls it on the
main thread immediately before `restart`. The Windows path never reaches that code — the NSIS
installer ends the process itself — but it is the same handle either way, and a comment is cheaper
than a platform split.

## Consequences

- **The plugin is registered first, ahead of every other plugin.** Plugin setups run in registration
  order and the app's own `setup` runs after all of them, so exiting inside the single-instance setup
  happens before the tray exists, before `apply_hotkeys`, and before the watcher spawns. Registering
  it later would mean a doomed process had already claimed things on its way out.
- **`load_state` still runs in the second process.** It reads config, loads the Registry and seeds
  the examples if they are absent, all before the builder — and therefore the plugin — exists. Every
  one of those is idempotent and none of them writes when the files are already there, so the cost is
  a few milliseconds of disk on a process that is about to exit. Moving state construction after the
  lock would mean managing it after the windows can already invoke commands, which is the thing the
  comment at the top of `main` exists to prevent.
- **The callback runs on the running process's event-loop thread** — a `WM_COPYDATA` handler on
  Windows, a Tokio task on macOS. `trigger::show_settings` already spawns, because
  `WebviewWindowBuilder::build` deadlocks on the main thread on Windows, so the pump is not blocked
  and no new rule is needed.
- **The lock is per identifier, not per version**, because the plugin's `semver` feature is off. Two
  differently-versioned Beckons are still one Beckon, which is what a self-updating app wants: the
  point of the lock is the hotkeys and the config directory, and both are shared across versions.
- **`rust-version` moves to 1.77.2**, the plugin's MSRV. Nothing else in the manifest changes.
- **On macOS the second launch has to be a second *process*.** Beckon is `LSUIElement`
  (ADR-0013), so there is no Dock tile to click, and Finder reopening an already-running `.app` is
  answered by LaunchServices with the copy that is running — no process starts, the socket is never
  notified, and no Settings window appears. What the lock catches there is `open -n`, the binary run
  from a shell, and the case below. Windows has no such shortcut: every launch is a process, so
  every launch reaches the mutex.
- **A development build and an installed build collide.** Both carry `com.beckon.app`, so
  `tauri dev` while an installed Beckon is resident now opens the installed copy's Settings and the
  dev build exits. Quitting the installed one from its tray first was already necessary — the two
  fought over the same hotkeys and the same config directory — but the failure is now immediate and
  legible instead of being a tray error icon.
