# VibeHex releases

VibeHex is this fork's macOS app: `VibeHex.app`, bundle ID
`ai.makeaiwork.vibehex`, signed with Denis Kazantsev's Developer ID (team
`3M7AWY4D32`) and notarized. It is a separate app from Hex (`anomaly.hex`):
macOS asks for Microphone and Accessibility again, Launch at login is set
again, and a Clean up on your own server key in the Keychain asks once to be
read by the new app. Settings, History, vocabulary, and models are shared,
because both use the `hex` data folder. Only one of them runs at a time
(`scripts/install-app.sh` stops a running Hex first).

## How updates reach an installed VibeHex

```ts
VibeHex (stable build, not HEX_LOCAL_BUILD)       // src/updater.rs
├── 10 s after launch, then hourly, and Check for updates in About
├── GET https://github.com/makeaiwork/VibeHex/releases/download/updates/stable.json
│     the release protocol's manifest (schema 1, app hex, artifact desktop)
├── version differs from the running one -> download macos-arm64 (the ZIP)
│     size and SHA-256 must match the manifest
├── unpack VibeHex.app beside the installed one, check its signature against
│     app/update-requirement.txt (identifier, team 3M7AWY4D32, notarized)
├── GET …/updates/stable-notes.json: every release's changes, newest first
│     (src/updater/notes.rs); the update sheet lists the staged release and
│     every one skipped since the running build, and opens by itself once
│     per staged version, bringing the window up without focus if closed
└── Restart to update swaps the bundles; a refusal leaves the old app as it was
```

It is Hex's own updater (`src/updater/macos.rs`), not Sparkle. Versions are
never compared: whatever `stable.json` names, if it isn't the running build,
is the update. A local build (`HEX_LOCAL_BUILD=1`) never checks.

The repository was `makeaiwork/hex` until November 10, 2026. Builds from
before then still poll `github.com/makeaiwork/hex/...`, and GitHub redirects
that (and its release downloads) to `makeaiwork/VibeHex`; the updater follows
redirects and only requires the download to stay on `github.com`. Never
create a repository named `hex` under `makeaiwork`: it would take the old
address and those installs would stop seeing updates.

## Publishing a version

First the notes. `docs/releases/next.md` collects, in plain words, what the
work since the last release changed for a person who uses the app; write a
line there with each change people notice (see Release notes below). Then:

```sh
scripts/publish-vibehex.sh notes 3.3.0-vibe.7   # next.md becomes 3.3.0-vibe.7.md, dated today
git commit -m "Release notes for 3.3.0-vibe.7" && git push
. /Volumes/DropboxSSD/dev/hex-env.sh
HEX_NOTARY_PROFILE=DINGDONE_NOTARY scripts/publish-vibehex.sh prepare 3.3.0-vibe.7
scripts/publish-vibehex.sh upload 3.3.0-vibe.7
```

`notes` renames `next.md` to `<version>.md`, sets its version, date, and
title, and starts an empty `next.md`; it stages both and commits nothing.
`prepare` refuses a commit without `docs/releases/<version>.md`, builds and
validates `VibeHex.app`, notarizes and staples the app and the DMG
(`scripts/release-app.sh`), checks the update ZIP against the update
requirement, and leaves in `dist/` the DMG, the ZIP, the manifest, the
release notes (the notes file's lines), the cumulative notes
(`stable-notes.json`: this release's lines on top of the published file's
entries, or, the first time, entries rebuilt from the earlier releases'
notes), and the commit.
`upload` refuses files that no longer match that manifest, creates the GitHub
release `v<version>` with the DMG and ZIP, then replaces `stable-notes.json`
and `stable.json` on the `updates` release, notes first. Without a step name the script runs both. Each version is
`X.Y.Z-vibe.N` of the checked-out crate version; never publish one twice.

`scripts/build-app.sh` copies the binaries from `target/release`; with
`hex-env.sh`'s `CARGO_TARGET_DIR` elsewhere, link `target/release` to
`$CARGO_TARGET_DIR/release` for the build, as for `scripts/install-app.sh`.

## Release notes

`docs/releases/<X.Y.Z-vibe.N>.md` (and `next.md`) are VibeHex's own notes;
the others in that folder are Hex's. The app carries them: Assistant answers
"what's new" from the releases since the version an install ran before
(`assistant::releases`), and offers what fits the person's settings. Each
file has front matter and short lines:

```markdown
---
version: 3.3.0-vibe.7          # next for next.md
date: 2026-11-12               # unreleased for next.md
labs: [files, stop-shortcut]   # `hex labs` names it added or changed
keys: [meetings.notes.writer]  # other `hex settings` keys it added or changed
---

# VibeHex 3.3.0-vibe.7

- New in Labs: Stop shortcut (macOS). Finish a dictation with another key.
- Changed: …
- Fixed: …
```

Write for someone who uses the app: no commit hashes, PR numbers, or code
names; each line two lines at most; the file under 1.5k tokens; nothing a
person can't notice. Tests (`scripts/check assistant::releases`) hold the
front matter to real Labs features and keys, and fail when a `v*-vibe.*`
tag has no notes file.

Not observed yet: a published update installed by a running VibeHex, and the
update sheet opening for one (its logic has tests; the `mac-update-sheet`
scene renders it).
