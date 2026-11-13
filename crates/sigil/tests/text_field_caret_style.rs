#![allow(clippy::expect_used)]

use sigil::kernel::Cx;
use sigil::layout::LayoutEngine;
use std::rc::Rc;

use sigil::events::EventRouter;
use sigil::geometry::{Constraint, Rect};
use sigil::layout::{TextMeasure, text_geometry};
use sigil::prelude::*;
use sigil::scene::{CommandScene, SceneCommand, emit_scene};
use sigil::text::HitPosition;

const VALUE: &str = "Testing this is a complet";

#[test]
fn caret_uses_the_fields_resolved_typography() {
    sigil::reactive::Owner::scope("text-field-caret-style", || {
        let scene = focused_field_scene();
        let text = scene.find_text(VALUE).expect("field text is painted");
        assert!(
            (text.font_size() - 14.0).abs() <= f32::EPSILON,
            "text-sm must paint at 14px (Tailwind scale)",
        );

        let expected = sigil::layout::TextEngineMeasure::default().caret(
            VALUE,
            text.style(),
            None,
            HitPosition::downstream(VALUE.len()),
        );
        let expected_x = text.rect().origin.x + expected.x();
        let caret = nearest_caret(&scene, expected_x);

        assert!(
            (caret.origin.x - expected_x).abs() <= 0.5,
            "caret must use the painted run's resolved typography: expected x={expected_x}, got x={}",
            caret.origin.x,
        );
        assert!(
            (caret.height - expected.height()).abs() <= 0.5,
            "caret height must use the painted run's resolved typography: expected {}, got {}",
            expected.height(),
            caret.height,
        );
    });
}

fn focused_field_scene() -> CommandScene {
    text_geometry(Rc::new(sigil::layout::TextEngineMeasure::default()));
    let mut cx = Cx::new_root();
    let root = cx.add(
        text_field(signal(VALUE.to_owned()))
            .width(420.0)
            .height(40.0)
            .font_size(14.0, "test fixture"),
    );
    let mut layout = LayoutEngine::new(Box::new(sigil::layout::TextEngineMeasure::default()));
    layout
        .compute(&mut cx, root, Constraint::fixed(500.0, 100.0))
        .expect("initial layout");
    EventRouter::new().set_focus(&mut cx, Some(root));
    layout
        .compute(&mut cx, root, Constraint::fixed(500.0, 100.0))
        .expect("focused layout");
    emit_scene(&cx, root, None)
}

fn nearest_caret(scene: &CommandScene, expected_x: f32) -> Rect {
    scene
        .commands()
        .iter()
        .filter_map(|command| match command {
            SceneCommand::Rect(draw)
                if (draw.rect().width - 2.0).abs() <= 0.01
                    && draw.rect().height < 30.0
                    && draw.stroke_style().is_none() =>
            {
                Some(draw.rect())
            }
            _ => None,
        })
        .min_by(|a, b| {
            (a.origin.x - expected_x)
                .abs()
                .total_cmp(&(b.origin.x - expected_x).abs())
        })
        .expect("focused field paints a caret")
}
