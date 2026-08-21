#![allow(clippy::unwrap_used)]

use super::*;
use sigil_kernel::Cx;
use sigil_reactive::Runtime;

#[test]
fn dot_uses_generic_container_role() {
    Runtime::scope("dot-role", || {
        let mut cx = Cx::new_root();
        let id = dot(Tone::Success).build(&mut cx);
        let node = cx.get(id).expect("dot node exists");
        assert_eq!(node.a11y.role, sigil_kernel::Role::GenericContainer);
    });
}

#[test]
fn dot_wraps_step9_fill_in_step11_ring() {
    // Two-layer construction: outer step-11 ring + inner step-9
    // fill. The ring lifts edge contrast against the surface so
    // low-luminance hues (amber / yellow / green) still meet WCAG
    // 1.4.11 at the 8 px Compact size.
    Runtime::scope("dot-ring-and-fill", || {
        let mut cx = Cx::new_root();
        let id = dot(Tone::Warn).build(&mut cx);
        let node = cx.get(id).expect("dot node exists");
        assert_eq!(
            node.children().len(),
            1,
            "dot is outer ring + one inner fill child",
        );
    });
}
