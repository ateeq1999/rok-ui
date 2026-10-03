# Roadmap

Progress against the plan in [enhance.md](enhance.md) (Part I). Update this file in the pull
request that starts or finishes an item, and link the pull request.

Status: **Done**, **Partial** (what is missing is listed), **Not started**.

## 0.6 Foundation (released 2026-10-03)

| Item | Status | Notes |
|---|---|---|
| H.1 Lints | Done | `[workspace.lints]`: clippy pedantic, `missing_docs`, `unsafe_code = "deny"`; CI denies warnings |
| H.1 Formatting | Done (stable) | Stable `rustfmt.toml`; the nightly import-grouping options wait on decision K.9 |
| H.2 Macro split (`rok-ui-grammar`) | Not started | Macros still live in `rok-ui-macros` only |
| H.3 `ignore` doc blocks -> compiled doctests | Not started | 121 `ignore` blocks remain in `src/`, `docs/guide/` and `README.md` |
| H.3 `llms.txt` | Done | Linked from the README and `AGENTS.md` |
| H.4 CI and release-plz | Done | fmt, clippy, 3-OS tests with Postgres, `cargo hack`, udeps, MSRV, docs, semantic PR titles, cargo-deny, release-plz |
| H.5 Coding-agent setup | Done | `AGENTS.md`, `CLAUDE.md`, `.agents/skills/` (`.claude` symlink) |
| Barrel files (no `mod.rs`) | Done | |
| Shared `rok_ui::runtime` | Done | |
| F.2 Workspace split | Not started | Workspace members are the facade and `macros` |
| Acceptance: `cargo public-api` diff | Not started | No public-API check in CI yet |

## 0.7 Data

Most of this shipped early, in 0.6.

| Item | Status | Notes |
|---|---|---|
| C.1 `Cx` | Done | |
| C.2 Query cache and `QueryOptions` | Done | |
| C.3 `Suspense` / `ErrorBoundary` | Done | |
| C.3 `#[shard]` | Not started | |
| C.4 Procedures and mutations (local) | Done | `#[procedure]`, `use_procedure`, `use_mutation` with optimistic updates |
| C.5 `#[memoize]` | Done | |
| `db::use_query` shim | Done | Deprecated in 0.6, removed in 0.8 |

## 0.8 Router v2

| Item | Status |
|---|---|
| E.1 File-based routing (`rok-ui-build`) | Not started |
| E.2 Code-based route tree | Not started |
| E.3 Typed links | Not started |
| E.4 Typed search params | Not started |
| E.5 Guards | Not started |
| E.6 Loaders and preloading | Not started |
| E.7 Per-window history and blocking | Not started |
| `router::legacy` for the 0.5 API | Not started |

## 0.9 Forms

| Item | Status |
|---|---|
| Part D: forms runtime, validators, arrays, schema adapters, bound inputs, `create_form_hook!` | Not started |

## 0.10 Ecosystem

| Item | Status | Notes |
|---|---|---|
| F.1 `cargo rok-ui new` templates | Not started | |
| G.4 `cargo rok-ui add` | Not started | |
| C.7 Field stores and persistence | Not started | |
| C.8 Devtools | Partial | `query::queries()` returns `QueryInfo` for every cached query; there is no devtools UI |
| G.1 Typestate props | Not started | |
| G.2 `#[key(..)]` | Not started | |

## 0.11 Reach and 1.0

| Item | Status |
|---|---|
| C.4 Remote procedures (axum) | Not started |
| `router::legacy` removed | Not started |
| 1.0 API review and semver | Not started |

## Open decisions

The questions in [enhance.md Part K](enhance.md#part-k-decisions-needed) are still open, except
K.9 (formatting), which the repository settles on stable rustfmt for now.
