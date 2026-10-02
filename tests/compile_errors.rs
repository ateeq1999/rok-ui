//! Style typos are compile errors that point at the mistake.

#[test]
fn style_mistakes_fail_to_compile() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/ui/*.rs");
}
