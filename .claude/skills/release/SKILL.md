---
name: release
description: "Use when shipping a Beckon version or publishing one already built — bump the version, tag, watch the release build, verify the draft, publish so installed copies see the update. Examples: \"release v0.1.1\", \"publish the draft release\", \"ship a new version\", \"cut a release\""
---

# Releasing Beckon

A tag is the only thing that ships (ADR-0022). Pushing `vX.Y.Z` makes
[release.yml](../../../.github/workflows/release.yml) build both platforms and leave a **draft**. A
draft is invisible to the updater endpoint, so nothing reaches a user until step 6 — that hand step
exists so step 5 can catch a bad build first.

Two entry points:

- **"release vX.Y.Z" / "ship a new version"** — start at step 1.
- **"publish the draft"** — the build already ran; start at step 5.

`0.1.1` / `v0.1.1` below stand for the version being shipped.

## Prerequisites (check once, not per release)

```bash
gh secret list          # TAURI_SIGNING_PRIVATE_KEY and ..._PASSWORD must both be present
```

The key must be the one matching `plugins.updater.pubkey`. `createUpdaterArtifacts` is on in
`tauri.conf.json`, so a missing key fails the build; a *wrong* key builds fine and produces a
`latest.json` no installed copy will accept.

## 1. Gates

Run before tagging — a failure after the push wastes the tag.

```bash
npx tsc --noEmit
(cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test)
```

Half of `src-tauri/src/platform/` cannot compile on the other platform, so this run is no evidence
about the platform you are not on. CI in step 4 is.

## 2. Bump the version

Read what already exists, so the new number is the next one:

```bash
gh release list
git tag -l
```

`package.json` is the only file that carries the version — `tauri.conf.json` reads `"version":
"../package.json"`. Leave `tauri.conf.json` alone.

```bash
npm version 0.1.1 --no-git-tag-version
```

**The number must equal the tag.** `tauri-action` names the release from the tag but writes
`latest.json` from `package.json`, so a mismatch ships a manifest whose version is not the assets',
and the updater compares against the wrong version. Step 5 reads the manifest back to catch this.

## 3. Commit and tag

Ask before pushing — a pushed tag starts a build that produces a public draft.

```bash
git add package.json package-lock.json
git commit -m "Release v0.1.1"
git tag v0.1.1
git push origin main
git push origin v0.1.1
```

## 4. Watch the build

Both pushes start a run — `ci.yml` on the `main` push, `release.yml` on the tag — so name the
workflow rather than letting `gh run watch` pick:

```bash
gh run watch "$(gh run list --workflow=release.yml --limit 1 --json databaseId -q '.[0].databaseId')"
```

`fail-fast: false`, so one platform can fail without cancelling the other and the run can finish
carrying one platform's assets only. Step 5 is what catches that.

## 5. Verify the draft

```bash
gh release view v0.1.1
```

Both platforms' assets must be present:

| Platform | Assets |
| --- | --- |
| Windows | NSIS `.exe` **+ its `.sig`**, `.msi` |
| macOS | universal `.dmg`, `.app.tar.gz` **+ its `.sig`** |
| Both | `latest.json` |

The NSIS `.exe` is the only Windows target that can update itself (the updater replaces files in
place; an MSI cannot be told to), so `latest.json` points at it and the `.msi` is install-only. The
macOS build is `--target universal-apple-darwin` because `latest.json` names one macOS asset — an
arm64-only build leaves every Intel Mac with no update path.

Then read the manifest, the only file an installed copy ever sees:

```bash
gh release download v0.1.1 --pattern latest.json --dir /tmp/beckon-release --clobber
cat /tmp/beckon-release/latest.json
```

It must carry a `version` equal to the tag without its `v`, a `windows-x86_64` entry, and a
`darwin-*` entry.

**A missing platform or key means do not publish.** `latest.json` names one asset per OS, so
publishing an incomplete draft breaks the missing platform's update path rather than leaving it
untouched. Delete the draft and the tag, fix, re-tag:

```bash
gh release delete v0.1.1 --yes
git push origin :refs/tags/v0.1.1 && git tag -d v0.1.1
```

Done when both table rows are present and all three manifest checks pass.

## 6. Publish

Confirm with the user before running this. It is irreversible in effect: the endpoint is
`releases/latest/download/latest.json`, so publishing is what makes the manifest fetchable at all,
and copies already installed will offer this version from then on.

```bash
gh release edit v0.1.1 --draft=false --latest
```

`--latest` is load-bearing: that endpoint resolves through GitHub's *latest release* pointer, so a
release published without holding it serves nobody, and moving the pointer back to an older release
would offer that older version as an update.

**Not done until** the release is reported as *published*, never as delivered:
`update::check_on_startup` runs once, 30 seconds after launch, and the tray's check is manual — a
user sees the update on their next launch, or when they ask.

## Not part of this flow

- **Code signing the installer.** Neither platform's installer is signed or notarized (ADR-0013).
  SmartScreen warns on Windows; macOS needs `xattr -dr com.apple.quarantine
  /Applications/Beckon.app` on first run. `release.yml`'s `releaseBody` says both. The signature
  ADR-0022 makes mandatory is a different one, over a different artifact — the updater manifest.
- **Local bundling.** `npm run build:signed` (or `pwsh scripts/build-signed.ps1` on macOS) builds
  with the key set for that one process. Useful for testing a bundle; it ships nothing.
- **Release notes.** `release.yml` writes `releaseBody` itself, the same text every release. Editing
  the draft's body is a choice, never a required step.
