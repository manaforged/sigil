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

use sigil::prelude::*;

const VIEWPORT: Size = Size::fixed(640.0, 480.0);

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(40.0, 16.0)
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

fn height_of(cx: &Cx, layout: &LayoutEngine, root: NodeId, label: &str) -> f32 {
    let hits = find_by_role_label(cx, root, Role::GenericContainer, label);
    hits.iter()
        .filter_map(|id| layout.result(*id).map(|res| res.rect().height))
        .sum()
}

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
        let h0 = layout.result(static0[0]).map_or(0.0, |r| r.rect().height);
        let h1 = layout.result(static1[0]).map_or(0.0, |r| r.rect().height);
        assert_eq!(h0, 0.0, "active=1 — static-0 collapses (height = {h0})");
        assert!(h1 > 0.0, "active=1 — static-1 visible (height = {h1})");
    });
}

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
        assert!(
            height_of(&cx, &layout, root, "mix-factory") > 0.0,
            "active=1 — factory body mounts under children_reactive"
        );
    });
}
