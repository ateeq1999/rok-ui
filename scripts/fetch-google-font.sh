#!/usr/bin/env bash
# Download static TrueType files for a Google Fonts family, plus its license.
#
#     scripts/fetch-google-font.sh "Cairo" 400,500,600,700 assets/fonts/cairo
#     scripts/fetch-google-font.sh "IBM Plex Sans Arabic" 400,700 my-app/fonts
#
# Files are named <Family>-<weight>.ttf (spaces removed). Register them at startup
# with `rok_ui::fonts::register_font_files` or embed them with `include_bytes!`
# and `rok_ui::fonts::register_fonts`.
#
# GPUI does not apply weight axes of variable fonts on every platform, so this
# asks Google for one static file per weight instead.

set -euo pipefail

if [ "$#" -lt 3 ]; then
  echo "usage: $0 <family> <weights, comma-separated> <output directory>" >&2
  exit 2
fi

family="$1"
weights="$2"
out="$3"
file_stem="${family// /}"
query="${family// /+}"
# The Google Fonts CSS API serves TrueType to clients without woff2 support.
user_agent="Mozilla/4.0"

mkdir -p "$out"
IFS=',' read -ra weight_list <<< "$weights"
for weight in "${weight_list[@]}"; do
  css=$(curl -fsS -A "$user_agent" "https://fonts.googleapis.com/css2?family=${query}:wght@${weight}")
  url=$(printf '%s' "$css" | grep -o 'https://[^)]*\.ttf' | head -n 1)
  if [ -z "$url" ]; then
    echo "No TrueType file for $family weight $weight" >&2
    exit 1
  fi
  curl -fsS -o "$out/${file_stem}-${weight}.ttf" "$url"
  echo "fetched $out/${file_stem}-${weight}.ttf"
done

# Google Fonts keeps each family's license next to its sources: ofl/, apache/ or ufl/.
slug=$(printf '%s' "$file_stem" | tr '[:upper:]' '[:lower:]')
for directory in ofl apache ufl; do
  for name in OFL.txt LICENSE.txt UFL.txt; do
    if curl -fsS -o "$out/LICENSE.txt" \
      "https://raw.githubusercontent.com/google/fonts/main/$directory/$slug/$name" 2>/dev/null; then
      echo "fetched $out/LICENSE.txt ($directory/$slug/$name)"
      exit 0
    fi
  done
done
echo "warning: could not find a license file for $family; add it before shipping the fonts" >&2
