//! Headless PNG capture of the site catalog widgets.
//!
//! Renders every catalog widget through [`Host::scene`] and the offscreen
//! renderer path, then writes `showcase/catalog/{light,dark}/{name}.png`
//! at 2x scale. Run from the workspace root: `cargo run -p catalog`.

#![allow(missing_docs)]
#![allow(clippy::print_stdout)]

use std::error::Error;
use std::path::{Path, PathBuf};
use std::{fs, io};

use sigil::prelude::*;
use sigil::render::Color;
use sigil_renderer::capture::{CaptureExtent, FORMAT, capture_texture, read_texture};
use sigil_renderer::{OffscreenTarget, WgpuRenderer};

const SCALE: f32 = 2.0;
const WIDTH: f32 = 440.0;

struct Entry {
    name: &'static str,
    height: f32,
    build: fn() -> Box<dyn IntoNode>,
}

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: WgpuRenderer,
}

type ThemeSpec = (&'static str, fn() -> Theme);

#[allow(
    clippy::too_many_lines,
    reason = "pure data table — one entry per catalog widget, no logic"
)]
fn entries() -> Vec<Entry> {
    vec![
        Entry {
            name: "button",
            height: 120.0,
            build: || {
                node(
                    row()
                        .child(button("Save changes").primary())
                        .child(button("Discard"))
                        .child(button("Ghost").ghost())
                        .child(button("Delete").destructive()),
                )
            },
        },
        Entry {
            name: "button_primary",
            height: 120.0,
            build: || node(button("Save changes").primary()),
        },
        Entry {
            name: "text_field",
            height: 120.0,
            build: || node(text_field(signal("Aldous Harding".to_owned()))),
        },
        Entry {
            name: "dropdown",
            height: 120.0,
            build: || {
                node(dropdown(
                    signal(0),
                    vec![
                        "Normal".to_owned(),
                        "Hard".to_owned(),
                        "Nightmare".to_owned(),
                    ],
                ))
            },
        },
        Entry {
            name: "checkbox",
            height: 120.0,
            build: || node(checkbox(signal(true)).label("Enable notifications")),
        },
        Entry {
            name: "radio_group",
            height: 170.0,
            build: || {
                node(
                    radio_group(signal("beta".to_owned()))
                        .option("stable", "Stable")
                        .option("beta", "Beta")
                        .option("nightly", "Nightly"),
                )
            },
        },
        Entry {
            name: "toggle",
            height: 120.0,
            build: || node(toggle(signal(true)).label("Notifications")),
        },
        Entry {
            name: "slider",
            height: 120.0,
            build: || node(slider(signal(0.65), 0.0..=1.0)),
        },
        Entry {
            name: "tabs",
            height: 120.0,
            build: || {
                node(tabs(
                    signal(0),
                    vec![
                        "General".to_owned(),
                        "Appearance".to_owned(),
                        "Keymap".to_owned(),
                    ],
                ))
            },
        },
        Entry {
            name: "chips",
            height: 120.0,
            build: || {
                node(
                    row()
                        .child(chip("NEUTRAL", Tone::Neutral))
                        .child(chip("INFO", Tone::Info))
                        .child(chip("SUCCESS", Tone::Success))
                        .child(chip("WARN", Tone::Warn))
                        .child(chip("ERROR", Tone::Error)),
                )
            },
        },
        Entry {
            name: "progress",
            height: 120.0,
            build: || node(progress(signal(0.65), 1.0)),
        },
        Entry {
            name: "gauge",
            height: 140.0,
            build: || node(gauge(signal(0.72))),
        },
    ]
}

fn row() -> NodeBuilder {
    container().layout(
        LayoutStyle::flex_row()
            .gap(12.0)
            .align_items(AlignItems::Center),
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut gpu = Gpu::new()?;
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("showcase/catalog");
    let themes: [ThemeSpec; 2] = [("light", Theme::default), ("dark", Theme::dark)];
    for (theme_name, theme) in themes {
        let dir = out.join(theme_name);
        fs::create_dir_all(&dir)?;
        for entry in entries() {
            let path = dir.join(format!("{}.png", entry.name));
            capture(&mut gpu, theme(), &entry, &path)?;
            let size = path.metadata()?.len();
            println!("{} {size} bytes", path.display());
        }
    }
    Ok(())
}

fn capture(gpu: &mut Gpu, theme: Theme, entry: &Entry, path: &Path) -> Result<(), Box<dyn Error>> {
    let viewport = (WIDTH, entry.height);
    let mut result = Ok(());
    Runtime::scope("catalog", || {
        result = render_entry(gpu, theme, entry, viewport, path);
    });
    result
}

fn render_entry(
    gpu: &mut Gpu,
    theme: Theme,
    entry: &Entry,
    viewport: (f32, f32),
    path: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut host = Host::new();
    let tree = (entry.build)();
    host.build(|cx| {
        cx.add(
            container()
                .theme(theme)
                .class("bg-surface")
                .layout(
                    LayoutStyle::flex_column()
                        .fill()
                        .padding(28.0)
                        .align_items(AlignItems::Start)
                        .justify_content(JustifyContent::Center),
                )
                .child(tree),
        )
    });
    let scene = host.scene(viewport)?;
    let extent = capture_extent(viewport);
    let texture = capture_texture(&gpu.device, extent);
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    gpu.renderer.render_command_scene_offscreen(
        OffscreenTarget {
            device: &gpu.device,
            queue: &gpu.queue,
            view: &view,
            format: FORMAT,
            viewport,
            scale: SCALE,
        },
        &scene,
        Color::rgba(0.0, 0.0, 0.0, 1.0),
    )?;
    let pixels = read_texture(&gpu.device, &gpu.queue, &texture, extent)?;
    image::save_buffer(
        path,
        &pixels,
        extent.width,
        extent.height,
        image::ColorType::Rgba8,
    )?;
    Ok(())
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "logical sizes are small positive constants; 2x stays far below limits"
)]
fn capture_extent(viewport: (f32, f32)) -> CaptureExtent {
    CaptureExtent {
        width: (viewport.0 * SCALE).round() as u32,
        height: (viewport.1 * SCALE).round() as u32,
    }
}

impl Gpu {
    fn new() -> Result<Self, Box<dyn Error>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|_| io::Error::other("no GPU adapter available for offscreen capture"))?;
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("sigil catalog capture device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&descriptor))?;
        Ok(Self {
            device,
            queue,
            renderer: WgpuRenderer::new()?,
        })
    }
}
