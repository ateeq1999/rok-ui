#!/usr/bin/env bash
# Create an app from each `cargo rok-ui new` template against this checkout, build and test it,
# and check that `cargo rok-ui add button` produces a copy that compiles.
#
#     scripts/check-templates.sh
#
# The apps reuse this workspace's Cargo.lock and target directory.

set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/target}"

cargo build --quiet -p rok-ui-cli --manifest-path "$root/Cargo.toml"
cli="$CARGO_TARGET_DIR/debug/cargo-rok-ui"

for template in minimal full db; do
  echo "== template $template"
  (cd "$work" && "$cli" rok-ui new "app-$template" --template "$template" --rok-ui-path "$root")
  cp "$root/Cargo.lock" "$work/app-$template/"
  (cd "$work/app-$template" && cargo test --quiet)
done

echo "== cargo rok-ui add button"
cd "$work/app-minimal"
"$cli" rok-ui add button
cat >> src/main.rs <<'RUST'

mod components {
    pub mod ui;
}

#[allow(dead_code)]
fn vendored() -> impl IntoElement {
    components::ui::button::Button::new("vendored").outline().label("Mine now")
}
RUST
cargo check --quiet
echo "All templates build."
