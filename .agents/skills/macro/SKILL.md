---
name: macro
description: Rules for rok-ui proc macros (grammar and expansion split).
---

# Macros

rok-ui's macros live in `macros/` (`rok-ui-macros`). Target architecture (enhance.md H.2): a
`rok-ui-grammar` crate with the syntax trees and code generation, and `rok-ui-macros` with
entry points only.

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
6. Tests: an expansion test per form, a trybuild failure case in `tests/ui/` for each error
   message, and a doctest in the macro's docs.
