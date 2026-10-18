---
name: release
description: How rok-ui versions and publishes.
---

# Releases

release-plz (`.github/workflows/release-plz.yml`, `release-plz.toml`) runs on every push to
`main` (and by hand with "Run workflow"):

1. The `release` job publishes every crate whose version is not on crates.io yet and tags
   `v<version>` with a GitHub release. So merging any PR that bumps the versions publishes
   them; it needs the `CARGO_REGISTRY_TOKEN` repository secret.
2. The `release-pr` job then opens or updates a release PR that bumps versions from
   Conventional Commits and adds a `CHANGELOG.md` section. Merging it publishes (step 1).

To release by hand instead: bump `[workspace.package] version` and the internal dependency
pins (`Cargo.toml`, `macros/`, `rok-ui-cli/`, `rok-ui-http/`), move `[Unreleased]` in
`CHANGELOG.md` to a dated section, update the install snippets, run `cargo publish --dry-run
--workspace --exclude rok-ui-example-file-routes`, and merge.

`semver_check` is off (cargo-semver-checks cannot build the old docs on the runner), so mark
breaking changes in commits with `!` or a `BREAKING CHANGE:` footer.

Publish order follows the dependency graph (release-plz works it out): `rok-ui-grammar` and
`rok-ui-bloc`, then `rok-ui-macros`, `rok-ui-build` and `rok-ui-http`, then `rok-ui`, then
`rok-ui-cli`. A new crate needs a `[[package]]` entry in `release-plz.toml`
(`changelog_update = false`, `git_release_enable = false`).

The `release-pr` job needs "Allow GitHub Actions to create and approve pull requests"
(Settings, Actions, General); without it the job fails with a 403 and leaves a
`release-plz-*` branch behind. The `release` job does not need it.

Before merging a release PR:

- Check the generated changelog reads well; edit the PR if needed.
- Make sure the README install snippets name the new minor version.
- Breaking changes need a `docs/migration/<version>.md` entry (enhance.md Part J).

Manual fallback: `cargo publish --workspace --exclude rok-ui-example-file-routes` (needs
`cargo login`).
