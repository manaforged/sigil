#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic)]

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::kernel::{Cx, NodeBuilder, NodeId, container};
use sigil::layout::{LayoutBuilderExt, LayoutStyle};
use sigil::reactive::{Runtime, signal};
use sigil::style::{
    ColorKey, Elevation, FontFamily, MillisU16, NodeStyleExt, Paint, Px, Radius, RenderStyle,
    Rgba8, StateStyle, StyleAtom, StyleBinding, StyleEngine, StyleState, Theme, Token,
    TransitionId,
};

static APPLY_ATOMS: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::Accent)),
    StyleAtom::TextThemeRef(ColorKey::Token(Token::OnSurface)),
    StyleAtom::BorderRadius(Px(6)),
    StyleAtom::Opacity(10_000),
];
static THEME_SWAP_ATOMS: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::Accent)),
    StyleAtom::TextThemeRef(ColorKey::Token(Token::OnAccent)),
    StyleAtom::BorderRadius(Px(6)),
    StyleAtom::Opacity(10_000),
];
static STATE_REST: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef(ColorKey::Token(
    Token::Surface,
))];
static STATE_FOCUS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef(ColorKey::Token(
    Token::SurfaceHover,
))];
static STATE_HOVER: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef(ColorKey::Token(
    Token::SurfaceHover,
))];
static STATE_PRESS: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::SurfaceHover)),
    StyleAtom::Opacity(8_800),
];
static STATE_DISABLED: &[StyleAtom] = &[
    StyleAtom::BackgroundThemeRef(ColorKey::Token(Token::SurfaceHover)),
    StyleAtom::Opacity(5_000),
];

fn paint_node() -> NodeBuilder {
    container()
        .layout(LayoutStyle::default().padding(16.0).gap(16.0))
        .bg(Token::Surface)
        .text_color(Token::OnSurface)
        .border_color(Token::Border)
        .border_w(1)
        .rounded(Radius::Md)
        .shadow(Elevation::Sm)
        .opacity(0.8)
        .font_size(14)
        .font_weight(400)
        .font_family(FontFamily::Sans)
        .line_height(16)
        .transition(TransitionId(1))
        .duration(MillisU16(150))
}

fn paint_resolve_atoms(c: &mut Criterion) {
    c.bench_function("paint_resolve_atoms", |b| {
        b.iter(|| {
            let mut cx = Cx::new_root();
            let id = cx.add(black_box(paint_node()));
            black_box(binding_len(&cx, id));
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

fn paint_override_last_wins(c: &mut Criterion) {
    c.bench_function("paint_override_last_wins", |b| {
        b.iter(|| {
            let mut cx = Cx::new_root();
            let node = paint_node()
                .bg(Token::Accent)
                .text_color(Token::OnAccent)
                .rounded(Radius::Lg)
                .shadow(Elevation::Lg)
                .opacity(1.0)
                .font_size(18)
                .font_weight(700)
                .line_height(24)
                .transition(TransitionId(2))
                .duration(MillisU16(300));
            let id = cx.add(black_box(node));
            black_box(binding_len(&cx, id));
        });
    });
}

fn visible_when_toggle(c: &mut Criterion) {
    Runtime::scope("sigil_style_bench_visible_toggle", || {
        let active = signal(false);
        let mut cx = Cx::new_root();
        let id = cx.add(container().visible_when(active));

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
        c.bench_function("visible_when_toggle", |b| {
            b.iter(|| {
                next = !next;
                active.set(black_box(next));
                black_box(reactive_len(&cx, id));
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
        paint_resolve_atoms,
        style_engine_apply_1000_nodes,
        paint_override_last_wins,
        visible_when_toggle,
        state_style_resolve_1000_widgets,
        theme_swap_10_000_bound_nodes
}
criterion_main!(benches);

fn binding_len(cx: &Cx, id: NodeId) -> usize {
    cx.get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .map_or(0, |binding| {
            binding.static_slice().len() + binding.dynamic_slice().len()
        })
}

fn reactive_len(cx: &Cx, id: NodeId) -> usize {
    cx.get(id)
        .and_then(|node| node.get_extension::<StyleBinding>())
        .map_or(0, |binding| binding.reactive_channels().flatten().count())
}
