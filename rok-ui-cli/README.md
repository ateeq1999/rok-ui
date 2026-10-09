# rok-ui-cli

`cargo rok-ui`: create [rok-ui](https://crates.io/crates/rok-ui) apps, generate BLoC features,
vendor components and write route trees.

```sh
cargo install rok-ui-cli

cargo rok-ui new my-app --template full     # minimal | full | db | bloc [--http]
cargo rok-ui g feature notes --event NotesRequested --event NoteAdded:title:String
cargo rok-ui g api todos -j @todos.json     # HTTP provider, DTOs, repository
cargo rok-ui add button dialog              # copy components into src/components/ui/
cargo rok-ui routes                         # write src/route_tree.rs from src/routes/
```

The full reference for `generate` (flags, the JSON spec, exit codes) is the
[CLI guide](https://github.com/ateeq1999/rok-ui/blob/main/docs/guide/cli.md).
