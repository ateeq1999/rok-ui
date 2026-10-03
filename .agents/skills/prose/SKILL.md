---
name: prose
description: Where guides live and how rok-ui documentation is written.
---

# Documentation

Placement:

- Guides live in `docs/guide/<module>.md` and are the module docs on docs.rs:
  `#![doc = include_str!("../docs/guide/<module>.md")]` at the top of the module.
- Item docs explain what the item is for, its default, and a short example when the call is not
  obvious. Builders document the default: "Default: 16px."
- `README.md` stays an overview; it links to the guides.
- `llms.txt` is the digest for coding agents. Add a line when you add a public concept.

Writing rules:

- Plain English. Short sentences. Present tense, describing the current state (not history).
- ASCII only. Use `->` and `...`, not arrows and ellipsis characters.
- Code examples compile: they are doctests. Use `no_run` for examples that open windows, and
  hidden `# ` lines for setup. Do not add `ignore` blocks.
