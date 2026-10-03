---
name: pr
description: Describe a rok-ui pull request.
---

# Pull requests

- Title: a Conventional Commits message (the semantic-PR check enforces it).
- Describe the full diff, not only the last commit. Use the template in
  `.github/PULL_REQUEST_TEMPLATE.md`: what changes and why, screenshots for visual changes
  (light, dark and RTL when they differ), and the checklist.
- Say how you tested: which checks ran (see the `check` skill) and anything run by hand
  (`cargo run --example gallery`).
- If an AI assistant wrote part of the change, say so.
- Update `roadmap.md` when the PR completes an item from `enhance.md`.
