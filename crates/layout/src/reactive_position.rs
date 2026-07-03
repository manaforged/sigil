//! Reactive absolute positioning — [`LayoutReactiveExt::position_reactive`].
//!
//! Binds a node's absolute inset to a `Signal<(f32, f32)>` without
//! rebuilding the node and without leaking style atoms per frame.
//!
//! ## Why a dedicated primitive
//!
//! The existing reactive style channel ([`style::style_reactive`]) drives
//! layout-affecting atoms (e.g. the splitter's pane `Width`), but it
//! requires `&'static [StyleAtom]` — so a live, continuously-varying value
//! must be interned into a leaked slice on every change (the splitter
//! bounds this with an atom cache). For dragging a node across a canvas
//! that is an unbounded per-frame leak. Worse, inset is not even
//! expressible as a `StyleAtom` — it lives only on `LayoutStyle`.
//!
//! ## Mechanism
//!
//! The extension simply holds the `Signal<(f32, f32)>`. The layout engine
//! reads it during per-node sync ([`crate::engine`]'s
//! `layout_inputs_for_node`) via an *untracked* `peek` — always the latest
//! value — and folds it into the node's absolute inset, forcing `position:
//! absolute`. Invalidation rides the existing layout-signature cache: the
//! signature hashes the live position ([`crate::cache_key`]), so a signal
//! change yields a cache miss and a recompute, while an unchanged position
//! is a cache hit (no work). No `&'static` interning, no allocation, no
//! leak — `Signal` is `Copy`.
//!
//! Because the node keeps a stable identity across updates, an in-flight
//! pointer capture (a drag) survives — unlike a `children_reactive`
//! rebuild, which would mint a fresh `NodeId` mid-drag and drop the capture.
//!
//! ## Invalidation scope (verified)
//!
//! The untracked `peek` creates no reactive dependency, so a `pos.set(...)`
//! does not mark the node dirty or bump `Cx::mutation_generation`. Instead
//! the layout-signature cache catches it: `resolve_cache_key` re-hashes the
//! subtree every `compute`, and the signature folds in the live position
//! ([`crate::cache_key`]). So **any `compute` after a `set` repositions the
//! node** — no dirty mark needed. This is proven two ways:
//!   * `crates/layout/tests/reactive_position.rs` — bare `set` + `compute`
//!     moves the box (no events at all);
//!   * `crates/events/tests/routing_drag.rs` — a full routed pointer drag
//!     (capture → `delta` → `set`) moves the box.
//!
//! Because the production frame loop calls `compute` every frame, a live
//! mouse drag — where pointer-move OS events drive frames continuously —
//! repositions smoothly. The one gap: a position change with **no frame in
//! flight** (e.g. a timer-driven tween that nothing else is repainting)
//! waits until the next unrelated event. Self-driven animation that needs
//! per-frame ticks without input is a separate concern (a frame scheduler),
//! not a defect in this primitive.

use kernel::NodeBuilder;
use reactive::Signal;

/// Node extension carrying the live absolute position. Read (untracked) by
/// the layout engine each sync and hashed into the layout signature.
#[derive(Clone, Copy)]
pub struct ReactivePosition {
    pos: Signal<(f32, f32)>,
}

impl ReactivePosition {
    /// The current `(x, y)` inset in logical pixels (untracked read).
    #[must_use]
    pub fn get(&self) -> (f32, f32) {
        self.pos.peek()
    }
}

/// Adds `.position_reactive()` to [`NodeBuilder`].
pub trait LayoutReactiveExt {
    /// Bind this node's absolute inset (`inset_left`, `inset_top`) to
    /// `pos`, forcing `position: absolute`. The node keeps a stable
    /// identity across updates, so an in-flight pointer capture (drag) is
    /// preserved. Leak-free: the position is read live from a `Copy`
    /// signal, never interned into a `&'static` atom slice.
    #[must_use]
    fn position_reactive(self, pos: Signal<(f32, f32)>) -> Self;
}

impl LayoutReactiveExt for NodeBuilder {
    fn position_reactive(self, pos: Signal<(f32, f32)>) -> Self {
        self.mutate(move |node| {
            let _ = node.attach(ReactivePosition { pos });
        })
    }
}
