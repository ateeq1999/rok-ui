# rok-ui-build

Build-script support for [rok-ui](https://crates.io/crates/rok-ui): generates a typed route
tree from the files in `src/routes/`, like TanStack Router's route generator.

```rust
// build.rs
fn main() {
    rok_ui_build::routes("src/routes").generate().unwrap();
}
```

See the rok-ui router guide for the file conventions.
