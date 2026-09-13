#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use std::cell::RefCell;
use std::rc::Rc;

use sigil::hal::{LogicalSize, Size};
use sigil::kernel::{TextContent, prelude::*};
use sigil::layout::{LayoutEngine, TextMeasure};
use sigil::style::{NodeStyleExt, StyleBinding, TextStyle, parse_class_string};

#[derive(Default)]
struct RecordingMeasure {
    calls: Rc<RefCell<Vec<(String, TextStyle)>>>,
}

impl TextMeasure for RecordingMeasure {
    fn measure(&self, content: &TextContent, _space: Size) -> LogicalSize {
        LogicalSize::new(8.0 * text_len(content), 16.0)
    }

    fn measure_styled(
        &self,
        content: &TextContent,
        style: &TextStyle,
        _space: Size,
    ) -> LogicalSize {
        self.calls
            .borrow_mut()
            .push((text_of(content), style.clone()));
        LogicalSize::new(style.font_size * text_len(content) * 0.5, style.font_size)
    }
}

fn text_of(content: &TextContent) -> String {
    match content {
        TextContent::Static(value) => value.clone(),
        TextContent::Reactive(value) => value.get(),
        TextContent::Compute(_) => content.resolve(),
    }
}

#[allow(clippy::cast_precision_loss)]
fn text_len(content: &TextContent) -> f32 {
    text_of(content).chars().count() as f32
}

type MeasureLog = Rc<RefCell<Vec<(String, TextStyle)>>>;

fn engine_with_recorder() -> (LayoutEngine, MeasureLog) {
    let recorder = RecordingMeasure::default();
    let calls = Rc::clone(&recorder.calls);
    (LayoutEngine::with_text_measure(Box::new(recorder)), calls)
}

fn styles_for<'a>(calls: &'a [(String, TextStyle)], needle: &str) -> Vec<&'a TextStyle> {
    calls
        .iter()
        .filter(|(text, _)| text == needle)
        .map(|(_, style)| style)
        .collect()
}

#[test]
fn text_child_measures_with_inherited_typography() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .class("text-lg font-bold")
            .child(text("Inherit")),
    );

    let (mut engine, calls) = engine_with_recorder();
    engine
        .compute(&mut cx, root, Size::fixed(300.0, 100.0))
        .expect("compute succeeds");

    let calls = calls.borrow();
    let styles = styles_for(&calls, "Inherit");
    assert!(
        !styles.is_empty(),
        "text child must be measured through measure_styled"
    );
    for style in styles {
        assert_eq!(
            style.font_size, 18.0,
            "measurement must see the ancestor's text-lg, not TextStyle::default()"
        );
        assert_eq!(
            style.font_weight, 700,
            "font-bold must inherit into measurement"
        );
    }
}

#[test]
fn ancestor_typography_change_remeasures_descendant_text() {
    let mut cx = Cx::new_root();
    let styled = container().class("text-lg").child(text("Target"));
    let root = cx.add(
        container()
            .child(styled)
            .child(container().child(text("Noise one")))
            .child(container().child(text("Noise two")))
            .child(container().child(text("Noise three")))
            .child(container().child(text("Noise four"))),
    );
    let styled_id = cx.get(root).unwrap().children().first().copied().unwrap();

    let (mut engine, calls) = engine_with_recorder();
    engine
        .compute(&mut cx, root, Size::fixed(600.0, 400.0))
        .expect("first compute");
    {
        let first = calls.borrow();
        let target_styles = styles_for(&first, "Target");
        assert!(target_styles.iter().all(|style| style.font_size == 18.0));
    }
    calls.borrow_mut().clear();

    let atoms = parse_class_string("text-xl").expect("text-xl parses");
    cx.get_mut(styled_id)
        .unwrap()
        .attach(StyleBinding::owned_atoms(atoms));
    engine
        .compute(&mut cx, root, Size::fixed(600.0, 400.0))
        .expect("second compute");

    let second = calls.borrow();
    let target_styles = styles_for(&second, "Target");
    assert!(
        !target_styles.is_empty(),
        "descendant text must re-measure when an ancestor's typography changes \
         (its own Node::version is unchanged — this is the DR-056 cascade)"
    );
    for style in target_styles {
        assert_eq!(style.font_size, 20.0, "re-measure must see text-xl");
    }
}
