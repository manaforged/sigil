//! Style operation benchmarks.

#![allow(missing_docs, clippy::panic)]

use std::hint::black_box;
use std::time::Duration;

use allocation_counter as _;
use criterion::{Criterion, criterion_group, criterion_main};
use phf as _;
use sigil_hal as _;
use sigil_kernel::{Cx, NodeId, container};
use sigil_reactive::{Runtime, signal};
use sigil_style::{
    AlignKind, Dimension, DisplayKind, FlexDir, JustifyKind, MillisU16, NodeStyleExt, Px,
    RenderStyle, Rgba8, StateStyle, StyleAtom, StyleBinding, StyleEngine, StyleState, Theme,
    TransitionId, classes, merge_classes,
};
use sigil_style_macros as _;
use smallvec as _;
use strsim as _;

static CLASS_50: &[StyleAtom] = classes!(
    "p-4 px-4 py-4 pt-4 pr-4 pb-4 pl-4 m-4 mx-4 my-4 mt-4 mr-4 mb-4 ml-4 gap-4 w-4 h-4 w-full h-full w-auto h-auto flex block hidden flex-row flex-col flex-row-reverse flex-col-reverse items-start items-center items-end items-stretch justify-start justify-center justify-end justify-between justify-around rounded rounded-sm rounded-md rounded-lg rounded-full bg-accent bg-border bg-surface text-xs text-sm text-base text-lg text-xl"
);

static APPLY_ATOMS: &[StyleAtom] = classes!("bg-accent text-on-surface rounded-md opacity-100");
static REACTIVE_OFF: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static REACTIVE_ON: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];
static THEME_SWAP_ATOMS: &[StyleAtom] = classes!("bg-accent text-on-accent rounded-md opacity-100");
static STATE_REST: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static STATE_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static STATE_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface-hover")];
static STATE_PRESS: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Opacity(8_800),
];
static STATE_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef("surface-hover"),
    StyleAtom::Opacity(5_000),
];

const DEFAULTS_32: [StyleAtom; 32] = [
    StyleAtom::PaddingTop(Px(1)),
    StyleAtom::PaddingRight(Px(2)),
    StyleAtom::PaddingBottom(Px(3)),
    StyleAtom::PaddingLeft(Px(4)),
    StyleAtom::MarginTop(Px(5)),
    StyleAtom::MarginRight(Px(6)),
    StyleAtom::MarginBottom(Px(7)),
    StyleAtom::MarginLeft(Px(8)),
    StyleAtom::Gap(Px(9)),
    StyleAtom::Width(Dimension::Px(Px(10))),
    StyleAtom::Height(Dimension::Px(Px(11))),
    StyleAtom::MinWidth(Dimension::Px(Px(12))),
    StyleAtom::MaxWidth(Dimension::Px(Px(13))),
    StyleAtom::Display(DisplayKind::Block),
    StyleAtom::FlexDirection(FlexDir::Column),
    StyleAtom::AlignItems(AlignKind::Start),
    StyleAtom::JustifyContent(JustifyKind::Start),
    StyleAtom::FlexGrow(1_000),
    StyleAtom::FlexShrink(1_000),
    StyleAtom::BackgroundThemeRef("surface"),
    StyleAtom::TextThemeRef("on-surface"),
    StyleAtom::BorderRadius(Px(2)),
    StyleAtom::Opacity(8_000),
    StyleAtom::FontSize(Px(14)),
    StyleAtom::FontWeight(400),
    StyleAtom::LineHeight(Px(16)),
    StyleAtom::Transition(TransitionId(1)),
    StyleAtom::TransitionDuration(MillisU16(150)),
    StyleAtom::UnknownThemeRef("shadow-sm"),
    StyleAtom::PaddingAll(Px(1)),
    StyleAtom::PaddingX(Px(1)),
    StyleAtom::MarginAll(Px(1)),
];

const OVERRIDES_32: [StyleAtom; 32] = [
    StyleAtom::PaddingTop(Px(11)),
    StyleAtom::PaddingRight(Px(12)),
    StyleAtom::PaddingBottom(Px(13)),
    StyleAtom::PaddingLeft(Px(14)),
    StyleAtom::MarginTop(Px(15)),
    StyleAtom::MarginRight(Px(16)),
    StyleAtom::MarginBottom(Px(17)),
    StyleAtom::MarginLeft(Px(18)),
    StyleAtom::Gap(Px(19)),
    StyleAtom::Width(Dimension::Px(Px(20))),
    StyleAtom::Height(Dimension::Px(Px(21))),
    StyleAtom::MinWidth(Dimension::Px(Px(22))),
    StyleAtom::MaxWidth(Dimension::Px(Px(23))),
    StyleAtom::Display(DisplayKind::Flex),
    StyleAtom::FlexDirection(FlexDir::Row),
    StyleAtom::AlignItems(AlignKind::Center),
    StyleAtom::JustifyContent(JustifyKind::Between),
    StyleAtom::FlexGrow(2_000),
    StyleAtom::FlexShrink(0),
    StyleAtom::BackgroundThemeRef("accent"),
    StyleAtom::TextThemeRef("on-accent"),
    StyleAtom::BorderRadius(Px(8)),
    StyleAtom::Opacity(10_000),
    StyleAtom::FontSize(Px(18)),
    StyleAtom::FontWeight(700),
    StyleAtom::LineHeight(Px(24)),
    StyleAtom::Transition(TransitionId(2)),
    StyleAtom::TransitionDuration(MillisU16(300)),
    StyleAtom::UnknownThemeRef("shadow-xl"),
    StyleAtom::PaddingAll(Px(15)),
    StyleAtom::PaddingX(Px(16)),
    StyleAtom::MarginAll(Px(17)),
];

fn classes_resolve_50_atoms(c: &mut Criterion) {
    c.bench_function("classes_resolve_50_atoms", |b| {
        b.iter(|| {
            let binding = StyleBinding::static_atoms(black_box(CLASS_50));
            black_box(binding.static_slice().len());
        });
    });
}

fn style_engine_apply_1000_nodes(c: &mut Criterion) {
    let theme = Theme::default();
    let engine = StyleEngine::new(&theme);
    let mut render_styles = vec![RenderStyle::default(); 1_000];

    c.bench_function("style_engine_apply_1000_nodes", |b| {
        b.iter(|| {
            for render in &mut render_styles {
                *render = RenderStyle::default();
                engine.apply(black_box(APPLY_ATOMS), render);
            }
            black_box(&render_styles);
        });
    });
}

fn merge_classes_worst_case(c: &mut Criterion) {
    let component_id = 7_007;
    let _warm = merge_classes(&DEFAULTS_32, &OVERRIDES_32, component_id);

    c.bench_function("merge_classes_worst_case", |b| {
        b.iter(|| {
            let merged = merge_classes(
                black_box(&DEFAULTS_32),
                black_box(&OVERRIDES_32),
                component_id,
            );
            black_box(merged.len());
        });
    });
}

fn style_reactive_toggle(c: &mut Criterion) {
    Runtime::scope("sigil_style_bench_reactive_toggle", || {
        let active = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(container().style_reactive(move || {
            if active.get() {
                REACTIVE_ON
            } else {
                REACTIVE_OFF
            }
        }));

        active.set(true);
        active.set(false);
        let allocations = allocation_counter::measure(|| {
            for index in 0..100 {
                active.set(index % 2 == 0);
            }
        });
        assert_eq!(allocations.count_total, 0, "{allocations:?}");
        active.set(false);

        let mut next = false;
        c.bench_function("style_reactive_toggle", |b| {
            b.iter(|| {
                next = !next;
                active.set(black_box(next));
                black_box(reactive_slice(&cx, id));
            });
        });
    });
}

fn state_style_resolve_1000_widgets(c: &mut Criterion) {
    let style = StateStyle::new(
        STATE_REST,
        STATE_FOCUS,
        STATE_HOVER,
        STATE_PRESS,
        STATE_DISABLED,
    );
    let states = (0..1_000)
        .map(|index| StyleState {
            disabled: index % 17 == 0,
            hovered: index % 3 == 0,
            pressed: index % 11 == 0,
            focused: index % 5 == 0,
        })
        .collect::<Vec<_>>();

    c.bench_function("state_style_resolve_1000_widgets", |b| {
        b.iter(|| {
            for state in &states {
                black_box(style.resolve(black_box(*state)));
            }
        });
    });
}

fn theme_swap_10_000_bound_nodes(c: &mut Criterion) {
    let default_theme = Theme::default();
    let accent_theme = Theme {
        accent: Rgba8 {
            r: 220,
            g: 38,
            b: 38,
            a: 255,
        },
        accent_hover: Rgba8 {
            r: 185,
            g: 28,
            b: 28,
            a: 255,
        },
        on_accent: Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        ..Theme::default()
    };
    let mut render_styles = vec![RenderStyle::default(); 10_000];
    let mut use_accent_theme = false;

    c.bench_function("theme_swap_10_000_bound_nodes", |b| {
        b.iter(|| {
            use_accent_theme = !use_accent_theme;
            let theme = if use_accent_theme {
                &accent_theme
            } else {
                &default_theme
            };
            let engine = StyleEngine::new(theme);
            for render in &mut render_styles {
                *render = RenderStyle::default();
                engine.apply(black_box(THEME_SWAP_ATOMS), render);
            }
            black_box(&render_styles);
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(3))
        .sample_size(30);
    targets =
        classes_resolve_50_atoms,
        style_engine_apply_1000_nodes,
        merge_classes_worst_case,
        style_reactive_toggle,
        state_style_resolve_1000_widgets,
        theme_swap_10_000_bound_nodes
}
criterion_main!(benches);

fn reactive_slice(cx: &Cx, id: NodeId) -> &'static [StyleAtom] {
    cx.get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .map_or(&[], StyleBinding::reactive_slice)
}
