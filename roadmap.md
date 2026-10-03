# rok-ui roadmap

This tracks [enhance.md](enhance.md), the enhancement plan, item by item. Update it when an
item starts or lands (see AGENTS.md).

Status: **Done** (implemented and tested), **Partial** (usable, with gaps listed),
**Not started**, **Deferred** (waiting on a decision in enhance.md Part K, or out of scope for
now, with the reason).

Last updated: 2026-10-03.

## Summary

The plan spreads the work over releases 0.6 to 1.0. Most of it now lives on one branch and is
additive (no 0.5 API was removed), so it can ship as 0.6, or be split across releases as
Part I proposes.

| Release (Part I) | Theme | Status |
|---|---|---|
| 0.6 | Foundation: lints, CI, releases, agent docs | Partial: done except the macro grammar split, the doc-block conversion and the workspace split |
| 0.7 | Data: `Cx`, queries, mutations, procedures, Suspense | Partial: done except `#[shard]` |
| 0.8 | Router v2 | Partial: done except per-window history, scroll restoration and persistence |
| 0.9 | Forms | Done (the app form hook is replaced by bound field components) |
| 0.10 | Ecosystem: CLI, templates, stores, devtools, props | Not started |
| 0.11 | Remote procedures, remove legacy router | Deferred |
| 1.0 | API review, semver | Not started |

## Part H: engineering standards

| Item | Status | Notes |
|---|---|---|
| H.1 Workspace lints | Done | `[workspace.lints]`: clippy pedantic, `missing_docs`, `unsafe_code = "deny"`, `broken_intra_doc_links = "deny"`, `mod_module_files = "deny"`. Lints that fight UI code are allowed with a reason (pixel casts, hex colors, builder flags, `match_same_arms`, `needless_pass_by_value`). About 1,190 pedantic findings and 584 missing docs were fixed; `#[must_use]` on every builder. `clippy.toml` lists `TanStack` and `StyleX` as valid doc names. |
| H.1 rustfmt | Partial | `rustfmt.toml` uses stable options only; the nightly import options wait on decision K.9. |
| H.1 Barrel files | Done | `src/components.rs`, `src/theme.rs`; new modules follow the rule. |
| H.2 Grammar / macro split | Not started | New macros (`procedure`, `memoize`, `file_route`, `Search`, `FormValues`) are in their own modules with parse-then-expand functions, ready to move into a `rok-ui-grammar` crate. |
| H.3 Docs as source of truth | Partial | New guides (`query.md`, `forms.md`, router additions) are module docs with compiled doctests. The 0.5 guides and component docs still have about 104 `ignore` blocks to convert. |
| H.3 `llms.txt` | Done | Linked from README and AGENTS.md. |
| H.3 docs.rs `--cfg docsrs` | Done | `doc_cfg` enabled; CI builds docs on nightly with `-D warnings`. |
| H.4 CI | Done | Separate pedantic clippy job, `cargo hack --each-feature`, `cargo udeps`, nightly docs, PostgreSQL service for `tests/db.rs`, `--locked`. Template builds wait on the CLI (0.10). |
| H.4 Semantic PR titles | Done | `.github/workflows/semantic-pr.yml`. |
| H.4 release-plz | Done | `release-plz.toml`, `cliff.toml`, `.github/workflows/release-plz.yml` (needs the `CARGO_REGISTRY_TOKEN` secret). |
| H.5 Agent setup | Done | `AGENTS.md`, `CLAUDE.md`, `.agents/skills/{check,commit,pr,prose,style,macro,component,route,release}`, `.claude -> .agents`. |

## Part C: reactive state and data

| Item | Status | Notes |
|---|---|---|
| C.1 `Cx` | Done | `rok_ui::Cx` (window + app, derefs to `App`); `#[component]` accepts `cx: &mut Cx`. Scoped values (`cx.with`, `cx.keyed`) are not implemented. |
| Shared runtime | Done | `rok_ui::runtime` (feature `runtime`); `db` uses it. |
| C.2 Query cache | Done | `rok_ui::query`: `QueryOptions`, `query_key!`, shared in-flight fetches, stale and gc times, retries with backoff, `enabled`, initial / placeholder / previous data, refetch interval, prefix invalidation, imperative API, `queries(cx)` for inspection. Not done: `select`, refetch on window focus. |
| C.2 `db::use_query` migration | Done | `db::db_query` / `db_mutation`; the 0.5 hook is deprecated and `db::invalidate` forwards. |
| C.3 `Suspense`, `ErrorBoundary` | Done | Content returns `Result` and uses `?` on `use_suspense_query`. |
| C.3 `#[shard]` | Deferred | Splitting an `async fn` into a `Send` data phase and a UI phase needs a macro that rewrites the body; `Suspense` + queries cover the use case today. |
| C.4 Procedures and mutations | Done | `use_mutation` (pending / success / error, optimistic updates with rollback, supersession), `#[procedure]` with typed input, output and error, `Procedure::call`, `provide` / `TaskCx`. |
| C.4 Remote transport | Deferred | Waits on decision K.6 (axum via axum-rok-http, or local only). |
| C.5 `#[memoize]` | Partial | Per-app scope with `memo::invalidate`. Per-frame and per-navigation scopes are not implemented. |
| C.6 Signals | Done | Unchanged foundation (`rok_ui::state`). |
| C.7 `#[derive(Store)]`, persisted stores | Not started | Planned for 0.10. |
| C.8 Devtools | Not started | Planned for 0.10; `query::queries(cx)` already exposes the cache. |

## Part D: forms

| Item | Status | Notes |
|---|---|---|
| D.1 Defining a form | Done | `#[derive(FormValues)]` field constants, `FormOptions`, `FormValidators`, `FormError`. |
| D.2 Rendering fields | Done | `form.field(cx, path)` returns a `FieldApi`; render with bound components or read its state. Fine-grained `form.subscribe` is unnecessary in GPUI (the window re-renders); `form.state()` serves the same need. |
| D.3 Field and form state | Done | `FieldMeta` (touched, blurred, dirty, validating, `error_map`, `form_error_map`) and `FormState`. |
| D.4 Validation | Done | Mount / change / blur / submit events, errors per event, debounced async validators (latest run wins), form validators, `listen_to`, `Schema`, `GardeSchema` (feature `form-garde`), server errors from the submit handler. The `validator` crate adapter waits on decision K.5. |
| D.5 Array and nested fields | Done | Composable paths; `push`, `insert`, `remove`, `swap`, `move`, `replace` move each row's meta, input and focus with the row. |
| D.6 Bound inputs | Partial | `BoundInput` / `TextField` (String), `CheckboxField`, `SwitchField`, `SubmitButton`, `FormErrors`, and `field.change_handler()` for any control with `on_change`. Not yet: dedicated bound `Select`, `Combobox`, `Slider`, `DatePicker`, `InputOtp`, `Textarea`. |
| D.6 `create_form_hook!` | Deferred | The bound field components already make forms one line per field; an app-specific hook macro adds little in Rust. |
| D.7 Behavior | Partial | Enter submits from bound inputs; a failed submit focuses the first invalid field; `reset`, `reset_field`, `set_value`, `validate`; `is_dirty()` works with `router::use_blocker`. Not yet: scrolling the invalid field into view, persisted drafts (needs C.7). |

## Part E: routing v2

| Item | Status | Notes |
|---|---|---|
| E.1 File-based routing | Done | `rok-ui-build` (no GPUI dependency) with TanStack's conventions: `__root`, `__not_found`, `index`, `$param`, `$` splat, flat `a.b.rs`, `_pathless`, `(group)`, `-ignored`, `[escape]`; `file_route!` (`params`, `search`, `component` / `layout`, `before_load`, `loader`); `rok_ui::routes!()`; `write_to` for checked-in trees. Layout-ness is declared (`layout:`) rather than inferred from children. Route files are `include!`d, so they cannot declare child modules. |
| E.2 Code-based routing | Done | The 0.5 `Router` stays first-class and gained `route_to`, `guard`, `loader`; no `router::legacy` module is needed (decision K.4). A `route_tree!` macro was not added. |
| E.3 Typed links and navigation | Done | `typed_route!` (fields checked against the pattern at compile time), `Route::href` / `parse`, `Link::to`, `navigate_to`, `replace_to`, `use_params`; trybuild tests for mismatched fields and wrong types. |
| E.4 Typed search params | Done | `#[derive(Search)]` with defaults and renames, `use_search`, `update_search`, `replace_search`, `Link::search`. `loader_deps` is not implemented (loaders re-run per location, which includes the query string). |
| E.5 Guards and control flow | Partial | `Router::guard`, `RouteControl::{Redirect, NotFound}`, `before_load` on route files (applies to child routes). Guards are synchronous; async guards with memoized helpers are not implemented. App context goes through GPUI globals or `query::provide`. |
| E.6 Loaders and preloading | Partial | `Router::loader` / `loader_to`, `file_route! { loader }`, run once per location; `Link::preload(true)` on hover; `router::preload`. Pending UI comes from `Suspense`. Not yet: `pending_ms` / `pending_min_ms`, viewport preloading, cancelling loaders on navigation, `router::state` for progress bars. |
| E.7 Blocking | Done | `use_blocker` with `proceed` / `reset`. |
| E.7 Per-window history, scroll restoration, persistence, transitions | Not started | Per-window history needs window-aware location reads in every API that now takes `&mut App`; planned with a `RouterProvider`. |

## Part F: folder structure

| Item | Status | Notes |
|---|---|---|
| F.1 App structure | Partial | `examples/file_routes` follows it (`routes/`, `features/`, `build.rs`). `cargo rok-ui new` templates wait on 0.10. |
| F.2 Workspace split | Deferred | Waits on decision K.2. The workspace now has `rok-ui`, `rok-ui-macros`, `rok-ui-build` and the `examples/file_routes` crate. |

## Part G: component standards

| Item | Status | Notes |
|---|---|---|
| G.1 Typestate props | Not started | Required props are already constructor arguments (a missing one is a compile error). `#[default]` / `#[default(expr)]` aliases are planned for 0.10. |
| G.2 `#[key(..)]` in `view!` | Not started | Planned for 0.10. |
| G.3 Component contract | Done | `.agents/skills/component/SKILL.md`. |
| G.4 `cargo rok-ui add` | Not started | Planned for 0.10. |

## Acceptance criteria (Part I)

| Release | Criterion | Status |
|---|---|---|
| 0.6 | Pedantic clippy clean with `-D warnings` | Done |
| 0.6 | Zero `ignore` doc blocks | Not yet (new docs have none; about 104 remain) |
| 0.6 | `cargo hack` green | Done locally for the new features; CI runs every feature |
| 0.6 | release-plz opens the release PR | Configured; runs on `main` |
| 0.6 | Public API unchanged (`cargo public-api`) | Additive only; not checked with the tool |
| 0.7 | Two components reading one key trigger one fetch | Done (`tests/query.rs`) |
| 0.7 | Prefix invalidation refetches all matching queries | Done (`tests/query.rs`) |
| 0.7 | A failing boundary does not affect siblings | Done (`tests/query.rs`) |
| 0.7 | An optimistic update rolls back on error | Done (`tests/query.rs`) |
| 0.8 | Adding `routes/notes/$id.rs` makes `routes::NotesId` available | Done (`examples/file_routes/tests`) |
| 0.8 | Linking with a wrong param type fails to compile | Done (trybuild, `tests/ui-router`) |
| 0.8 | A loader is not re-run within `stale_time` | Done in spirit: loaders run once per location and prefetch through the query cache |
| 0.8 | Navigating away cancels its loader | Not yet |
| 0.8 | Two windows keep independent histories | Not yet |
| 0.8 | A guard redirect never renders the guarded component | Done (`tests/router.rs`, example tests) |
| 0.9 | Blur and submit errors are kept apart | Done (`tests/form.rs`) |
| 0.9 | A debounced async validator runs once per pause | Done (`tests/form.rs`) |
| 0.9 | Array operations keep each row's state | Done (`tests/form.rs`) |
| 0.9 | garde errors land on the right fields | Done (`tests/form.rs`) |
| 0.9 | A failed submit focuses the first invalid field | Done (`tests/form.rs`) |
| 0.10 | Templates build in CI; `cargo rok-ui add button` compiles; missing prop names the prop | Not started |

## Part K: decisions

These were left open in the plan. The work so far assumed the following; each can still be
changed.

| Decision | Assumed |
|---|---|
| K.1 Version cadence | Undecided; the work is additive, so it can ship as one 0.6 or be split. |
| K.2 Workspace split in 0.6 | Not done; waiting for a decision. |
| K.3 File routing naming | TanStack conventions (`$id.rs`), as proposed. |
| K.4 Router v2 compatibility | The 0.5 router stays as is; v2 features are additive. |
| K.5 Form validation adapters | `garde` only. |
| K.6 Remote procedures | Local only for now. |
| K.7 Signal primitives in rok-ui-hooks | Queries and forms do not need new hook primitives; nothing moved. |
| K.8 rok-db helpers | rok-db stays one data source (`db_query`, `db_mutation`). |
| K.9 Formatting | Stable rustfmt. |

## Migration notes

`docs/migration/0.6.md` covers moving from `db::use_query`, components taking `Cx`, typed
routes and file-based routes.
