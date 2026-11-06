#!/bin/bash
# Fallback DMG packaging without Finder automation. Build the .app first.
set -euo pipefail
cd "$(dirname "$0")/.."
app_path="src-tauri/target/release/bundle/macos/Daymark.app"
version="$(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' "$app_path/Contents/Info.plist")"
output_path="${1:-packages/Daymark_${version}_$(uname -m).dmg}"
if [[ ! -d "$app_path" ]]; then
  echo 'Build first: pnpm tauri build --bundles app' >&2
  exit 1
fi
if [[ -e "$output_path" ]]; then
  echo "Output already exists: $output_path" >&2
  exit 1
fi
mkdir -p "$(dirname "$output_path")"
staging_dir="$(mktemp -d "${TMPDIR:-/tmp}/daymark-dmg.XXXXXX")"
trap 'rm -rf "$staging_dir"' EXIT
ditto "$app_path" "$staging_dir/Daymark.app"
ln -s /Applications "$staging_dir/Applications"
hdiutil create -volname Daymark -srcfolder "$staging_dir" -format UDZO "$output_path"
hdiutil verify "$output_path"
