#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::float_cmp,
    clippy::cast_precision_loss
)]

use sigil::events::{
    EventRouter, ModifiersState, MouseButton, MouseButtons, PointerEventInit, PointerKind,
    PointerPhase, ShellEvent,
};
use sigil::geometry::Point;
use sigil::kernel::{Cx, NodeId};
use sigil::layout::{Dimension, LayoutEngine};
use sigil::prelude::*;
use sigil::renderer::{OffscreenTarget, WgpuRenderer};
use sigil::scene::emit_scene;
use sigil::style::Color;
use sigil_kittest::{CharMeasure, gpu::read_pixels};
use std::env::temp_dir;
use std::process;
use wgpu::TextureFormat;

const VIEWPORT: Size = Size::fixed(1440.0, 900.0);

const TEXT_MEASURE: CharMeasure = CharMeasure {
    char_width: 8.0,
    line_height: 16.8,
};

fn editor_view(name: &str, body: &str) -> NodeBuilder {
    let line_count = body.lines().count().max(1);
    let extent = (line_count as f32).mul_add(16.8, 20.0);
    let mono = text(body.to_owned())
        .font_size(14.0)
        .font_family(FontFamily::Mono);
    let code = container()
        .layout(LayoutStyle::flex_column().padding(10.0).width(2000.0))
        .child(mono);
    let column = LayoutStyle::flex_column()
        .grow(1.0)
        .width(Dimension::Percent(1.0));
    container()
        .layout(column)
        .child(
            container()
                .label(name.to_owned())
                .layout(LayoutStyle::flex_row().height(28.0).padding_x(10.0)),
        )
        .child(scroll(code).extent(extent).width(580.0).height(720.0))
}

fn recompute(layout: &mut LayoutEngine, cx: &mut Cx, root: NodeId) {
    layout.compute(cx, root, VIEWPORT).expect("layout computes");
}

const FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;

fn renderer_device(renderer: &mut WgpuRenderer) -> Option<(wgpu::Device, wgpu::Queue)> {
    *renderer = sigil_kittest::gpu::headless_renderer().expect("sigil-kittest")?;
    let device = renderer.device()?.clone();
    let queue = renderer.queue()?.clone();
    Some((device, queue))
}

const W: u32 = 1440;
const H: u32 = 900;

fn render(
    renderer: &mut WgpuRenderer,
    cx: &Cx,
    root: NodeId,
    dev: Option<&(wgpu::Device, wgpu::Queue)>,
    label: &str,
) {
    let scene = emit_scene(cx, root);
    let Some((device, queue)) = dev else {
        return;
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    renderer
        .render_command_scene_offscreen(
            OffscreenTarget {
                device,
                queue,
                view: &view,
                format: FORMAT,
                viewport: (W as f32, H as f32),
                scale: 1.0,
            },
            &scene,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        )
        .expect("render_command_scene_offscreen");

    let raw = read_pixels(device, queue, &texture, W, H).expect("sigil-kittest");
    let img = image::RgbaImage::from_raw(W, H, raw).expect("rgba");
    img.save(temp_dir().join(format!("swap_{}_{label}.png", process::id())))
        .expect("save");
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn clicking_second_file_swaps_editor_without_breaking() {
    sigil::reactive::Runtime::scope("editor-swap-probe", || {
        let files = [
            (
                "short.rs".to_owned(),
                "fn main() {}\nlet x = 1;\n".to_owned(),
            ),
            (
                "long.rs".to_owned(),
                (0..120)
                    .map(|i| format!("// line {i} of a much longer source file here"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        ];
        let selected = signal(0_usize);

        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().grow(1.0))
                .children_reactive(move || {
                    let (name, body) = &files[selected.get()];
                    vec![editor_view(name, body)]
                }),
        );

        let mut renderer = WgpuRenderer::new().expect("WgpuRenderer::new");
        let dev = renderer_device(&mut renderer);

        let mut layout = fresh_engine();
        recompute(&mut layout, &mut cx, root);
        render(&mut renderer, &cx, root, dev.as_ref(), "file-0-initial");

        selected.set(1);
        recompute(&mut layout, &mut cx, root);
        render(&mut renderer, &cx, root, dev.as_ref(), "file-1-after-swap");

        selected.set(0);
        recompute(&mut layout, &mut cx, root);
        render(&mut renderer, &cx, root, dev.as_ref(), "file-0-again");

        selected.set(1);
        recompute(&mut layout, &mut cx, root);
        render(&mut renderer, &cx, root, dev.as_ref(), "file-1-again");
    });
}

fn fresh_engine() -> LayoutEngine {
    LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE))
}

const fn pointer_move(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: None,
        phase: PointerPhase::Move,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}

const fn pointer_down(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        phase: PointerPhase::Down,
        buttons: MouseButtons::LEFT,
        modifiers: ModifiersState::empty(),
    }))
}

fn shell_root(selected: Signal<usize>, files: Vec<(String, String)>) -> NodeBuilder {
    let explorer_files = files.clone();
    container()
        .layout(LayoutStyle::flex_row().grow(1.0))
        .child(
            container()
                .layout(LayoutStyle::flex_column().width(248.0).padding(6.0))
                .children_reactive(move || {
                    let active = selected.get();
                    explorer_files
                        .iter()
                        .enumerate()
                        .map(|(idx, (name, _))| explorer_row(selected, idx, name, idx == active))
                        .collect::<Vec<_>>()
                }),
        )
        .child(
            container()
                .layout(LayoutStyle::flex_column().grow(1.0))
                .children_reactive(move || {
                    let (name, body) = &files[selected.get()];
                    vec![editor_view(name, body)]
                }),
        )
}

fn explorer_row(selected: Signal<usize>, idx: usize, name: &str, active: bool) -> NodeBuilder {
    let row = container()
        .role(Role::Button)
        .label(name.to_owned())
        .focusable(true)
        .layout(LayoutStyle::flex_row().height(22.0).padding_x(8.0));
    let row = if active { row.bg(Token::Accent) } else { row };
    row.pointer_down(move |ev| {
        selected.set(idx);
        ev.stop_propagation();
    })
    .child(text(name.to_owned()).nowrap())
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn routed_clicks_through_explorer_dont_break() {
    sigil::reactive::Runtime::scope("editor-swap-routed", || {
        let files = vec![
            ("a.rs".to_owned(), "fn a() {}\n".to_owned()),
            (
                "b.rs".to_owned(),
                (0..90)
                    .map(|i| format!("// row {i}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            ("c.rs".to_owned(), "const C: u8 = 3;\n".to_owned()),
        ];
        let selected = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(shell_root(selected, files));

        let mut renderer = WgpuRenderer::new().expect("renderer");
        let dev = renderer_device(&mut renderer);
        let mut layout = fresh_engine();
        let mut router = EventRouter::new();
        recompute(&mut layout, &mut cx, root);
        render(&mut renderer, &cx, root, dev.as_ref(), "routed-0");

        for (label_name, y) in [("routed-1", 39.0), ("routed-2", 61.0), ("routed-0b", 17.0)] {
            router.route(&mut cx, &layout, pointer_move(20.0, y));
            router.route(&mut cx, &layout, pointer_down(20.0, y));
            recompute(&mut layout, &mut cx, root);
            render(&mut renderer, &cx, root, dev.as_ref(), label_name);
        }
    });
}

const fn pointer_up(x: f32, y: f32) -> ShellEvent {
    ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
        position: Point::new(x, y),
        delta: Point::new(0.0, 0.0),
        kind: PointerKind::Mouse,
        button: Some(MouseButton::Left),
        phase: PointerPhase::Up,
        buttons: MouseButtons::empty(),
        modifiers: ModifiersState::empty(),
    }))
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
fn captured_scroll_thumb_survives_editor_rebuild() {
    sigil::reactive::Runtime::scope("editor-swap-capture", || {
        let files = vec![
            (
                "a.rs".to_owned(),
                (0..90)
                    .map(|i| format!("// a {i}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            (
                "b.rs".to_owned(),
                (0..90)
                    .map(|i| format!("// b {i}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        ];
        let selected = signal(0_usize);
        let mut cx = Cx::new_root();
        let root = cx.add(shell_root(selected, files));
        let mut layout = fresh_engine();
        let mut router = EventRouter::new();
        recompute(&mut layout, &mut cx, root);

        for tx in [820.0_f32, 824.0, 816.0, 818.0] {
            for ty in [34.0_f32, 40.0, 30.0, 50.0] {
                router.route(&mut cx, &layout, pointer_move(tx, ty));
                router.route(&mut cx, &layout, pointer_down(tx, ty));
                router.route(&mut cx, &layout, pointer_move(tx, ty + 40.0));
            }
        }

        selected.set(1);
        recompute(&mut layout, &mut cx, root);

        router.route(&mut cx, &layout, pointer_move(824.0, 120.0));
        router.route(&mut cx, &layout, pointer_up(824.0, 120.0));
        recompute(&mut layout, &mut cx, root);
        for s in [0_usize, 1, 0, 1] {
            router.route(&mut cx, &layout, pointer_down(824.0, 40.0));
            selected.set(s);
            recompute(&mut layout, &mut cx, root);
            router.route(&mut cx, &layout, pointer_up(824.0, 120.0));
        }
    });
}

#[test]
#[ignore = "needs a wgpu adapter; run with --ignored on GPU hardware (see README verification)"]
#[allow(clippy::too_many_lines)]
fn deep_reactive_scroll_switch_stays_positioned() {
    sigil::reactive::Runtime::scope("deep-reactive-scroll", || {
        let tall = signal(false);
        let pane = container()
            .layout(LayoutStyle::flex_column().grow(1.0))
            .children_reactive(move || {
                let lines = if tall.get() { 200 } else { 2 };
                let body = (0..lines)
                    .map(|i| format!("line {i} of source"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let extent = (lines as f32).mul_add(16.8, 20.0);
                let code = container()
                    .layout(LayoutStyle::flex_column().padding(10.0).width(2000.0))
                    .child(text(body));
                vec![scroll(code).extent(extent).width(580.0).height(720.0)]
            });
        let mut filler = container().layout(LayoutStyle::flex_column());
        for i in 0..120 {
            filler = filler.child(text(format!("filler row {i}")).nowrap());
        }
        let tree = container()
            .layout(LayoutStyle::flex_column().grow(1.0))
            .child(filler)
            .child(
                container()
                    .layout(LayoutStyle::flex_row().grow(1.0))
                    .child(
                        container()
                            .layout(LayoutStyle::flex_column().width(248.0))
                            .child(text("SIDEBAR".to_owned()).nowrap()),
                    )
                    .child(pane),
            );
        let mut cx = Cx::new_root();
        let root = cx.add(tree);
        let mut layout = fresh_engine();
        recompute(&mut layout, &mut cx, root);

        tall.set(true);
        recompute(&mut layout, &mut cx, root);

        let mut escaped = Vec::new();
        cx.visit_preorder(root, |id, _node| {
            if let Some(res) = layout.result(id) {
                let r = res.rect();
                if r.height > 900.0 && r.width > 1500.0 && r.origin.x < 60.0 && r.origin.y < 60.0 {
                    escaped.push((r.origin.x, r.origin.y, r.width, r.height));
                }
            }
        });
        assert!(
            escaped.is_empty(),
            "tall scroll content escaped to the top-left (incremental containment-root bug): {escaped:?}"
        );
    });
}
