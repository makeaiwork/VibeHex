#!/bin/sh
set -eu

# Publishes VibeHex <version> (X.Y.Z-vibe.N of the checked-out X.Y.Z) to the
# fork's GitHub releases, where every VibeHex checks for updates
# (docs/vibehex-releases.md), in two steps that can run apart:
#   prepare  scripts/release-app.sh builds, signs (HEX_TEAM_ID, default
#            3M7AWY4D32), notarizes (HEX_NOTARY_PROFILE), and staples the DMG
#            and update ZIP from a clean, pushed commit; the ZIP is checked
#            against the update requirement, and dist/ gets the manifest
#            (stable.json), the release notes (commits since the last
#            VibeHex release), every release's notes for the update sheet
#            (stable-notes.json), and the commit they belong to.
#   upload   creates release v<version> with both files, then replaces the
#            `updates` release's stable-notes.json and stable.json, which
#            running apps read. It refuses files that no longer match the
#            prepared manifest.
# With no step, both run. A version is never published twice.
#
#   HEX_NOTARY_PROFILE=<profile> scripts/publish-vibehex.sh [prepare|upload] 3.3.0-vibe.2
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
repo=${VIBEHEX_REPO:-makeaiwork/hex}
channel=stable
step=all
case "${1:-}" in
  prepare | upload) step=$1; shift ;;
esac
if [ "$#" -ne 1 ]; then
  echo "Usage: HEX_NOTARY_PROFILE=<profile> $0 [prepare|upload] <X.Y.Z-vibe.N>" >&2
  exit 1
fi
version=$1
current=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root/Cargo.toml" | head -1)
case "$version" in
  "$current"-vibe.[0-9]*) ;;
  *) echo "Publish $current-vibe.N, not $version." >&2; exit 1 ;;
esac
tag="v$version"
dist="$root/dist"
dmg="$dist/vibehex-$version-macos-arm64.dmg"
zip="$dist/vibehex-$version-macos-arm64.zip"
manifest="$dist/vibehex-$version-$channel.json"
notes="$dist/vibehex-$version-notes.md"
history="$dist/vibehex-$version-$channel-notes.json"
commit_file="$dist/vibehex-$version-commit"
if gh release view "$tag" -R "$repo" >/dev/null 2>&1; then
  echo "$tag is already published; pick the next vibe number." >&2
  exit 1
fi

prepare() {
  : "${HEX_NOTARY_PROFILE:?Set HEX_NOTARY_PROFILE to your notarytool keychain profile}"
  export HEX_TEAM_ID=${HEX_TEAM_ID:-3M7AWY4D32}
  commit=$(git -C "$root" rev-parse HEAD)
  # release-app.sh stamps the version into Cargo.toml and Cargo.lock; put them back.
  work=$(mktemp -d "${TMPDIR:-/tmp}/vibehex-publish.XXXXXX")
  trap 'git -C "$root" checkout -- Cargo.toml Cargo.lock; rm -rf "$work"' EXIT HUP INT TERM
  HEX_VERSION="$version" HEX_BUILD_CHANNEL="$channel" "$root/scripts/release-app.sh" prepare >/dev/null
  # The updater checks the same requirement before it swaps the bundle in.
  /usr/bin/ditto -x -k "$zip" "$work/unpacked"
  codesign --verify --deep --strict -R="$(cat "$root/app/update-requirement.txt")" "$work/unpacked/VibeHex.app"

  downloads="https://github.com/$repo/releases/download/$tag"
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

  # The notes list the commits since the last VibeHex release.
  git -C "$root" fetch -q --tags origin 2>/dev/null || true
  previous=$(git -C "$root" tag --list 'v*-vibe.*' --sort=-version:refname --merged "$commit" | head -1)
  {
    echo "VibeHex $version for macOS (Apple silicon). Download the DMG; installed copies update themselves."
    if [ -n "$previous" ]; then
      echo
      echo "Changes since $previous:"
      echo
      git -C "$root" log --no-merges --format='- %s' "$previous..$commit" | sed 's/ \[skip ci\]$//'
    fi
  } > "$notes"

  # Every release's changes, newest first, for the update sheet: an app
  # several releases behind lists each one it skipped. The first publish
  # rebuilds the earlier entries from their GitHub releases.
  : > "$work/items"
  if [ -n "$previous" ]; then
    git -C "$root" log --no-merges --format='%s' "$previous..$commit" \
      | sed 's/ \[skip ci\]$//' > "$work/items"
  fi
  if gh release download updates -R "$repo" -p "$channel-notes.json" -O "$work/earlier.json" 2>/dev/null; then
    earlier=notes
  else
    gh api "repos/$repo/releases?per_page=100" > "$work/earlier.json"
    earlier=releases
  fi
  python3 -I - "$version" "https://github.com/$repo/releases/tag/$tag" \
    "$work/items" "$earlier" "$work/earlier.json" > "$history" <<'PY'
import json, re, sys, time
version, url, items_path, earlier, earlier_path = sys.argv[1:]
def items(lines):
    return [{"title": line.strip()} for line in lines if line.strip()]
with open(earlier_path) as file:
    data = json.load(file)
if earlier == "notes":
    builds = data.get("builds", [])
else:
    builds = []
    for release in data:
        tag = release.get("tag_name", "")
        if not re.fullmatch(r"v.+-vibe\.[0-9]+", tag) or release.get("draft"):
            continue
        body = (release.get("body") or "").splitlines()
        builds.append({
            "version": tag[1:],
            "published_at": release.get("published_at"),
            "url": release.get("html_url"),
            "items": items(line[2:] for line in body if line.startswith("- ")),
        })
    builds.sort(key=lambda build: build["published_at"] or "", reverse=True)
with open(items_path) as file:
    this = {
        "version": version,
        "published_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "url": url,
        "items": items(file),
    }
builds = [this] + [build for build in builds if build.get("version") != version]
print(json.dumps({"schema": 1, "builds": builds[:100]}, indent=2, ensure_ascii=False))
PY
  printf '%s\n' "$commit" > "$commit_file"
  echo "Prepared VibeHex $version from $commit:"
  echo "  $dmg"
  echo "  $zip"
  echo "  $manifest"
  echo "  $notes"
  echo "  $history"
}

upload() {
  for file in "$dmg" "$zip" "$manifest" "$notes" "$history" "$commit_file"; do
    [ -f "$file" ] || { echo "Missing $file; run prepare $version first." >&2; exit 1; }
  done
  python3 -I - "$manifest" "$zip" "$dmg" <<'PY'
import hashlib, json, os, sys
manifest, *files = sys.argv[1:]
targets = json.load(open(manifest))["release"]["targets"].values()
expected = {os.path.basename(t["url"]): (t["size"], t["sha256"]) for t in targets}
for path in files:
    digest = hashlib.sha256(open(path, "rb").read()).hexdigest()
    if expected.get(os.path.basename(path)) != (os.path.getsize(path), digest):
        sys.exit(f"{path} no longer matches the prepared manifest")
PY
  commit=$(cat "$commit_file")
  gh release create "$tag" -R "$repo" --target "$commit" --title "VibeHex $version" \
    --notes-file "$notes" --latest "$dmg" "$zip"
  if ! gh release view updates -R "$repo" >/dev/null 2>&1; then
    gh release create updates -R "$repo" --target "$commit" --title "VibeHex update manifests" \
      --notes "What installed VibeHex apps read to find their update. Not a release to download." \
      --prerelease --latest=false
  fi
  # Running apps look for these exact names; the notes go first, so an app
  # that finds the new manifest finds its notes too.
  cp "$history" "$dist/$channel-notes.json"
  gh release upload updates "$dist/$channel-notes.json" -R "$repo" --clobber
  cp "$manifest" "$dist/$channel.json"
  gh release upload updates "$dist/$channel.json" -R "$repo" --clobber
  echo "Published VibeHex $version: https://github.com/$repo/releases/tag/$tag"
}

case "$step" in
  prepare) prepare ;;
  upload) upload ;;
  all) prepare && upload ;;
esac
