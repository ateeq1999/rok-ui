#!/usr/bin/env bash
# Build the library with each Cargo feature on its own (and no default features),
# so a component never depends on another one its feature doesn't enable.
#
#     scripts/check-features.sh            # every feature
#     scripts/check-features.sh button     # just the ones named
#
# Warnings count as failures: an unused import under one feature is a bug.

set -uo pipefail
cd "$(dirname "$0")/.."

if [ "$#" -gt 0 ]; then
  features=("$@")
else
  # Every key in the [features] table except `default`.
  mapfile -t features < <(awk '/^\[features\]/ { in_features = 1; next }
    /^\[/ { in_features = 0 }
    in_features && /^[a-z0-9-]+ *=/ && $1 != "default" { print $1 }' Cargo.toml)
fi

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target/feature-check}"
failed=()
for feature in "${features[@]}"; do
  output=$(cargo check --lib --no-default-features --features "$feature" --color never 2>&1)
  status=$?
  # Ignore the future-incompatibility notice about third-party crates.
  problems=$(echo "$output" | grep -E '^(error|warning)' | grep -v 'future version of Rust')
  if [ "$status" -ne 0 ] || [ -n "$problems" ]; then
    echo "FAIL $feature"
    echo "$output" | grep -E '^(error|warning)' -A6 | grep -v 'future version of Rust' | head -20
    failed+=("$feature")
  else
    echo "ok   $feature"
  fi
done

echo
if [ "${#failed[@]}" -gt 0 ]; then
  echo "${#failed[@]} of ${#features[@]} features failed: ${failed[*]}"
  exit 1
fi
echo "All ${#features[@]} features build on their own."
