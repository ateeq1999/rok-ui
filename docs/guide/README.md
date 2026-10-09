# rok-ui guides

In-depth guides to the parts of rok-ui that go beyond single components. The main
[README](../../README.md) covers installation, components, styling, theming and
right-to-left support.

| Guide | Feature | Covers |
|---|---|---|
| [Architecture](architecture.md) | `bloc` (opt-in) | The standard app structure: data, business logic (blocs, cubits) and presentation; the rules; testing |
| [HTTP](http.md) | `http` (opt-in) | `HttpClient`, `ApiError`, `Session`, cancellation, API errors on forms |
| [The CLI](cli.md) | `rok-ui-cli` | `cargo rok-ui new`, `generate` (features, blocs, APIs from flags or JSON), `add`, `routes` |
| [App shells and layouts](app-shells.md) | `shell` (in `full`) | `Scaffold`, `AppBar`, navigation bar, rail and drawer, `AdaptiveScaffold`, `Row` / `Column` / `Stack` / `GridView` / `LayoutBuilder` |
| [Reactive state](state.md) | `state` (in `full`) | rok-ui-hooks signals, memos, effects and stores; `cx.track`, `use_signal`, `use_tracked`; async resources |
| [Data](query.md) | `query` (opt-in) | Query cache, keys and invalidation, mutations, procedures, `Suspense`, `#[memoize]` |
| [Forms](forms.md) | `form` (in `full`) | `use_form`, typed fields, validators per event, async validation, list fields, bound controls |
| [Routing](router.md) | `router` (opt-in) | Patterns, typed routes, search params, guards, loaders, blocking, file-based routes |
| [Database](database.md) | `db` (opt-in) | PostgreSQL with rok-db: connecting, `db_query`, mutations, `run`, transactions, testing |

Runnable examples live in [`examples/`](../../examples):

| Example | Shows | Run |
|---|---|---|
| `gallery` | Every component, themes, RTL | `cargo run --example gallery` |
| `app_shell` | `AdaptiveScaffold`, drawers, layout widgets | `cargo run --example app_shell` |
| `notes` | The BLoC architecture: repository, bloc, builders, router | `cargo run --example notes --features router,bloc` |
| `db_users` | rok-db queries and writes | `DATABASE_URL=… cargo run --example db_users --features db` |
| `sign_up` | Forms: validation, async checks, list fields, submit | `cargo run --example sign_up` |
| `file_routes` | File-based routes, layouts, guards, loaders, queries | `cargo run -p rok-ui-example-file-routes` |
| `arabic`, `arabic_chat` | Arabic UI in Cairo | `cargo run --example arabic --features font-cairo` |

The same guides are the module documentation on docs.rs: `rok_ui::bloc`, `rok_ui::http`,
`rok_ui::state`, `rok_ui::query`, `rok_ui::form`, `rok_ui::router` and `rok_ui::db`. Upgrading from 0.5: see
[docs/migration/0.6.md](../migration/0.6.md).
