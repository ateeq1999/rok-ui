---
name: bloc
description: Build or change app features in the BLoC architecture (data / business logic / presentation), use `cargo rok-ui generate`, or work on `rok-ui-bloc`, `rok_ui::bloc` or the generator itself.
---

# BLoC architecture

The standard is `docs/guide/architecture.md` (also the `rok_ui::bloc` module docs); the
generator is documented in `docs/guide/cli.md`; decisions are in `docs/design/bloc.md`.

## Writing app code

1. Generate, then edit: `cargo rok-ui g feature <name> --event ... --dry-run` first, then
   without `--dry-run`. Prefer a JSON spec (`-j @spec.json`) for anything with more than a
   couple of events; `cargo rok-ui g schema` describes it. Exit code 2 means a generated file
   you edited would be overwritten: decide, then pass `--force` or leave it.
2. Keep the rules (architecture guide): one-way dependencies; no bloc inside a bloc; no GPUI
   in `features/<f>/bloc/`; states are `Clone + PartialEq` values; events past tense; views
   only read state and add events; repositories own mapping and caching; routes are thin;
   barrel files, no `mod.rs`.
3. Business logic goes in the bloc's `on` handler (async, on the shared runtime). Views read
   with `cx.bloc::<B>()` and render with `BlocBuilder`; side effects use `BlocListener`.
4. Test each event with `rok_ui::bloc::test::run` (`run_cubit` for cubits) and a fake
   repository in `tests/features/<feature>/`.
5. Check: `cargo clippy --all-targets -- -D warnings` must pass; the template's `clippy.toml`
   fails the build if a GPUI type reaches the bloc layer.

## Working on rok-ui itself

- `rok-ui-bloc/` is GPUI-free (tokio only): `Bloc`, `Cubit`, `Emitter`, handles,
  concurrency, `test`. Keep it that way; `tests/ui/bloc_holds_bloc.rs` proves a bloc cannot
  hold a `BlocHandle`.
- `src/bloc.rs` is the GPUI side: providers (a thread-local scope pushed during layout and
  paint), builders, listeners, `cx.bloc`. Tests: `tests/bloc.rs` (GPUI) and
  `rok-ui-bloc/tests/`.
- The generator is `rok-ui-cli/src/generate/`: `spec.rs` (the JSON, `deny_unknown_fields`,
  schemars), `resolve.rs` (defaults, checks with JSON paths), `render.rs` (code),
  `output.rs` (barrels, markers, conflicts, atomic writes). Every change to generated code:
  run `UPDATE_SNAPSHOTS=1 cargo test -p rok-ui-cli`, read the snapshot diff, and run
  `scripts/check-templates.sh`, which builds the `bloc` and `bloc-http` templates and extra
  generated features under clippy pedantic with warnings denied.
- Generated code must pass clippy pedantic: spell out defaults (`String::new()`, not
  `Default::default()`), add `;` after unit calls in closures, import only what is used.
