# rok-ui guides

In-depth guides to the parts of rok-ui that go beyond single components. The main
[README](../../README.md) covers installation, components, styling, theming and
right-to-left support.

| Guide | Feature | Covers |
|---|---|---|
| [App shells and layouts](app-shells.md) | `shell` (in `full`) | `Scaffold`, `AppBar`, navigation bar, rail and drawer, `AdaptiveScaffold`, `Row` / `Column` / `Stack` / `GridView` / `LayoutBuilder` |
| [Reactive state](state.md) | `state` (in `full`) | rok-ui-hooks signals, memos, effects and stores; `cx.track`, `use_signal`, `use_tracked`; async resources |
| [Routing](router.md) | `router` (opt-in) | Patterns, parameters and queries, navigation and history, `Link`, redirects, layouts, guards |
| [Database](database.md) | `db` (opt-in) | PostgreSQL with rok-db: connecting, `use_query`, `run`, transactions, errors, testing |

Runnable examples live in [`examples/`](../../examples):

| Example | Shows | Run |
|---|---|---|
| `gallery` | Every component, themes, RTL | `cargo run --example gallery` |
| `app_shell` | `AdaptiveScaffold`, drawers, layout widgets | `cargo run --example app_shell` |
| `notes` | Router, stores, `use_signal` | `cargo run --example notes --features router` |
| `db_users` | rok-db queries and writes | `DATABASE_URL=… cargo run --example db_users --features db` |
| `arabic`, `arabic_chat` | Arabic UI in Cairo | `cargo run --example arabic --features font-cairo` |

The same guides are the module documentation on docs.rs: `rok_ui::state`, `rok_ui::router` and
`rok_ui::db`.
