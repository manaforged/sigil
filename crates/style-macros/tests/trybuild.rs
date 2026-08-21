#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

#[test]
fn compile_fail_cases() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/trybuild/*.rs");
}
