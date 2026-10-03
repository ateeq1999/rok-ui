# rok-ui roadmap

This tracks [enhance.md](enhance.md), the enhancement plan, item by item. Update it when an
item starts or lands (see AGENTS.md).

Status: **Done** (implemented and tested), **Partial** (usable, with the gaps listed),
**Not started**, **Deferred** (waiting on a decision in enhance.md Part K, or set aside, with
the reason).

Last updated: 2026-10-03.

## Summary

The plan spreads the work over releases 0.6 to 1.0. 0.6.0 (released 2026-10-03) shipped the
foundation and most of the data layer; the rest is additive: no 0.5 API was removed (one
function is deprecated), so it can ship as one release or be split as Part I proposes.

| Release (Part I) | Theme | Status |
|---|---|---|
| 0.6 | Foundation: lints, docs, CI, releases, agent docs, macro split | Done, except the workspace split (decision K.2) |
| 0.7 | Data: `Cx`, queries, mutations, procedures, Suspense | Done, except `#[shard]` and remote procedures |
| 0.8 | Router v2 | Done, except scroll restoration, transitions and pending-UI timing |
| 0.9 | Forms | Done (the app form hook is replaced by bound field components) |
| 0.10 | Ecosystem: CLI, templates, stores, devtools, props, keys | Done |
| 0.11 | Remote procedures, remove the legacy router | Deferred (K.6; no legacy router to remove, see K.4) |
| 1.0 | API review, semver | Not started |

What exists now, by crate:

| Crate | What it is |
|---|---|
| `rok-ui` | Components plus `Cx`, `runtime`, `query`, `router` (typed, file-based), `form`, `state` (with `#[derive(Store)]`), `persist`, `devtools`, `Keyed` |
| `rok-ui-macros` | Proc-macro entry points only |
| `rok-ui-grammar` | Parsing and code generation for every macro, with expansion tests |
| `rok-ui-build` | The file-based route generator for `build.rs` |
| `rok-ui-cli` | `cargo rok-ui new`, `add`, `routes` |
| `examples/file_routes` | A complete file-routed app with integration tests (not published) |

## Part H: engineering standards

| Item | Status | Notes |
|---|---|---|
| H.1 Workspace lints | Done | `[workspace.lints]`: clippy pedantic, `missing_docs`, `unsafe_code = "deny"`, `broken_intra_doc_links = "deny"`, `mod_module_files = "deny"`. Lints that fight UI code are allowed with a reason (pixel casts, hex colors, builder flags, `match_same_arms`, `needless_pass_by_value`). About 1,190 pedantic findings and 584 missing docs were fixed; `#[must_use]` on every builder. `clippy.toml` lists `TanStack` and `StyleX` as valid doc names. |
| H.1 rustfmt | Partial | `rustfmt.toml` uses stable options only; the nightly import options wait on decision K.9. |
| H.1 Barrel files | Done | `src/components.rs`, `src/theme.rs`, and every new module. |
| H.2 Grammar / macro split | Done | `rok-ui-grammar` holds one module per macro; `rok-ui-macros` only forwards tokens. `rok-ui-grammar/tests/expansions.rs` checks that every expansion parses as Rust, is deterministic, and reports mistakes clearly. |
| H.3 Docs as source of truth | Done | Guides in `docs/guide/` are module docs. All 126 former `ignore` examples compile (155 doctests in `rok-ui`, 7 in `rok-ui-macros`, none ignored). |
| H.3 `llms.txt` | Done | Linked from README and AGENTS.md. |
| H.3 docs.rs `--cfg docsrs` | Done | `doc_cfg` enabled; CI builds docs on nightly with `-D warnings`. |
| H.4 CI | Done | Pedantic clippy job, `cargo hack --each-feature`, `cargo udeps`, nightly docs, PostgreSQL service for `tests/db.rs`, `--locked`, a templates job (`scripts/check-templates.sh`), and `cargo deny` (`deny.toml`). |
| H.4 Semantic PR titles | Done | `.github/workflows/semantic-pr.yml`. |
| H.4 release-plz | Done | `release-plz.toml` (publish order: grammar, macros and build, rok-ui, cli), `cliff.toml`, `.github/workflows/release-plz.yml`. Needs the `CARGO_REGISTRY_TOKEN` secret. |
| H.5 Agent setup | Done | `AGENTS.md`, `CLAUDE.md`, `.agents/skills/{check,commit,pr,prose,style,macro,component,route,db,quality,release}`, `.claude -> .agents`. |

## Part C: reactive state and data

| Item | Status | Notes |
|---|---|---|
| C.1 `Cx` | Partial | `rok_ui::Cx` (window and app, derefs to `App`, `get` / `try_get` / `use_state`); `#[component]` accepts `cx: &mut Cx`. Scoped values (`cx.with`, `cx.keyed`) are not implemented; `Keyed` (G.2) covers stable identity. |
| Shared runtime | Done | `rok_ui::runtime` (feature `runtime`); `db` and `query` use it. |
| C.2 Query cache | Done | `QueryOptions`, `query_key!`, shared in-flight fetches, stale and gc times, retries with backoff, `enabled`, initial / placeholder / previous data, refetch interval, refetch on window focus, prefix invalidation, the imperative API, `queries(cx)`, and `use_query_select` (TanStack's `select`, derived once per data change). |
| C.2 `db::use_query` migration | Done | `db::db_query` / `db_mutation`; the 0.5 hook is deprecated and `db::invalidate` forwards. |
| rok-db 0.3 | Done | The `db` feature depends on rok-db 0.3. `db::watch_changes::<M>(cx)` uses its change feeds to invalidate `M::TABLE` when rows change in any process (`tests/db.rs`, against PostgreSQL). |
| C.3 `Suspense`, `ErrorBoundary` | Done | Content returns `Result` and uses `?` on `use_suspense_query`. |
| C.3 `#[shard]` | Deferred | An `async fn` that returns elements needs a macro that splits the body into a `Send` data phase and a UI phase; `Suspense` plus queries cover the use case. |
| C.4 Procedures and mutations | Done | `use_mutation` (pending / success / error, optimistic updates with rollback, supersession), `#[procedure]` with typed input, output and error, `Procedure::call`, `provide` / `TaskCx`. |
| C.4 Remote transport | Deferred | Waits on decision K.6. |
| C.5 `#[memoize]` | Partial | App-wide scope with `memo::invalidate`; per-frame and per-navigation scopes are not implemented. |
| C.6 Signals | Done | Unchanged foundation (`rok_ui::state`). |
| C.7 Stores | Done | `#[derive(Store)]` (a signal per field); `persist` feature: `persisted_store` with versioned JSON, migrations, debounced atomic writes. |
| C.8 Devtools | Partial | `devtools` feature: an overlay (Ctrl-Shift-D) with the router history and the query cache. Mutations, forms and the signal graph are not shown yet. |

## Part D: forms

| Item | Status | Notes |
|---|---|---|
| D.1 Defining a form | Done | `#[derive(FormValues)]` field constants, `FormOptions`, `FormValidators`, `FormError`. |
| D.2 Rendering fields | Done | `form.field(cx, path)` returns a `FieldApi`. GPUI re-renders the window, so `form.state()` replaces fine-grained `subscribe`. |
| D.3 Field and form state | Done | `FieldMeta` (touched, blurred, dirty, validating, `error_map`, `form_error_map`) and `FormState`. |
| D.4 Validation | Done | Mount / change / blur / submit events, errors per event, debounced async validators (latest run wins), form validators, `listen_to`, `Schema`, `GardeSchema` (`form-garde`), server errors from the submit handler. The `validator` adapter waits on decision K.5. |
| D.5 Array and nested fields | Done | Composable paths; list operations move each row's state, input and focus with the row. |
| D.6 Bound inputs | Partial | `BoundInput` / `TextField`, `TextareaField`, `CheckboxField`, `SwitchField`, `SelectField`, `RadioGroupField`, `SliderField`, `SubmitButton`, `FormErrors`, and `field.change_handler()` for any other control whose `on_change` passes the value. No dedicated bound `Combobox`, `DatePicker` or `InputOtp` yet. |
| D.6 `create_form_hook!` | Deferred | The bound field components already make forms one line per field. |
| D.7 Behavior | Partial | Enter submits, a failed submit focuses the first invalid field, `reset` / `reset_field` / `set_value` / `validate`, `is_dirty()` with `use_blocker`. Not yet: scrolling the invalid field into view, persisted drafts. |

## Part E: routing v2

| Item | Status | Notes |
|---|---|---|
| E.1 File-based routing | Done | `rok-ui-build` with TanStack's conventions (`__root`, `__not_found`, `index`, `$param`, `$` splat, flat `a.b.rs`, `_pathless`, `(group)`, `-ignored`, `[escape]`); `file_route!` (`params`, `search`, `component` / `layout`, `before_load`, `loader`); `rok_ui::routes!()`; `write_to` and `cargo rok-ui routes` for checked-in trees. Layouts are declared with `layout:`. Route files are `include!`d, so they cannot declare child modules. |
| E.2 Code-based routing | Done | The 0.5 `Router` stays and gained `route_to`, `guard`, `loader`. No `route_tree!` macro. |
| E.3 Typed links and navigation | Done | `typed_route!` (checked at compile time), `Route::href` / `parse`, `Link::to`, `navigate_to`, `replace_to`, `use_params`. |
| E.4 Typed search params | Done | `#[derive(Search)]`, `use_search`, `update_search`, `replace_search`, `Link::search`. No `loader_deps` (loaders run per location, query string included). |
| E.5 Guards and control flow | Partial | `Router::guard`, `RouteControl`, `before_load` on route files. Guards are synchronous. |
| E.6 Loaders and preloading | Partial | `Router::loader` / `loader_to`, `file_route! { loader }`, once per location; `Link::preload(true)`; `router::preload`; fetches a loader started are cancelled when the user navigates away. Not yet: `pending_ms` / `pending_min_ms`, viewport preloading, a `router::state` for progress bars. |
| E.7 Blocking | Done | `use_blocker` with `proceed` / `reset`. |
| E.7 Per-window history | Done | `set_per_window_history`, `with_window`; a router's subtree reads its window's history. App-wide stays the default. |
| E.7 Persistence | Done | `router::persist_location` (router + persist). |
| E.7 Scroll restoration, transitions | Not started | |

## Part F: folder structure

| Item | Status | Notes |
|---|---|---|
| F.1 App structure | Done | `cargo rok-ui new` templates (`minimal`, `full`, `db`) and `examples/file_routes` follow it. |
| F.2 Workspace split | Deferred | Waits on decision K.2. Tooling crates are split out already (grammar, build, cli). |

## Part G: component standards

| Item | Status | Notes |
|---|---|---|
| G.1 Props | Done | `#[default]` / `#[default(expr)]`; required props stay constructor arguments, so a missing one is a compile error that names it (trybuild). No full typestate builder. |
| G.2 `#[key(..)]` | Done | `view!` / `children!` keyed loops wrap items in `Keyed`; hook state follows reordered items. |
| G.3 Component contract | Done | `.agents/skills/component/SKILL.md`. |
| G.4 `cargo rok-ui add` | Done | Copies sources with rewritten paths into `src/components/ui/`; all 60 components compile when copied together. Shared component helpers are `#[doc(hidden)] pub` for this. |

## Acceptance criteria (Part I)

| Release | Criterion | Status |
|---|---|---|
| 0.6 | Pedantic clippy clean with `-D warnings` | Done |
| 0.6 | Zero `ignore` doc blocks | Done |
| 0.6 | `cargo hack` green | Done locally for every new feature; CI runs all |
| 0.6 | release-plz opens the release PR | Configured; runs on `main` |
| 0.6 | Public API unchanged (`cargo public-api`) | Additive only (one deprecation); not checked with the tool |
| 0.7 | Two components reading one key trigger one fetch | Done (`tests/query.rs`) |
| 0.7 | Prefix invalidation refetches all matching queries | Done (`tests/query.rs`) |
| 0.7 | A failing boundary does not affect siblings | Done (`tests/query.rs`) |
| 0.7 | An optimistic update rolls back on error | Done (`tests/query.rs`) |
| 0.8 | Adding `routes/notes/$id.rs` makes `routes::NotesId` available | Done (`examples/file_routes/tests`) |
| 0.8 | Linking with a wrong param type fails to compile | Done (trybuild, `tests/ui-router`) |
| 0.8 | A loader is not re-run within `stale_time` | Done in spirit: loaders run once per location and prefetch through the query cache (`tests/router.rs`) |
| 0.8 | Navigating away cancels its loader | Done (`tests/router.rs`) |
| 0.8 | Two windows keep independent histories | Done (`tests/router.rs`, opt-in) |
| 0.8 | A guard redirect never renders the guarded component | Done (`tests/router.rs`, example tests) |
| 0.9 | Blur and submit errors are kept apart | Done (`tests/form.rs`) |
| 0.9 | A debounced async validator runs once per pause | Done (`tests/form.rs`) |
| 0.9 | Array operations keep each row's state | Done (`tests/form.rs`) |
| 0.9 | garde errors land on the right fields | Done (`tests/form.rs`) |
| 0.9 | A failed submit focuses the first invalid field | Done (`tests/form.rs`) |
| 0.10 | Each template builds in CI | Done (`scripts/check-templates.sh`, CI job) |
| 0.10 | `cargo rok-ui add button` produces a compiling copy | Done (same script) |
| 0.10 | A missing required prop fails to compile with the prop name | Done (`tests/ui/missing_required_prop.rs`) |

## Part K: decisions

The plan left these open. The work so far assumed the following; each can still change.

| Decision | Assumed |
|---|---|
| K.1 Version cadence | Undecided; the work is additive, so it can ship as one 0.6 or be split. |
| K.2 Workspace split in 0.6 | Not done; only tooling crates were split out. |
| K.3 File routing naming | TanStack conventions (`$id.rs`), as proposed. |
| K.4 Router v2 compatibility | The 0.5 router stays as is; v2 features are additive, so there is no `router::legacy`. |
| K.5 Form validation adapters | `garde` only. |
| K.6 Remote procedures | Local only for now. |
| K.7 Signal primitives in rok-ui-hooks | Nothing moved; queries and forms did not need new primitives. |
| K.8 rok-db helpers | rok-db stays one data source (`db_query`, `db_mutation`, `watch_changes`). |
| K.9 Formatting | Stable rustfmt. |

## Still open

- The workspace split (F.2, decision K.2) and remote procedures (C.4, decision K.6).
- `#[shard]` (C.3); scoped `Cx` values (C.1); memoize scopes (C.5).
- Router: async guards, pending timing (`pending_ms`), viewport preloading, scroll
  restoration, transitions.
- Forms: bound `Combobox`, `DatePicker` and `InputOtp`, scroll-into-view on a failed submit, persisted drafts.
- Devtools: mutations, forms and the signal graph.
- 1.0: an API review against the component contract and Part B conventions, and
  `cargo public-api` checks in CI.

## Migration notes

`docs/migration/0.6.md` covers moving from `db::use_query`, components taking `Cx`, typed
routes and file-based routes.
