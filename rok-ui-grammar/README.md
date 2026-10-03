# rok-ui-grammar

Parsing and code generation for [rok-ui](https://crates.io/crates/rok-ui)'s macros
(`#[component]`, `styles!`, `view!`, `#[procedure]`, the derives). The proc-macro crate,
`rok-ui-macros`, only forwards tokens to these functions, so expansions are tested as plain
Rust. Not intended for direct use.
