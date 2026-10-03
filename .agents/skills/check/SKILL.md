---
name: check
description: Run the same checks CI runs on rok-ui before committing or opening a pull request.
---

# Local checks

Run from the repository root, in this order (fast to slow):

```sh
cargo fmt --all --check
RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-features
RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +nightly doc --workspace --no-deps --all-features
cargo deny --all-features check     # licenses and advisories (deny.toml)
```

Feature isolation (each feature builds on its own). CI runs `cargo hack`; locally, check the
features you touched:

```sh
scripts/check-features.sh button dialog          # or: cargo hack clippy --lib --each-feature --no-dev-deps -p rok-ui
```

Notes:

- `tests/compile_errors.rs` (trybuild) is slow on a cold cache. If a macro's error message
  changes on purpose, regenerate with `TRYBUILD=overwrite cargo test --test compile_errors`
  and review the `.stderr` diff.
- `tests/db.rs` skips unless `ROK_UI_TEST_DATABASE_URL` points at a PostgreSQL server.
- Then review the diff with the `quality` skill.
- A clippy pedantic warning is fixed, not silenced. Allow a lint locally only with a comment
  saying why the code is clearer as it is.
