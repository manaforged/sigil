//! Reactive absolute positioning — [`LayoutReactiveExt::position_reactive`].
//!
//! Binds a node's absolute inset to a `Signal<(f32, f32)>` without
//! rebuilding the node and without leaking style atoms per frame.
//!
//! ## Why a dedicated primitive
//!
//! The existing reactive style channel ([`sigil_style::NodeStyleExt::style_reactive`]) drives
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
//! reads it during per-node sync (`crate::engine`'s
//! `layout_inputs_for_node`) via an *untracked* `peek` — always the latest
//! value — and folds it into the node's absolute inset, forcing `position:
//! absolute`. Invalidation rides the existing layout-signature cache: the
//! signature hashes the live position (`crate::cache_key`), so a signal
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
//! (`crate::cache_key`). So **any `compute` after a `set` repositions the
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

use sigil_kernel::NodeBuilder;
use sigil_reactive::Signal;

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

/// Node extension that PUBLISHES the node's computed border-box size into a signal.
///
/// The inverse of [`ReactivePosition`]: layout writes out; the widget reads to size
/// itself (e.g. a virtualized list deriving its visible-row count from the real viewport
/// height, which it otherwise can't know at build time). The engine publishes from
/// `attach_results` only when the node's box changes, and [`Self::publish`] dedups against
/// the signal's current value, so a steady frame does no work and there's no feedback loop
/// (a resize publishes once, the dependent rebuild leaves the box unchanged, it settles).
///
/// Caveats (safe under the single-consumer pattern, but know them): the convergence
/// guarantee assumes the bound node's size is **content-independent** (it's driven by its
/// parent, like a `h-full` pane) — binding it on an `h-auto` node whose content reacts to
/// the published size would oscillate. Bind ONE node per signal (two nodes ping-pong the
/// last write). The signal must outlive the node, which holds for a widget that owns both.
#[derive(Clone, Copy)]
pub struct SizeReactive {
    size: Signal<(f32, f32)>,
}

impl SizeReactive {
    /// Write `(w, h)` into the bound signal iff it changed. Called by the layout engine
    /// during the Layout phase (writes are legal there; the dependent effect defers to the
    /// next Effect phase, so the in-flight paint is never invalidated).
    pub fn publish(&self, size: (f32, f32)) {
        if self.size.peek() != size {
            self.size.set(size);
        }
    }
}

/// Node extension that PUBLISHES the node's computed absolute border-box
/// rect `(x, y, w, h)` into a signal.
///
/// [`SizeReactive`]'s sibling for consumers that need the origin, not just
/// the extent — e.g. a drop target hit-testing the pointer against its own
/// laid-out rect during a widget-to-widget drag. Same publish discipline:
/// the engine writes from `attach_results` only when the node's box
/// changes, and [`Self::publish`] dedups against the signal's current
/// value, so a steady frame does no work. The same caveats apply: bind ONE
/// node per signal; the signal must outlive the node.
#[derive(Clone, Copy)]
pub struct BoundsReactive {
    bounds: Signal<(f32, f32, f32, f32)>,
}

impl BoundsReactive {
    /// Write `(x, y, w, h)` into the bound signal iff it changed. Called by
    /// the layout engine during the Layout phase (writes are legal there).
    pub fn publish(&self, bounds: (f32, f32, f32, f32)) {
        if self.bounds.peek() != bounds {
            self.bounds.set(bounds);
        }
    }
}

/// Adds `.position_reactive()` / `.size_reactive()` / `.bounds_reactive()`
/// to [`NodeBuilder`].
pub trait LayoutReactiveExt {
    /// Bind this node's absolute inset (`inset_left`, `inset_top`) to
    /// `pos`, forcing `position: absolute`. The node keeps a stable
    /// identity across updates, so an in-flight pointer capture (drag) is
    /// preserved. Leak-free: the position is read live from a `Copy`
    /// signal, never interned into a `&'static` atom slice.
    #[must_use]
    fn position_reactive(self, pos: Signal<(f32, f32)>) -> Self;

    /// Publish this node's computed border-box size into `size` after every layout pass
    /// that changes it. Lets a widget react to its own (dynamic, parent-driven) size —
    /// e.g. virtualize against the real viewport height instead of a fixed guess.
    #[must_use]
    fn size_reactive(self, size: Signal<(f32, f32)>) -> Self;

    /// Publish this node's computed absolute border-box rect `(x, y, w, h)`
    /// into `bounds` after every layout pass that changes it. Lets an event
    /// handler hit-test the pointer against the node's real laid-out rect —
    /// e.g. a drop target under a dragged payload.
    #[must_use]
    fn bounds_reactive(self, bounds: Signal<(f32, f32, f32, f32)>) -> Self;
}

impl LayoutReactiveExt for NodeBuilder {
    fn position_reactive(self, pos: Signal<(f32, f32)>) -> Self {
        self.mutate(move |node| {
            let _ = node.attach(ReactivePosition { pos });
        })
    }

    fn size_reactive(self, size: Signal<(f32, f32)>) -> Self {
        self.mutate(move |node| {
            let _ = node.attach(SizeReactive { size });
        })
    }

    fn bounds_reactive(self, bounds: Signal<(f32, f32, f32, f32)>) -> Self {
        self.mutate(move |node| {
            let _ = node.attach(BoundsReactive { bounds });
        })
    }
}
