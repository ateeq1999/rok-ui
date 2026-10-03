---
name: release
description: How rok-ui versions and publishes.
---

# Releases

release-plz (`.github/workflows/release-plz.yml`, `release-plz.toml`) runs on every push to
`main`:

1. It opens or updates a release PR that bumps versions from Conventional Commits and adds a
   `CHANGELOG.md` section.
2. Merging that PR publishes to crates.io and tags the release.

Publish order follows the dependency graph: `rok-ui-macros`, then `rok-ui` (later: grammar,
macros, sub-crates, then the facade).

Before merging a release PR:

- Check the generated changelog reads well; edit the PR if needed.
- Make sure the README install snippets name the new minor version.
- Breaking changes need a `docs/migration/<version>.md` entry (enhance.md Part J).

Manual fallback: `cargo publish -p rok-ui-macros && cargo publish -p rok-ui`.
