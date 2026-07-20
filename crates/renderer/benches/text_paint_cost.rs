//! Text paint cost before and after carrying positioned glyphs in commands.

#![allow(missing_docs)]
#![allow(clippy::panic)]
#![allow(clippy::unwrap_used)]

use ahash as _;
use bytemuck as _;
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use half as _;
use image as _;
use pollster as _;
use proptest as _;
use raw_window_handle as _;
use sigil_hal::Size;
use sigil_kernel::{TextContent, prelude::*};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};
use sigil_renderer::{SceneCommand, emit_scene};
use sigil_shell as _;
use sigil_style as _;
use sigil_text::{OwnedTextEngine, TextConfig};
use skrifa as _;
use smallvec as _;
use tracing as _;
use wgpu as _;
#[cfg(windows)]
use windows_core as _;

use sigil_icons as _;
use zeno as _;

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

fn warm_interim_cache(engine: &OwnedTextEngine, runs: &[&sigil_renderer::DrawGlyphRun]) {
    for draw in runs {
        let run = &draw.run;
        let _ = engine
            .layout_paragraph_styled(&run.text, Some(run.rect.width.max(1.0)), &run.style)
            .expect("bench text layout succeeds");
    }
}

fn text_scene() -> sigil_renderer::CommandScene {
    let mut cx = Cx::new_root();
    let mut root = container().layout(LayoutStyle::flex_column().fixed_size(360.0, 1_200.0));
    for index in 0..50 {
        root = root.child(
            text(format!("Text node {index:02}"))
                .layout(LayoutStyle::block().fixed_size(280.0, 22.0)),
        );
    }
    let root = cx.add(root);
    LayoutEngine::with_text_measure(Box::new(StubMeasure))
        .compute(&mut cx, root, Size::fixed(360.0, 1_200.0))
        .expect("bench scene layout computes");
    emit_scene(&cx, root)
}

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        sigil_hal::LogicalSize::new(160.0, 18.0)
    }
}

criterion_group!(benches, text_paint_cost);
criterion_main!(benches);
