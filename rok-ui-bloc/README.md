# rok-ui-bloc

The BLoC primitives behind [`rok_ui::bloc`](https://docs.rs/rok-ui): `Bloc`, `Cubit`,
`Emitter`, `BlocHandle`, the per-event concurrency modes and the `test` helpers. This crate has
no GPUI dependency, so the business-logic layer of an app (or a workspace crate holding it) can
depend on it alone. Apps normally use it through `rok_ui::bloc` (feature `bloc`).

See `docs/guide/architecture.md` in the rok-ui repository for the architecture standard.
