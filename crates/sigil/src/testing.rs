use crate::kernel::Cx;
use crate::reactive::Owner;
use crate::reactive::Runtime;

pub fn with_cx<F: FnOnce(&mut Cx)>(name: &'static str, f: F) {
    Owner::scope(name, || {
        let mut cx = Cx::new_root();
        f(&mut cx);
    });
}

pub fn with_runtime<F: FnOnce(&Runtime)>(name: &'static str, f: F) {
    Owner::scope(name, || f(&Runtime));
}

pub fn tick_intervals() -> usize {
    crate::reactive::tick_intervals()
}

pub fn tree_size(name: &'static str, root: impl crate::kernel::IntoNode) -> usize {
    let mut size = 0;
    with_cx(name, |cx| {
        let root = cx.add(root);
        cx.visit_preorder(root, |_, _| size += 1);
    });
    size
}

pub fn flaws(
    host: &crate::Host,
    tolerance: crate::layout::audit::Tolerance,
) -> Vec<crate::layout::audit::Issue> {
    host.flaws(tolerance)
}

pub fn emit_since(
    host: &mut crate::Host,
    viewport: (f32, f32),
    delta: &crate::kernel::FrameDelta,
    prev: &crate::scene::CommandScene,
) -> (crate::scene::CommandScene, crate::scene::emit::EmitDiff) {
    host.emit_since(viewport, delta, prev)
}

pub fn take_delta(cx: &mut Cx) -> crate::kernel::FrameDelta {
    cx.take_delta()
}

#[must_use]
pub const fn style_data(style: &crate::layout::LayoutStyle) -> &crate::layout::StyleData {
    style.data()
}

#[cfg(test)]
#[path = "testing_tests.rs"]
mod tests;
