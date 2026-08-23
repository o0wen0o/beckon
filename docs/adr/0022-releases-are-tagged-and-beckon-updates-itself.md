---
status: accepted
---

# Releases are tagged, and Beckon updates itself

Pushing a `v*` tag builds both platforms, publishes a draft GitHub release, and writes a signed
`latest.json` beside the artifacts. The installed app reads that manifest, verifies it against a
public key compiled into its own binary, and can replace itself. The version everything reports comes
from `package.json`, which is now the only file a bump edits.

Before this there were no tags, no releases, and one `workflow_dispatch` job that uploaded unnamed
CI artifacts. There was no answer to "how does a user get version two".

## Why an updater at all, and not "download the new installer"

Every other product decision in Beckon follows from it being resident and invisible: it starts with
the machine, it lives in the tray, and it is summoned by a hotkey rather than launched. That is the
whole point — and it means the user has no ritual that a new version could attach to. There is no
splash screen, no launch, no window they open on purpose. A release note nobody is looking for is not
a distribution channel.

The same property is what makes the check cheap to justify. A tray app is already running when the
question "is there a newer one" becomes answerable, and it already owns a menu and a notification
channel for exactly this shape of message — `tray::set_error` and its balloon were built for "something
needs your attention, and the tray is where you look for it".

## The signature is not code signing, and does not become it

Two different signatures are in play and conflating them would be the easy mistake here.

**minisign, over the artifact.** `plugins.updater.pubkey` is compiled into the binary; the private
half lives only in GitHub secrets and on the maintainer's machine. This is what makes the update
channel trustworthy: an attacker who takes over the release page still cannot produce a payload any
installed Beckon will accept. It is mandatory — `createUpdaterArtifacts` is on, so a build with no
key fails rather than producing a release that cannot be updated from.

**Platform code signing.** Still absent, on both platforms, exactly as the README's out-of-scope
table says. SmartScreen still warns and Gatekeeper still quarantines the first install.

The two are independent, and the order matters in the user's favour: the *first* install is the
unverified one, and every update after it is authenticated against a key that already-trusted binary
carries. Getting an updater before a code-signing certificate is therefore not backwards — it means
the number of unverified installs per user is one, forever, instead of one per version.

## NSIS updates; the MSI does not

Windows gets both bundles and only one of them can update itself. The updater replaces files in place
and hands off to an installer that must have those files closed; an MSI is a transaction the Windows
Installer service owns and cannot be driven that way. So `latest.json` names the NSIS `.exe`, and the
MSI stays in the release as the install-only target — the one an administrator deploys.

`installMode` is `passive`: a progress window, no wizard, no questions. A tray app's update should not
open a five-page installer, and `silent` would replace the app under the user with no visible sign
that anything happened.

macOS gets one **universal** `.dmg` rather than the runner's native arm64. The manifest names one
macOS asset, so an arm64-only build would leave Intel users with neither an install nor an update
path.

## Two checks, two voices

The automatic check runs once per launch, thirty seconds in, and speaks only when there is something
to install. Beckon starts at login, so once per launch is roughly once per day without a timer; the
delay is there because a check fired during boot fails about the network rather than about the
version. A resident app that announces "still up to date" at every login has trained its user to
dismiss the one notification that mattered.

The tray item is the loud one. A menu click that produces no notification reads as a broken menu, so
"up to date", "the endpoint is unreachable" and "the signature did not verify" are all said out loud
there. It is one item with two labels — `Check for Updates…` with nothing pending, `Update to 0.2.0…`
once a check has found something — and the click handler branches on the same value the label is
built from, so what it says and what it does cannot drift apart.

## An install is refused while the Popover is open

Installing ends the process. On Windows the NSIS installer requires it; on macOS the swapped bundle
has to be relaunched. Either way the Exchange in an open Popover is gone, and it was never on disk to
come back to (ADR-0004).

So a visible Popover is a refusal with a reason, not a race. This is the one place the update path
reaches into the rest of the app, and it reads the window rather than the Exchange deliberately: the
question is "is the user looking at something they would lose", and a Popover on screen answers it for
a streaming turn, a finished answer they have not copied yet, and a composer with typed input alike.

## Where the update state lives, and where the switch does

Not in Settings, and not in `config.toml` — the *state*, that is. ADR-0003 makes the filesystem the source of truth for
config and Actions, and the reload path broadcasts whole snapshots of both to the windows. An
available update is neither: nothing on disk has an opinion about it, no window renders it, and a
`config-changed` event carrying it would put a network result into the one channel whose whole
contract is "this is what the files say".

`AppState` holds it — `pending_update` and `updating`, beside `capturing` and `balloon_shown`, which
are the other two values that are process state rather than config — and the tray is the only surface
that reads them. `update.rs` imports `i18n`, `tray` and `state`, plus the Popover's window label from
`trigger` to answer the one question an install has to ask, and touches no command, no Exchange and
no pane.

The *switch* is the other half of that distinction, and it does belong in both. `update_check` is a
`config.toml` field with a row in Settings → Triggering, because whether a background process may
contact GitHub on its own is a preference a user holds — the kind of thing that survives a restart and
belongs in a file they can read. What the check *found* is not.

It governs the automatic check only. The tray item asks whenever it is clicked, switch or no switch,
and the hint under the switch says so: an explicit click is not the thing being declined, and a menu
item that silently did nothing would be the worse reading. Off means Beckon opens no connection it was
not asked to open — the only requests left are the turns the user started.

Settings → Triggering rather than a pane of its own: the two settings on that pane which are not the
hotkey are both about the resident process rather than about a request — `autostart` is the other —
and a fifth nav item for one switch is more chrome than the switch. The pane's lede says so now
("how Beckon is summoned, and how it replaces itself").

## One version, in `package.json`

`tauri.conf.json` reads `"version": "../package.json"`, so a bump edits one file. That version reaches
the bundle, the release, `latest.json` and `package_info().version` — and `latest.json` is compared
against it, so a version that disagrees with the tag is an app that offers to update itself to what it
already is. `Cargo.toml`'s `version` is now decorative and carries a comment saying so.

## Consequences

- `tauri-plugin-updater` is a dependency and a plugin; `src-tauri/src/update.rs` is the whole channel,
  ~200 lines, and it adds no IPC command — the tray reaches it directly. The only frontend change is
  the `update_check` switch: one field on `Config`, one `FieldGroup`, three catalog entries.
- Ten sentences enter `i18n.rs` and its two-language test. `TRAY_SURFACE` joins `MODIFIERS` as a
  per-platform wording pair: the sentence that sends a reader to the update item has to name the tray
  on Windows and the menu bar on macOS.
- `tray::menu` gains an item and `tray::retranslate` becomes one caller of a private `rebuild`, so the
  language and the pending version are always read out of state together.
- `Config` gains `update_check`, defaulting to `true` through the container-level `serde(default)` that
  fills a missing field from `Config::default()` rather than from `bool::default()`. It needs no
  `fold_legacy` clause: there is no invariant relating it to anything, which is the difference between
  a preference and the provider table.
- **The private key is now load-bearing.** Lose it and every installed copy is orphaned — there is no
  rotation path, because the key that would authorise a new key is the one that is gone. It belongs in
  a password manager, not only in GitHub secrets.
- `npm run tauri build` now needs a signing key. `scripts/build-signed.ps1` is the supported way to
  give it one: it sets the two variables for that process and writes nothing to the user environment,
  because a password in `HKCU\Environment` is readable by every process that account runs, forever, for
  the sake of one command. It can remember the password in `~/.tauri/beckon.pass` under DPAPI — useless
  to another account, useless copied elsewhere — and asks every time where there is no DPAPI. `tauri
  dev` and all four gates are unaffected: none of them bundle. The `bundle` job in `ci.yml` takes the
  secret for the same reason a local build takes the key.
