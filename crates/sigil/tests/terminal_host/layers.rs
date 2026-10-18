use proptest::prelude::*;
use sigil::Host;
use sigil::kernel::{NodeBuilder, container, text};
use sigil::layout::LayoutStyle;
use sigil::prelude::{LayoutBuilderExt, Paint, Signal};
use sigil::reactive::{Runtime, signal};
use sigil::style::{NodeStyleExt, NumericAtomKind, Theme, Token, Transform2D};
use sigil::terminal::Access;
use sigil::widgets::{modal, scroll};

use super::live::Side;

const CELL: f32 = Host::CELL;

#[derive(Clone, Debug)]
enum Step {
    Tint,
    Theme,
    Shift(f32),
    Label(String),
    Open,
    Lines(usize),
    Scroll(f32),
    Resize(u16, u16),
}

#[derive(Clone, Copy)]
struct Knobs {
    tint: Signal<bool>,
    dark: Signal<bool>,
    shift: Signal<f32>,
    label: Signal<String>,
    open: Signal<bool>,
    lines: Signal<usize>,
    offset: Signal<f32>,
}

fn knobs() -> Knobs {
    Knobs {
        tint: signal(false),
        dark: signal(false),
        shift: signal(0.0),
        label: signal(String::from("moving")),
        open: signal(false),
        lines: signal(12),
        offset: signal(0.0),
    }
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        Just(Step::Tint),
        Just(Step::Theme),
        prop_oneof![Just(0.0_f32), Just(4.0), Just(24.0), Just(48.0)].prop_map(Step::Shift),
        "[a-z]{1,8}".prop_map(Step::Label),
        Just(Step::Open),
        (0..20_usize).prop_map(Step::Lines),
        prop_oneof![Just(0.0_f32), Just(8.0), Just(40.0)].prop_map(Step::Scroll),
        (24..60_u16, 10..20_u16).prop_map(|(cols, rows)| Step::Resize(cols, rows)),
    ]
}

fn tree(knobs: Knobs) -> NodeBuilder {
    let lines = knobs.lines;
    container()
        .layout(LayoutStyle::flex_column().fill_width().fill_height())
        .child(
            container()
                .paint_when(knobs.tint, |style| style.text_color(Token::Accent))
                .child(container().child(text("tinted deep"))),
        )
        .children_reactive(move || {
            let theme = if knobs.dark.get() {
                Theme::dark()
            } else {
                Theme::light()
            };
            vec![
                container()
                    .theme(theme)
                    .bg(Token::Surface)
                    .child(container().child(text("themed"))),
            ]
        })
        .child(
            container()
                .layout(LayoutStyle::block().fill_width().height(CELL * 2.0))
                .child(
                    container()
                        .transform(Transform2D::IDENTITY.origin(0.0, 0.0))
                        .animate(NumericAtomKind::TranslateX, knobs.shift)
                        .child(text(knobs.label)),
                ),
        )
        .child(
            container()
                .child(container().child(text("stable")))
                .child(modal(knobs.open).title("Kept")),
        )
        .child(
            scroll(container().children_reactive(move || {
                (0..lines.get())
                    .map(|line| container().child(text(format!("line {line}"))))
                    .collect::<Vec<_>>()
            }))
            .fill()
            .scroll_bind(knobs.offset),
        )
}

fn apply(knobs: Knobs, step: &Step, size: &mut (u16, u16)) {
    match step {
        Step::Tint => knobs.tint.update(|tint| *tint = !*tint),
        Step::Theme => knobs.dark.update(|dark| *dark = !*dark),
        Step::Shift(shift) => knobs.shift.set(*shift),
        Step::Label(label) => knobs.label.set(label.clone()),
        Step::Open => knobs.open.update(|open| *open = !*open),
        Step::Lines(lines) => knobs.lines.set(*lines),
        Step::Scroll(offset) => knobs.offset.set(*offset),
        Step::Resize(cols, rows) => *size = (*cols, *rows),
    }
}

fn frames(steps: &[Step], full: bool) -> Vec<String> {
    Runtime::scope("layers-oracle", || {
        let shared = knobs();
        let mut size = (48_u16, 16_u16);
        let mut host = Host::cells();
        host.mount(move || tree(shared));
        let mut screen = Side::mounted(host, size, Access::default().status);
        for _ in 0..2 {
            screen.frame(Vec::new(), size, full);
        }
        steps
            .iter()
            .map(|step| {
                apply(shared, step, &mut size);
                screen.frame(Vec::new(), size, full)
            })
            .collect()
    })
}

fn diverged(steps: &[Step]) -> Option<String> {
    let quick = frames(steps, false);
    let whole = frames(steps, true);
    quick
        .iter()
        .zip(&whole)
        .enumerate()
        .find(|(_, (quick, whole))| quick != whole)
        .map(|(index, (quick, whole))| {
            format!(
                "step {index} {:?} diverged\nincremental: {quick:?}\nfull:        {whole:?}",
                steps[index]
            )
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn layered_paint_matches_a_full_repaint(
        steps in proptest::collection::vec(step(), 1..24),
    ) {
        let mismatch = diverged(&steps);
        prop_assert!(mismatch.is_none(), "{}", mismatch.unwrap_or_default());
    }
}
