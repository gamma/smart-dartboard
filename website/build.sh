#!/usr/bin/env bash
set -euo pipefail

script_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
repo_root="$(CDPATH= cd -- "$script_dir/.." && pwd)"
output="$repo_root/dist"

rm -rf -- "$output"
mkdir -p "$output/assets/modes" "$output/assets/neon"

cp "$script_dir/index.html" "$output/index.html"
cp "$script_dir/styles.css" "$output/styles.css"
cp "$script_dir/app.js" "$output/app.js"
cp "$script_dir/favicon.svg" "$output/favicon.svg"
cp "$script_dir/robots.txt" "$output/robots.txt"
cp -R "$script_dir/assets/screenshots" "$output/assets/screenshots"
cp "$repo_root"/web/static/assets/modes/*.webp "$output/assets/modes/"
cp "$repo_root"/web/static/assets/themes/neon/modes/*.webp "$output/assets/neon/"

touch "$output/.nojekyll"
echo "GitHub Pages site built at $output"
