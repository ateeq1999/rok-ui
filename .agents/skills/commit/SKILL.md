---
name: commit
description: Write a Conventional Commits message with a rok-ui scope.
---

# Commit messages

Format: `type(scope): summary` in the imperative, lower case, no trailing period, at most 72
characters. The body says why, wrapped at 72 columns.

Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `chore`, `ci`, `build`, `style`,
`revert`. Breaking changes add `!` (`feat(router)!: ...`) and a `BREAKING CHANGE:` footer.

Scopes: `router`, `query`, `forms`, `state`, `db`, `procedure`, `shard`, `macros`, `build`,
`cli`, `theme`, `bidi`, `fonts`, `motion`, `sx`, or a component name in kebab-case
(`date-picker`). Leave the scope out for changes that span many areas.

release-plz builds the changelog from these messages, so `feat` and `fix` summaries should read
well to users.
