---
name: macro
description: Rules for rok-ui proc macros (grammar and expansion split).
---

# Macros

Parsing and code generation live in `rok-ui-grammar/` (a normal library, one module per
macro, tested in `rok-ui-grammar/tests/expansions.rs`). `macros/` (`rok-ui-macros`) only has
the `#[proc_macro*]` entry points, which forward tokens to `rok_ui_grammar::<module>::expand*`
and turn errors into `compile_error!` (enhance.md H.2).

Rules for new and changed macros:

1. Parse into a lossless syntax tree: keep tokens and spans (`pub` token fields) so errors
   point at the user's code.
2. `Parse` validates the shape without interpreting it. Interpretation lives in methods on the
   tree.
3. Generate code with `ToTokens`/`quote!`; always use absolute paths (`::rok_ui::...`,
   `::core::...`) so expansions work in any module.
4. Entry points parse, call the expansion, and turn `syn::Error` into `to_compile_error()`.
5. Generated public items carry docs (CI runs with `missing_docs`) and generated builders carry
   `#[must_use]`.
6. Tests: an expansion test per form in `rok-ui-grammar/tests/expansions.rs` (it parses the
   output as Rust), a trybuild failure case in `tests/ui/` for each error the user sees, and a
   doctest in the macro's docs.
