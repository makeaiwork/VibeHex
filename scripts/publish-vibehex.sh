#!/bin/sh
set -eu

# Publishes VibeHex <version> (X.Y.Z-vibe.N of the checked-out X.Y.Z) to the
# fork's GitHub releases, where every VibeHex checks for updates
# (docs/vibehex-releases.md):
#   1. scripts/release-app.sh builds, signs (HEX_TEAM_ID, default 3M7AWY4D32),
#      notarizes (HEX_NOTARY_PROFILE), and staples the DMG and update ZIP
#      from a clean, pushed commit;
#   2. release v<version> gets both files;
#   3. the `updates` release's stable.json names them, which is what running
#      apps read. A version is never published twice with different bytes.
#
#   HEX_NOTARY_PROFILE=<notarytool profile> scripts/publish-vibehex.sh 3.3.0-vibe.1
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
repo=${VIBEHEX_REPO:-makeaiwork/hex}
channel=stable
if [ "$#" -ne 1 ]; then
  echo "Usage: HEX_NOTARY_PROFILE=<profile> $0 <X.Y.Z-vibe.N>" >&2
  exit 1
fi
version=$1
current=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -1)
case "$version" in
  "$current"-vibe.[0-9]*) ;;
  *) echo "Publish $current-vibe.N, not $version." >&2; exit 1 ;;
esac
: "${HEX_NOTARY_PROFILE:?Set HEX_NOTARY_PROFILE to your notarytool keychain profile}"
export HEX_TEAM_ID=${HEX_TEAM_ID:-3M7AWY4D32}
tag="v$version"
if gh release view "$tag" -R "$repo" >/dev/null 2>&1; then
  echo "$tag is already published; pick the next vibe number." >&2
  exit 1
fi

# release-app.sh stamps the version into Cargo.toml and Cargo.lock; put them back.
trap 'git -C "$root" checkout -- Cargo.toml Cargo.lock' EXIT HUP INT TERM
HEX_VERSION="$version" HEX_BUILD_CHANNEL="$channel" "$root/scripts/release-app.sh" prepare >/dev/null
dmg="$root/dist/vibehex-$version-macos-arm64.dmg"
zip="$root/dist/vibehex-$version-macos-arm64.zip"
# The updater checks the same requirement before it swaps the bundle in.
work=$(mktemp -d "${TMPDIR:-/tmp}/vibehex-publish.XXXXXX")
trap 'git -C "$root" checkout -- Cargo.toml Cargo.lock; rm -rf "$work"' EXIT HUP INT TERM
/usr/bin/ditto -x -k "$zip" "$work/unpacked"
codesign --verify --deep --strict -R="$(cat "$root/app/update-requirement.txt")" "$work/unpacked/VibeHex.app"

downloads="https://github.com/$repo/releases/download/$tag"
manifest="$work/$channel.json"
python3 -I - "$version" "$channel" "$downloads" "$zip" "$dmg" > "$manifest" <<'PY'
import hashlib, json, os, sys, time
version, channel, downloads, zip_path, dmg_path = sys.argv[1:]
def target(path):
    digest = hashlib.sha256()
    with open(path, "rb") as file:
        for block in iter(lambda: file.read(1 << 20), b""):
            digest.update(block)
    return {"url": f"{downloads}/{os.path.basename(path)}", "size": os.path.getsize(path), "sha256": digest.hexdigest()}
print(json.dumps({
    "schema": 1,
    "app": "hex",
    "artifact": "desktop",
    "channel": channel,
    "sequence": int(time.time()),
    "published_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    "release": {
        "version": version,
        "rollout_percent": 100,
        "required": False,
        "targets": {"macos-arm64": target(zip_path), "macos-arm64-dmg": target(dmg_path)},
        "distributions": [{"kind": "direct"}],
    },
}, indent=2))
PY

commit=$(git -C "$root" rev-parse HEAD)
gh release create "$tag" -R "$repo" --target "$commit" --title "VibeHex $version" \
  --notes "VibeHex $version for macOS (Apple silicon). Download the DMG; installed copies update themselves." \
  --latest "$dmg" "$zip"
if ! gh release view updates -R "$repo" >/dev/null 2>&1; then
  gh release create updates -R "$repo" --target "$commit" --title "VibeHex update manifests" \
    --notes "What installed VibeHex apps read to find their update. Not a release to download." \
    --prerelease --latest=false
fi
gh release upload updates "$manifest" -R "$repo" --clobber
echo "Published VibeHex $version: https://github.com/$repo/releases/tag/$tag"
