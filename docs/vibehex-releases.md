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
├── 10 s after launch, then hourly, and Check for updates in the sidebar footer
├── GET https://github.com/makeaiwork/hex/releases/download/updates/stable.json
│     the release protocol's manifest (schema 1, app hex, artifact desktop)
├── version differs from the running one -> download macos-arm64 (the ZIP)
│     size and SHA-256 must match the manifest
├── unpack VibeHex.app beside the installed one, check its signature against
│     app/update-requirement.txt (identifier, team 3M7AWY4D32, notarized)
└── Restart to update swaps the bundles; a refusal leaves the old app as it was
```

It is Hex's own updater (`src/updater/macos.rs`), not Sparkle. Versions are
never compared: whatever `stable.json` names, if it isn't the running build,
is the update. A local build (`HEX_LOCAL_BUILD=1`) never checks.

## Publishing a version

From a clean commit pushed to `origin`:

```sh
. /Volumes/DropboxSSD/dev/hex-env.sh
HEX_NOTARY_PROFILE=<notarytool profile> scripts/publish-vibehex.sh 3.3.0-vibe.1
```

It builds and validates `VibeHex.app`, notarizes and staples the app and the
DMG (`scripts/release-app.sh`), checks the update ZIP against the update
requirement, creates the GitHub release `v3.3.0-vibe.1` with
`vibehex-<version>-macos-arm64.dmg` and `.zip`, then replaces `stable.json`
on the `updates` release. Each version is `X.Y.Z-vibe.N` of the checked-out
crate version; never publish one twice with different bytes.

`scripts/build-app.sh` copies the binaries from `target/release`; with
`hex-env.sh`'s `CARGO_TARGET_DIR` elsewhere, link `target/release` to
`$CARGO_TARGET_DIR/release` for the build, as for `scripts/install-app.sh`.

Not observed yet: a published update installed by a running VibeHex.
