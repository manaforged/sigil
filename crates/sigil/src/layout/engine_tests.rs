use crate::geometry::{LogicalSize, Size};
use crate::kernel::{Cx, TextContent, container};
use crate::reactive::{Runtime, effect, signal};
use crate::style::{Dimension, Px, StyleAtom};

use crate::layout::{LayoutBuilderExt, LayoutReactiveExt, LayoutStyle, TextMeasure};

use super::LayoutEngine;

struct Measure;

impl TextMeasure for Measure {
    fn measure(&self, _: &TextContent, _: Size) -> LogicalSize {
        LogicalSize::new(0.0, 0.0)
    }
}

#[test]
fn bounds_callbacks_invalidate_geometry_for_the_next_frame() {
    Runtime::scope("layout-bounds-feedback", || {
        let measured = signal((0.0, 0.0));
        let shifted = signal(false);
        effect(move || {
            if measured.get().1 > 0.0 {
                shifted.set(true);
            }
        });
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(80.0, 40.0))
                .size_reactive(measured)
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(20.0, 10.0))
                        .style_reactive(move || {
                            if shifted.get() {
                                &[StyleAtom::MarginTop(Px(20))] as &[_]
                            } else {
                                &[StyleAtom::MarginTop(Px(0))] as &[_]
                            }
                        }),
                ),
        );
        let child = cx.first_child(root).expect("child");
        let mut engine = LayoutEngine::with_text_measure(Box::new(Measure));
        let viewport = Size::fixed(80.0, 40.0);
        engine
            .compute(&mut cx, root, viewport)
            .expect("measurement");
        assert!(shifted.peek(), "the measurement requests a new position");
        engine.compute(&mut cx, root, viewport).expect("next frame");
        let top = engine.result(child).expect("child layout").rect().origin.y;
        assert!((top - 20.0).abs() < f32::EPSILON, "content stays at {top}");
        let generation = engine.generation();
        engine
            .compute(&mut cx, root, viewport)
            .expect("unchanged frame");
        assert_eq!(
            engine.generation(),
            generation,
            "settled geometry stays cached"
        );
    });
}

#[test]
fn repaint_reuses_layout_storage_and_preserves_geometry() {
    Runtime::scope("layout-repaint-storage", || {
        let revision = signal(0);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(80.0, 40.0))
                .children_reactive(move || {
                    let _revision = revision.get();
                    (0..20)
                        .map(|_| container().layout(LayoutStyle::block().fixed_size(80.0, 2.0)))
                        .collect::<Vec<_>>()
                }),
        );
        let mut engine = LayoutEngine::with_text_measure(Box::new(Measure));
        let viewport = Size::fixed(80.0, 40.0);
        engine
            .compute(&mut cx, root, viewport)
            .expect("first frame");
        let expected: Vec<_> = cx
            .get(root)
            .expect("root")
            .children()
            .iter()
            .map(|id| engine.result(*id).expect("child layout").rect())
            .collect();
        for frame in 1..100 {
            revision.set(frame);
            engine.compute(&mut cx, root, viewport).expect("repaint");
            let actual: Vec<_> = cx
                .get(root)
                .expect("root")
                .children()
                .iter()
                .map(|id| engine.result(*id).expect("child layout").rect())
                .collect();
            assert_eq!(actual, expected, "repaint changes no geometry");
            assert_eq!(
                engine.drive.count(),
                cx.len(),
                "frame {frame} retains dead layout nodes"
            );
        }
    });
}

static NARROW: &[StyleAtom] = &[StyleAtom::Width(Dimension::Px(Px(32)))];
static WIDE: &[StyleAtom] = &[StyleAtom::Width(Dimension::Px(Px(96)))];

#[test]
fn reactive_layout_style_change_invalidates_cache() {
    Runtime::scope("layout_reactive_style_cache", || {
        let wide = signal(false);
        let mut cx = Cx::new_root();
        let root =
            cx.add(container().style_reactive(move || if wide.get() { WIDE } else { NARROW }));
        let mut engine = LayoutEngine::with_text_measure(Box::new(Measure));
        let available = Size::fixed(120.0, 40.0);
        let width = |laid: &LayoutEngine| laid.result(root).expect("root layout").rect().width;

        engine
            .compute(&mut cx, root, available)
            .expect("initial style compute succeeds");
        let generation = engine.generation();
        assert_eq!(width(&engine).to_bits(), 32.0_f32.to_bits());

        wide.set(true);
        engine
            .compute(&mut cx, root, available)
            .expect("changed style compute succeeds");

        assert_ne!(engine.generation(), generation);
        assert_eq!(width(&engine).to_bits(), 96.0_f32.to_bits());
    });
}

#[test]
fn resting_interactive_nodes_are_not_reseeded_when_another_node_moves() {
    Runtime::scope("layout-resting-seed", || {
        let pos = signal((0.0_f32, 0.0_f32));
        let mut page = container().layout(LayoutStyle::flex_column().fixed_size(400.0, 800.0));
        for index in 0..20 {
            page = page.child(crate::widgets::button(format!("button {index}")));
        }
        page = page.child(
            container()
                .layout(LayoutStyle::block().fixed_size(20.0, 20.0))
                .position_reactive(pos),
        );
        let mut cx = Cx::new_root();
        let root = cx.add(page);
        let mut engine = LayoutEngine::with_text_measure(Box::new(Measure));
        engine
            .compute(&mut cx, root, Size::fixed(400.0, 800.0))
            .expect("first frame");
        engine
            .compute(&mut cx, root, Size::fixed(400.0, 800.0))
            .expect("settled frame");
        pos.set((0.0, 40.0));
        let mut dirty = Vec::new();
        engine.seed_reactive_dirty(&cx, &mut dirty, true, false);
        let buttons = dirty
            .iter()
            .filter(|id| {
                cx.get(**id)
                    .is_some_and(|node| node.a11y.role == accesskit::Role::Button)
            })
            .count();
        assert_eq!(buttons, 0, "resting buttons were reseeded: {dirty:?}");
    });
}

struct CharWidth;

impl TextMeasure for CharWidth {
    fn measure(&self, content: &TextContent, _: Size) -> LogicalSize {
        let chars = match content {
            TextContent::Static(text) => text.chars().count(),
            TextContent::Reactive(text) => text.peek().chars().count(),
            TextContent::Compute(text) => text().chars().count(),
        };
        let chars = u16::try_from(chars).unwrap_or(u16::MAX);
        LogicalSize::new(f32::from(chars) * 8.0, 16.0)
    }
}

#[test]
fn a_reactive_label_keeps_its_size_on_its_first_change() {
    Runtime::scope("layout-first-text-change", || {
        let label = signal("12:00:00".to_owned());
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(200.0, 40.0))
                .child(crate::kernel::text(label)),
        );
        let child = *cx
            .get(root)
            .and_then(|node| node.children().first())
            .expect("label node");
        let mut engine = LayoutEngine::with_text_measure(Box::new(CharWidth));
        engine
            .compute(&mut cx, root, Size::fixed(200.0, 40.0))
            .expect("first frame");
        label.set("12:00:01".to_owned());
        let kept = engine
            .dirty_roots(&mut cx, &[child], root, false)
            .expect("same-width change");
        assert!(kept.is_empty(), "same-width text re-solved {kept:?}");
        label.set("12:00:01 PM".to_owned());
        let grown = engine
            .dirty_roots(&mut cx, &[child], root, false)
            .expect("wider change");
        assert!(!grown.is_empty(), "wider text must re-solve");
    });
}

#[test]
fn a_resized_label_is_named_in_the_profile() {
    let _gate = crate::profile::exclusive();
    Runtime::scope("layout-attribution", || {
        let label = signal("short".to_owned());
        let mut cx = Cx::new_root();
        let mut page = container().layout(LayoutStyle::flex_column().fixed_size(400.0, 800.0));
        for index in 0..30 {
            page = page.child(crate::kernel::text(format!("row {index}")));
        }
        let root = cx.add(
            page.child(
                container()
                    .layout(LayoutStyle::flex_row().fixed_size(400.0, 40.0))
                    .child(crate::kernel::text(label)),
            ),
        );
        let mut engine = LayoutEngine::with_text_measure(Box::new(CharWidth));
        for _ in 0..2 {
            engine
                .compute(&mut cx, root, Size::fixed(400.0, 800.0))
                .expect("settled frame");
        }
        crate::profile::enable();
        crate::profile::reset();
        crate::profile::begin_frame(crate::profile::BackendStatus::TERMINAL);
        label.set("a much longer label".to_owned());
        engine
            .compute(&mut cx, root, Size::fixed(400.0, 800.0))
            .expect("resized frame");
        crate::profile::tick_frame();
        let hot = crate::profile::frames()
            .last()
            .map(|frame| frame.hot.clone())
            .unwrap_or_default();
        crate::profile::disable();
        let named = |hotspot: crate::profile::Hotspot| {
            hot.iter().any(|entry| {
                entry.hotspot == hotspot && entry.subject.contains("a much longer label")
            })
        };
        assert!(named(crate::profile::Hotspot::LayoutResized), "{hot:?}");
        assert!(
            hot.iter()
                .any(|entry| entry.hotspot == crate::profile::Hotspot::LayoutRoot
                    && entry.subject.contains("a much longer label")),
            "{hot:?}"
        );
    });
}

struct WordWrap;

impl TextMeasure for WordWrap {
    fn measure(&self, content: &TextContent, space: Size) -> LogicalSize {
        let text = content.resolve();
        let limit = match space.width {
            crate::geometry::Dim::Fixed(width) => width,
            _ => f32::INFINITY,
        };
        let (mut widest, mut line, mut lines) = (0.0_f32, 0.0_f32, 1_u16);
        for word in text.split(' ') {
            let width = f32::from(u16::try_from(word.chars().count()).unwrap_or(u16::MAX)) * 8.0;
            let next = if line > 0.0 {
                line + 8.0 + width
            } else {
                width
            };
            if line > 0.0 && next > limit {
                widest = widest.max(line);
                line = width;
                lines += 1;
            } else {
                line = next;
            }
        }
        LogicalSize::new(widest.max(line), f32::from(lines) * 16.0)
    }
}

#[test]
fn a_reflowed_label_is_not_kept_on_a_size_measured_at_its_old_width() {
    Runtime::scope("layout-reflowed-label", || {
        let label = signal("ab cd".to_owned());
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column())
                .child(crate::kernel::text(label).wrap()),
        );
        let child = *cx
            .get(root)
            .and_then(|node| node.children().first())
            .expect("label node");
        let mut engine = LayoutEngine::with_text_measure(Box::new(WordWrap));
        engine
            .compute(&mut cx, root, Size::fixed(200.0, 100.0))
            .expect("wide frame");
        engine
            .compute(&mut cx, root, Size::fixed(24.0, 100.0))
            .expect("narrow frame");
        label.set("abcde".to_owned());
        let roots = engine
            .dirty_roots(&mut cx, &[child], root, false)
            .expect("reflowed change");
        assert!(
            !roots.is_empty(),
            "a two-line label became one line but was kept"
        );
    });
}
