#!/bin/sh
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$root/scripts/macos-signing.sh"
# HEX_TEST_BUILD=1 builds the CI test app: ad-hoc signed, under its own
# name and bundle ID so it installs beside the released Hex and never
# touches its permissions; it never checks for updates.
# HEX_LOCAL_BUILD=1 marks a build made from a checkout for its owner's own
# machine (Developer ID signed as usual): it shows "Local build · updates off"
# with its commit and never checks for or offers updates.
# HEX_RELEASE_UNSIGNED=1 builds the release app (VibeHex, ai.makeaiwork.vibehex) ad-hoc
# signed: the Release workflow's build job, which holds no credentials. Its
# sign-macos job signs it with the Developer ID (.github/release/sign-macos.sh).
if [ "${HEX_RELEASE_UNSIGNED:-}" = 1 ]; then
  team_id=
  identity=-
  app_name=VibeHex
  bundle_id=ai.makeaiwork.vibehex
elif [ "${HEX_TEST_BUILD:-}" = 1 ]; then
  team_id=
  identity=-
  app_name="Hex Test"
  bundle_id=anomaly.hex.test
else
  team_id=${HEX_TEAM_ID:?Set HEX_TEAM_ID to the Apple Developer signing team}
  identity=$(hex_codesign_identity "$team_id")
  # VIBEHEX_SIDE_BY_SIDE=1 builds "VibeHex 2" (ai.makeaiwork.vibehex2), a
  # copy to try a change beside the installed VibeHex. It shares VibeHex's
  # data folder, so only one of them runs at a time.
  if [ "${VIBEHEX_SIDE_BY_SIDE:-}" = 1 ]; then
    app_name="VibeHex 2"
    bundle_id=ai.makeaiwork.vibehex2
  else
    app_name=VibeHex
    bundle_id=ai.makeaiwork.vibehex
  fi
fi
# Every build carries the `capture` experiment (dormant until enabled), with
# Scrawl's `scrawl-helper` beside `hex`, where the engine looks for it.
# crates/capture/build.rs compiles the helper from crates/capture/helper into
# target/release.
icon_output="$root/target/AppIcon.assets"
icon_info="$root/target/AppIcon-info.plist"
version=$(hex_version "$root")
build_number=$(hex_build_number "$version")
bundle_name="$app_name.app"
executable_name=hex

bundle="$root/target/app/$bundle_name"
executable="$bundle/Contents/MacOS/$executable_name"

sign() {
  if [ "$identity" = - ]; then
    codesign --force --sign - "$@"
  else
    codesign --force --timestamp --options runtime --sign "$identity" "$@"
  fi
}

export MACOSX_DEPLOYMENT_TARGET=15.0
# Cargo may run through a wrapper that does not forward the environment (Kit's
# HiveMind-backed cargo), so build-time variables also travel as --config.
env_args=()
if [ -z "${HEX_COMMIT:-}" ] && { [ "${HEX_LOCAL_BUILD:-}" = 1 ] || [ "${HEX_TEST_BUILD:-}" = 1 ]; }; then
  HEX_COMMIT=$(git -C "$root" rev-parse --short=7 HEAD 2>/dev/null || true)
fi
for name in HEX_LOCAL_BUILD HEX_TEST_BUILD HEX_COMMIT HEX_BUILD_CHANNEL; do
  value=${!name:-}
  if [ -n "$value" ]; then
    case "$value" in
      *\'*) echo "$name must not contain a single quote" >&2; exit 1 ;;
    esac
    env_args+=(--config "env.$name='$value'")
  fi
done
# The helper's embedded Info.plist names Hex, not Scrawl.
SCRAWL_HELPER_PLIST="$root/packaging/macos/scrawl-helper-Info.plist" \
  cargo build --locked --release ${env_args[@]+"${env_args[@]}"} --manifest-path "$root/Cargo.toml"
rm -rf "$bundle"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources" "$bundle/Contents/Library" "$bundle/Contents/Frameworks"
cp "$root/target/release/hex" "$executable"
cp "$root/target/release/scrawl-helper" "$bundle/Contents/MacOS/scrawl-helper"
cp "$root/app/Info.plist" "$bundle/Contents/Info.plist"
cp "$root/LICENSE" "$root/THIRD_PARTY_NOTICES.md" "$bundle/Contents/Resources/"

# The built-in cleanup engine (src/dictation/cleanup_server/engine.rs):
# llama.cpp's own macOS build of `llama-server`, pinned by build and SHA-256,
# beside `hex`, and the libraries it loads in Contents/Frameworks.
llama_build=b11960
llama_sha256=58e5a5e1d7c2bf66cedd9c29e0323f3e570660a70a1e2156d32eed08f4cc1e3c
llama_archive="$root/target/llama.cpp/llama-$llama_build-bin-macos-arm64.tar.gz"
if [ ! -f "$llama_archive" ] || ! echo "$llama_sha256  $llama_archive" | shasum -a 256 -c - >/dev/null 2>&1; then
  mkdir -p "$(dirname "$llama_archive")"
  curl -fsSL --retry 3 -o "$llama_archive.partial" \
    "https://github.com/ggml-org/llama.cpp/releases/download/$llama_build/llama-$llama_build-bin-macos-arm64.tar.gz"
  mv "$llama_archive.partial" "$llama_archive"
fi
echo "$llama_sha256  $llama_archive" | shasum -a 256 -c - >/dev/null || {
  echo "llama.cpp $llama_build doesn't match its pinned SHA-256" >&2
  exit 1
}
llama_unpacked=$(mktemp -d "${TMPDIR:-/tmp}/hex-llama.XXXXXX")
tar -xzf "$llama_archive" -C "$llama_unpacked"
llama_dir="$llama_unpacked/llama-$llama_build"
llama_libraries="libllama-server-impl.dylib libllama-common.0.dylib libmtmd.0.dylib libllama.0.dylib
  libggml.0.dylib libggml-base.0.dylib libggml-cpu.0.dylib libggml-blas.0.dylib libggml-metal.0.dylib
  libggml-rpc.0.dylib"
cp "$llama_dir/llama-server" "$bundle/Contents/MacOS/llama-server"
for library in $llama_libraries; do
  # The archive's .0 names are symlinks; the bundle holds the files under them.
  cp -L "$llama_dir/$library" "$bundle/Contents/Frameworks/$library"
done
install_name_tool -add_rpath @executable_path/../Frameworks "$bundle/Contents/MacOS/llama-server" 2>&1 | grep -v "will invalidate the code signature" >&2 || true
cp "$llama_dir/LICENSE" "$bundle/Contents/Resources/llama.cpp-LICENSE"
rm -rf "$llama_unpacked"
HEX_VERSION="$version" HEX_BUILD_NUMBER="$build_number" HEX_TEAM_ID="$team_id" HEX_CODESIGN_IDENTITY="${identity#-}" HEX_SKIP_INSTALL_SCRIPT=1 \
  "$root/scripts/build-hex-microphone-driver.sh" "$bundle/Contents/Library" >&2
/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $version" "$bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $build_number" "$bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleIdentifier $bundle_id" "$bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleExecutable $executable_name" "$bundle/Contents/Info.plist"
plutil -replace CFBundleName -string "$app_name" "$bundle/Contents/Info.plist"
plutil -replace CFBundleDisplayName -string "$app_name" "$bundle/Contents/Info.plist"
# Links into the playground (src/rune_ui/story_link.rs); a test build's own
# scheme never opens the installed Hex.
url_scheme=hex
[ "${HEX_TEST_BUILD:-}" = 1 ] && url_scheme=hex-test
/usr/libexec/PlistBuddy -c "Set :CFBundleURLTypes:0:CFBundleURLName $bundle_id" "$bundle/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Set :CFBundleURLTypes:0:CFBundleURLSchemes:0 $url_scheme" "$bundle/Contents/Info.plist"
# The helper mirrors a cabled iPhone as a camera; macOS asks on Hex's behalf.
plutil -replace NSCameraUsageDescription -string "VibeHex mirrors and records a cabled iPhone's screen for a capture." "$bundle/Contents/Info.plist"
rm -rf "$icon_output"
mkdir -p "$icon_output"
xcrun actool "$root/app/AppIcon.icon" \
  --compile "$icon_output" \
  --platform macosx \
  --minimum-deployment-target 15.0 \
  --app-icon AppIcon \
  --standalone-icon-behavior all \
  --output-partial-info-plist "$icon_info" \
  --output-format human-readable-text \
  --warnings \
  --notices >&2
cp "$icon_output/AppIcon.icns" "$icon_output/Assets.car" "$bundle/Contents/Resources/"

sign "$bundle/Contents/Library/HexMicrophone.driver"
# Hex runs it only to decode the capture's movie, which needs no
# entitlement; anything it touches is attributed to Hex, its responsible
# process.
sign --identifier "$bundle_id.scrawl-helper" "$bundle/Contents/MacOS/scrawl-helper"
for library in $llama_libraries; do
  sign "$bundle/Contents/Frameworks/$library"
done
sign --identifier "$bundle_id.llama-server" "$bundle/Contents/MacOS/llama-server"
sign --entitlements "$root/.github/release/hex.entitlements" "$bundle"
codesign --verify --deep --strict --verbose=2 "$bundle"
echo "$bundle"
