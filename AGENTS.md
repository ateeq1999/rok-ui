# Agent guide for rok-ui

rok-ui is a shadcn/ui-style component system and app framework for GPUI desktop apps.

Start here:

- `llms.txt`: a dense digest of the public API and the conventions every API follows.
- `.agents/skills/`: task checklists. Read the matching skill before you start a task:

| Task | Skill |
|---|---|
| Run the checks CI runs | `.agents/skills/check/SKILL.md` |
| Write a commit message | `.agents/skills/commit/SKILL.md` |
| Open a pull request | `.agents/skills/pr/SKILL.md` |
| Write or edit docs and guides | `.agents/skills/prose/SKILL.md` |
| Code style, docs and tests | `.agents/skills/style/SKILL.md` |
| Change or add a proc macro | `.agents/skills/macro/SKILL.md` |
| Add or change a component | `.agents/skills/component/SKILL.md` |
| Add routes or change the router | `.agents/skills/route/SKILL.md` |
| Use rok-db or change the `db` feature | `.agents/skills/db/SKILL.md` |
| Build app features (BLoC), use or change `cargo rok-ui generate` | `.agents/skills/bloc/SKILL.md` |
| Call APIs with `rok_ui::http`, API errors on forms | `.agents/skills/http/SKILL.md` |
| Review a change for code quality | `.agents/skills/quality/SKILL.md` |
| Cut a release | `.agents/skills/release/SKILL.md` |

Ground rules:

- `enhance.md` is the plan; `roadmap.md` tracks what is done. Update `roadmap.md` when you
  finish or start an item from the plan.
- Every public item has a doc comment. `cargo clippy --workspace --all-targets --all-features`
  must be warning-free (CI denies warnings, and the workspace enables clippy pedantic).
- No `mod.rs`: a module `foo` with children is `foo.rs` next to a `foo/` directory.
- ASCII only in code and docs, except test data and examples that exercise other scripts.
- On Linux, GPUI needs `libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libvulkan-dev
  libfontconfig-dev libx11-xcb-dev libxcb1-dev` to link tests.
