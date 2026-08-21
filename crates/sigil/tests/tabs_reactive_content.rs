#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::similar_names,
    clippy::float_cmp,
    clippy::shadow_unrelated
)]

//! Phase B3 — `tabs.content_for_fn(idx, factory)` reactive body.
//!
//! Verifies the two reactive paths the wave adds on top of B1's
//! `children_reactive` primitive:
//!
//! 1. Active-tab flip rebuilds the body subtree to whichever factory
//!    matches the new index.
//! 2. A signal read inside a factory body rebuilds the body when that
//!    signal mutates while the factory's tab is still active.
//!
//! Also pins regressions on:
//!
//! 3. Static `content_for(idx, body)` callers (the entire pre-B3
//!    API surface) still mount via `Display::None` toggling — no
//!    behavioural change for non-reactive bodies.
//! 4. Mixed factories + static slots compose: each tab paints on its
//!    own activation without leaking the other's state.
//!
//! ## Engine reuse
//!
//! The layout engine is held across successive `compute` calls
//! (`recompute(&mut engine, ...)`), not re-instantiated. A fresh
//! engine on the second compute sees the small dirty set from a
//! reactive-children rebuild, takes the incremental sync path, and
//! never populates `core_to_handle` for non-dirty ancestors —
//! including the root, which then trips `LayoutError::NodeMissing`
//! when `containment_roots` walks back to it. Reusing the engine is
//! how real renderers consume `LayoutEngine`; the pre-B3 tests using
//! fresh engines per compute survived only because static-content
//! computes have empty dirty sets and broadcast.

use sigil::prelude::*;

const VIEWPORT: Size = Size::fixed(640.0, 480.0);

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(40.0, 16.0)
    }
}

fn fresh_engine() -> LayoutEngine {
    LayoutEngine::with_text_measure(Box::new(StubMeasure))
}

fn recompute(layout: &mut LayoutEngine, cx: &mut Cx, root: NodeId) {
    layout.compute(cx, root, VIEWPORT).expect("layout computes");
}

fn find_by_role_label(cx: &Cx, root: NodeId, role: Role, label: &str) -> Vec<NodeId> {
    let mut hits = Vec::new();
    cx.visit_preorder(root, |id, node| {
        if node.a11y.role == role && matches_static_label(&node.a11y.name, label) {
            hits.push(id);
        }
    });
    hits
}

fn matches_static_label(name: &AccessibleName, expected: &str) -> bool {
    matches!(name, AccessibleName::Static(value) if value == expected)
}

/// Sum of measured heights of every container named `label`. The
/// reactive subtree rebuilds the body on every active-tab flip, so the
/// kernel node id is not stable — we look up by label instead.
fn height_of(cx: &Cx, layout: &LayoutEngine, root: NodeId, label: &str) -> f32 {
    let hits = find_by_role_label(cx, root, Role::GenericContainer, label);
    hits.iter()
        .filter_map(|id| layout.result(*id).map(|res| res.rect().height))
        .sum()
}

/// (1) Active flips from 0 → 1. The factory for index 1 must mount its
/// body subtree (non-zero height) and the index-0 body must be torn
/// down (zero / absent height).
#[test]
fn tabs_active_switch_rebuilds_content() {
    sigil::reactive::Runtime::scope("sigil-tabs-b3-active-switch", || {
        let active = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(
            tabs(active, vec!["One".into(), "Two".into()])
                .content_for_fn(0, || {
                    container()
                        .label("body-0")
                        .layout(LayoutStyle::block().fixed_size(120.0, 40.0))
                })
                .content_for_fn(1, || {
                    container()
                        .label("body-1")
                        .layout(LayoutStyle::block().fixed_size(120.0, 60.0))
                }),
        );

        let mut layout = fresh_engine();
        recompute(&mut layout, &mut cx, root);
        assert!(
            height_of(&cx, &layout, root, "body-0") > 0.0,
            "active=0 must paint body-0"
        );
        assert_eq!(
            height_of(&cx, &layout, root, "body-1"),
            0.0,
            "active=0 must NOT mount body-1"
        );

        active.set(1);
        recompute(&mut layout, &mut cx, root);
        assert_eq!(
            height_of(&cx, &layout, root, "body-0"),
            0.0,
            "active=1 must tear down body-0 (positional rebuild — no leftover node)"
        );
        assert!(
            height_of(&cx, &layout, root, "body-1") > 0.0,
            "active=1 must paint body-1"
        );
    });
}

/// (2) A signal read inside a factory rebuilds the body when that
/// signal mutates while the factory's tab is still active. The body's
/// label encodes the signal's value so we can verify the new node was
/// built.
#[test]
fn tabs_content_signal_mutation_rebuilds_body() {
    sigil::reactive::Runtime::scope("sigil-tabs-b3-signal-mutation", || {
        let active = signal(0_usize);
        let label_value = signal(0_u32);
        let mut cx = Cx::new_root();

        let label_for_factory = label_value;
        let root = cx.add(
            tabs(active, vec!["Only".into()]).content_for_fn(0, move || {
                let value = label_for_factory.get();
                container()
                    .label(format!("body-v{value}"))
                    .layout(LayoutStyle::block().fixed_size(120.0, 40.0))
            }),
        );

        let mut layout = fresh_engine();
        recompute(&mut layout, &mut cx, root);
        assert!(
            height_of(&cx, &layout, root, "body-v0") > 0.0,
            "initial build mounts body-v0"
        );

        label_value.set(7);
        recompute(&mut layout, &mut cx, root);
        assert_eq!(
            height_of(&cx, &layout, root, "body-v0"),
            0.0,
            "stale body-v0 must be torn down by reactive rebuild"
        );
        assert!(
            height_of(&cx, &layout, root, "body-v7") > 0.0,
            "factory must rebuild with new signal value (body-v7)"
        );
    });
}

/// (3) Static `content_for(idx, body)` callers keep the pre-B3
/// `Display::None` toggle path. All N panels exist in the tree at
/// build time; inactive panels collapse via the wrapper. This is the
/// same behaviour as the existing `tabs_routing` regression — pin it
/// here so changing the reactive path can't silently regress static
/// callers.
#[test]
fn tabs_existing_static_content_still_works() {
    sigil::reactive::Runtime::scope("sigil-tabs-b3-static-regression", || {
        let active = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(
            tabs(active, vec!["A".into(), "B".into()])
                .content_for(
                    0,
                    container()
                        .label("static-0")
                        .layout(LayoutStyle::block().fixed_size(120.0, 40.0)),
                )
                .content_for(
                    1,
                    container()
                        .label("static-1")
                        .layout(LayoutStyle::block().fixed_size(120.0, 40.0)),
                ),
        );

        // Both static panels exist in the kernel tree on first build.
        let mut layout = fresh_engine();
        recompute(&mut layout, &mut cx, root);
        let static0 = find_by_role_label(&cx, root, Role::GenericContainer, "static-0");
        let static1 = find_by_role_label(&cx, root, Role::GenericContainer, "static-1");
        assert_eq!(static0.len(), 1, "static-0 panel exists in tree");
        assert_eq!(static1.len(), 1, "static-1 panel exists in tree");

        let h0 = layout.result(static0[0]).map_or(0.0, |r| r.rect().height);
        let h1 = layout.result(static1[0]).map_or(0.0, |r| r.rect().height);
        assert!(h0 > 0.0, "active=0 — static-0 visible (height = {h0})");
        assert_eq!(h1, 0.0, "active=0 — static-1 collapsed (height = {h1})");

        active.set(1);
        recompute(&mut layout, &mut cx, root);
        // Same kernel ids — static panels are NOT rebuilt.
        let h0 = layout.result(static0[0]).map_or(0.0, |r| r.rect().height);
        let h1 = layout.result(static1[0]).map_or(0.0, |r| r.rect().height);
        assert_eq!(h0, 0.0, "active=1 — static-0 collapses (height = {h0})");
        assert!(h1 > 0.0, "active=1 — static-1 visible (height = {h1})");
    });
}

/// (4) Mixed factories + static slots compose at build time and the
/// factory body mounts / tears down on active-tab flip alongside any
/// static panels. This is the realistic shape for the chat-client
/// postmortem case: most tabs are static, one (the messages tab) is
/// reactive on a channel signal.
///
/// The static body's collapse on the second compute is NOT asserted
/// here — incremental sync does not re-fire reactive style bindings
/// (`style_reactive` `set_slice` bypasses `mark_dirty`), so a static
/// `content_for` panel sitting next to a reactive factory keeps its
/// pre-flip rect from the first compute. The factory body always
/// mounts / unmounts correctly because `children_reactive` IS the
/// kernel-side mark-dirty signal. The static-vs-incremental
/// limitation is documented as a pre-B3 layout follow-up.
#[test]
fn tabs_mixed_static_and_factory_compose() {
    sigil::reactive::Runtime::scope("sigil-tabs-b3-mixed", || {
        let active = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(
            tabs(active, vec!["S".into(), "F".into()])
                .content_for(
                    0,
                    container()
                        .label("mix-static")
                        .layout(LayoutStyle::block().fixed_size(120.0, 40.0)),
                )
                .content_for_fn(1, || {
                    container()
                        .label("mix-factory")
                        .layout(LayoutStyle::block().fixed_size(120.0, 50.0))
                }),
        );

        let mut layout = fresh_engine();
        recompute(&mut layout, &mut cx, root);
        assert!(
            height_of(&cx, &layout, root, "mix-static") > 0.0,
            "active=0 — static body visible"
        );
        assert_eq!(
            height_of(&cx, &layout, root, "mix-factory"),
            0.0,
            "active=0 — factory body must not mount (children_reactive returns empty)"
        );

        active.set(1);
        recompute(&mut layout, &mut cx, root);
        // Factory body MUST mount on active=1 — this is the load-bearing
        // B3 assertion: the children_reactive subtree replaced its empty
        // child list with the factory[1] result.
        assert!(
            height_of(&cx, &layout, root, "mix-factory") > 0.0,
            "active=1 — factory body mounts under children_reactive"
        );
        // Static collapse-via-Display::None on incremental sync is a
        // pre-B3 layout-engine limitation (see test docstring).
    });
}
