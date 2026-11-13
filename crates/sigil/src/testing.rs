use crate::kernel::Cx;
use crate::reactive::Owner;
use crate::reactive::Runtime;

/// Runs `f` with a new root tree inside an owner scope named `name`.
pub fn cx<F: FnOnce(&mut Cx)>(name: &'static str, f: F) {
    Owner::scope(name, || {
        let mut cx = Cx::new_root();
        f(&mut cx);
    });
}

/// Runs `f` with the reactive runtime inside an owner scope named `name`.
pub fn runtime<F: FnOnce(&Runtime)>(name: &'static str, f: F) {
    Owner::scope(name, || f(&Runtime));
}

/// Fires the interval timers that are due and returns how many fired.
#[must_use]
pub fn tick_intervals() -> usize {
    crate::reactive::tick_intervals()
}

/// Returns the number of nodes in the tree that `root` builds.
pub fn tree_size(name: &'static str, root: impl crate::kernel::IntoNode) -> usize {
    let mut size = 0;
    cx(name, |cx| {
        let root = cx.add(root);
        cx.visit_preorder(root, |_, _| size += 1);
    });
    size
}

/// Lists the design rules that the laid-out tree of `host` breaks.
pub fn flaws(
    host: &crate::Host,
    tolerance: crate::layout::audit::Tolerance,
) -> Vec<crate::layout::audit::Issue> {
    host.flaws(tolerance)
}

/// Emits the scene of `host` from the previous scene `prev` and the changes in `delta`.
pub fn emit_since(
    host: &mut crate::Host,
    viewport: (f32, f32),
    delta: &crate::kernel::FrameDelta,
    prev: &crate::scene::CommandScene,
) -> (crate::scene::CommandScene, crate::scene::emit::EmitDiff) {
    host.emit_since(viewport, delta, prev)
}

/// Takes the changes that the tree recorded since the last call.
pub fn take_delta(cx: &mut Cx) -> crate::kernel::FrameDelta {
    cx.take_delta()
}

/// The resolved layout values of `style`.
#[must_use]
pub const fn style_data(style: &crate::layout::LayoutStyle) -> &crate::layout::StyleData {
    style.data()
}

#[cfg(test)]
#[path = "testing_tests.rs"]
mod tests;
