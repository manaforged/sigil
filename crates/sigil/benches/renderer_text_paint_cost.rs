#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::panic)]
#![allow(clippy::unwrap_used)]

use criterion::{Criterion, criterion_group, criterion_main};
use sigil::hal::Size;
use sigil::kernel::prelude::*;
use sigil::layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle};
use sigil::scene::{SceneCommand, emit_scene};
use sigil::text::{OwnedTextEngine, TextConfig};
use sigil_kittest::FixedMeasure;
use std::hint::black_box;

fn text_paint_cost(c: &mut Criterion) {
    let scene = text_scene();
    let runs = scene
        .commands
        .iter()
        .filter_map(|command| match command {
            SceneCommand::GlyphRun(draw) => Some(draw),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(runs.len(), 50);

    let engine = OwnedTextEngine::new(TextConfig::default());
    warm_interim_cache(&engine, &runs);

    c.bench_function("text_paint_cost/dr025_interim_50_text_nodes", |b| {
        b.iter(|| {
            let mut glyphs = 0_usize;
            for draw in &runs {
                let run = &draw.run;
                let paragraph = engine
                    .layout_paragraph_styled(&run.text, Some(run.rect.width.max(1.0)), &run.style)
                    .expect("bench text layout succeeds");
                glyphs = glyphs.saturating_add(paragraph.rendered_glyphs().len());
            }
            black_box(glyphs);
        });
    });

    c.bench_function("text_paint_cost/phase11_positioned_50_text_nodes", |b| {
        b.iter(|| {
            let mut glyphs = 0_usize;
            for draw in &runs {
                glyphs = glyphs.saturating_add(draw.run.positioned_glyphs().len());
            }
            black_box(glyphs);
        });
    });
}

fn warm_interim_cache(engine: &OwnedTextEngine, runs: &[&sigil::scene::DrawGlyphRun]) {
    for draw in runs {
        let run = &draw.run;
        let _ = engine
            .layout_paragraph_styled(&run.text, Some(run.rect.width.max(1.0)), &run.style)
            .expect("bench text layout succeeds");
    }
}

fn text_scene() -> sigil::scene::CommandScene {
    let mut cx = Cx::new_root();
    let mut root = container().layout(LayoutStyle::flex_column().fixed_size(360.0, 1_200.0));
    for index in 0..50 {
        root = root.child(
            text(format!("Text node {index:02}"))
                .layout(LayoutStyle::block().fixed_size(280.0, 22.0)),
        );
    }
    let root = cx.add(root);
    LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE))
        .compute(&mut cx, root, Size::fixed(360.0, 1_200.0))
        .expect("bench scene layout computes");
    emit_scene(&cx, root)
}

const TEXT_MEASURE: FixedMeasure = FixedMeasure {
    width: 160.0,
    height: 18.0,
};

criterion_group!(benches, text_paint_cost);
criterion_main!(benches);
