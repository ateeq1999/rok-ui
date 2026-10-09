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

for template in minimal full db bloc bloc-http; do
  echo "== template $template"
  (cd "$work" && "$cli" rok-ui new "app-$template" --template "$template" --rok-ui-path "$root")
  cp "$root/Cargo.lock" "$work/app-$template/"
  (cd "$work/app-$template" && cargo test --quiet)
done

echo "== bloc template: clippy pedantic, then more generated features"
cd "$work/app-bloc"
cargo clippy --quiet --all-targets -- -D warnings
"$cli" rok-ui g feature tasks -j @"$root/rok-ui-cli/fixtures/tasks.json"
"$cli" rok-ui g bloc Counter --event Incremented --event Reset --status initial,success,failure --view
# Generating again changes nothing.
"$cli" rok-ui g feature notes -j @"$root/rok-ui-cli/fixtures/notes.json" | grep -v unchanged && exit 1
cargo clippy --quiet --all-targets -- -D warnings
cargo test --quiet

echo "== bloc-http template: clippy pedantic, then an API from flags only"
cd "$work/app-bloc-http"
cargo clippy --quiet --all-targets -- -D warnings
"$cli" rok-ui g api todos --dto "TodoDto:id:TodoId,title:String,done:bool" \
  --endpoint "list_todos:GET:/todos:Vec<TodoDto>" \
  --endpoint "delete_todo:DELETE:/todos/{id:TodoId}:()"
cargo clippy --quiet --all-targets -- -D warnings
cargo test --quiet

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
