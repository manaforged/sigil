#![allow(missing_docs)]

use phf as _;
use proc_macro2 as _;
use quote as _;
use sigil as _;
use sigil_style as _;
use sigil_style_macros as _;
use syn as _;

#[test]
fn compile_fail_cases() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/trybuild/*.rs");
}
