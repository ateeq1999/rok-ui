# rok-ui-cli

`cargo rok-ui`: create [rok-ui](https://crates.io/crates/rok-ui) apps, vendor components and
write route trees.

```sh
cargo install rok-ui-cli

cargo rok-ui new my-app --template full     # minimal | full | db
cargo rok-ui add button dialog              # copy components into src/components/ui/
cargo rok-ui routes                         # write src/route_tree.rs from src/routes/
```
