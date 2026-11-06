#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::cast_precision_loss,
    reason = "pixel sizes and sample counts are small"
)]

use std::time::{Duration, Instant};

use sigil::Host;
use sigil::app::{FrameHooks, set_theme};
use sigil::geometry::PhysicalSize;
use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::renderer::CpuRenderer;
use sigil::renderer::capture::{ImageRenderer, ImageTarget, render_to_image};
use sigil::scene::paint_backdrop;
use sigil::style::Color;
use sigil_kittest::gpu::headless_renderer;
use sigil_kittest::parity::{Frame, gpu_rgba};

const VIEW: (f32, f32) = (900.0, 6000.0);
const DEVICE: (u32, u32) = (900, 6000);
const ROUNDS: usize = 15;

const FLOWS: [&str; 7] = [
    "flowchart TD\n  A[Request] --> B{Cached?}\n  B -->|yes| C[Serve]\n  B -->|no| D[Fetch]\n  D --> C\n",
    "sequenceDiagram\n  Alice->>Bob: Hello\n  Bob-->>Alice: Hi\n  Alice->>Bob: Bye\n",
    "flowchart LR\n  Parse --> Lower --> Paint --> Present\n",
    "classDiagram\n  class Scene\n  class Path\n  Scene --> Path\n",
    "stateDiagram-v2\n  [*] --> Idle\n  Idle --> Busy\n  Busy --> Idle\n  Busy --> [*]\n",
    "flowchart TD\n  subgraph Host\n    W[Window] --> R[Renderer]\n  end\n  R --> G[GPU]\n",
    "pie title Share\n  \"A\" : 40\n  \"B\" : 35\n  \"C\" : 25\n",
];
const GRAPHS: [&str; 7] = [
    "digraph { a -> b; b -> c; c -> a; }",
    "digraph { rankdir=LR; parse -> lower -> paint; lower -> cache; }",
    "graph { x -- y; y -- z; z -- x; x -- w; }",
    "digraph { node [shape=box]; app -> host; host -> renderer; renderer -> gpu; renderer -> cpu; }",
    "digraph { a -> {b c d}; b -> e; c -> e; d -> e; }",
    "digraph { s [shape=circle]; t [shape=doublecircle]; s -> t [label=go]; }",
    "digraph { 1 -> 2 -> 3 -> 4 -> 5; 1 -> 5; }",
];
const FORMULAS: [&str; 6] = [
    r"e^{i\pi} + 1 = 0",
    r"\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}",
    r"\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}",
    r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
    r"f(x) = \frac{1}{\sigma\sqrt{2\pi}} e^{-\frac{(x-\mu)^2}{2\sigma^2}}",
    r"\nabla \cdot \mathbf{E} = \frac{\rho}{\varepsilon_0}",
];

fn transcript() -> NodeBuilder {
    let mut list = column();
    for source in FLOWS {
        list = list
            .child(text("A diagram:"))
            .child(mermaid(signal(source.to_string())));
    }
    for source in GRAPHS {
        list = list
            .child(text("A graph:"))
            .child(graphviz(signal(source.to_string())));
    }
    for source in FORMULAS {
        list = list
            .child(text("A formula:"))
            .child(math(signal(source.to_string())));
    }
    list
}

enum Painter {
    Cpu(CpuRenderer),
    Gpu(Box<sigil::renderer::WgpuRenderer>),
}

impl Painter {
    fn paint(&mut self, host: &mut Host) {
        let _ = host.frame(VIEW, 1.0 / 60.0, FrameHooks::default());
        let mut scene = host.emit(VIEW);
        paint_backdrop(&mut scene, host.theme(), VIEW);
        let clear = Color::rgba(0.0, 0.0, 0.0, 1.0);
        let size = PhysicalSize::new(DEVICE.0, DEVICE.1);
        match self {
            Self::Cpu(renderer) => {
                let target = ImageTarget {
                    size,
                    viewport: VIEW,
                    clear,
                };
                let _ = render_to_image(ImageRenderer::Cpu(renderer), &scene, target).expect("cpu");
            }
            Self::Gpu(renderer) => {
                let frame = Frame {
                    scene: &scene,
                    viewport: VIEW,
                    size,
                    clear,
                };
                let _ = gpu_rgba(renderer, &frame).expect("gpu");
            }
        }
    }
}

fn median(mut samples: Vec<Duration>) -> f64 {
    samples.sort();
    samples[samples.len() / 2].as_secs_f64() * 1000.0
}

fn measure(name: &str, mut painter: Painter) {
    Owner::scope("svg_picture_perf", || {
        let mut host = Host::new(HostMode::Pixel);
        set_theme(Theme::light());
        host.mount(transcript());
        for _ in 0..3 {
            painter.paint(&mut host);
        }
        let mut steady = Vec::new();
        let mut switch = Vec::new();
        for round in 0..ROUNDS {
            let start = Instant::now();
            painter.paint(&mut host);
            steady.push(start.elapsed());
            set_theme(if round % 2 == 0 {
                Theme::dark()
            } else {
                Theme::light()
            });
            let start = Instant::now();
            painter.paint(&mut host);
            switch.push(start.elapsed());
        }
        println!(
            "bench {name} steady_ms {:.3} theme_switch_ms {:.3}",
            median(steady),
            median(switch)
        );
    });
}

#[test]
#[ignore = "manual: release frame and theme-switch timing for a transcript of 20 diagrams"]
fn a_transcript_of_twenty_diagrams_paints_and_rethemes() {
    measure("cpu", Painter::Cpu(CpuRenderer::new()));
    if let Some(renderer) = headless_renderer().expect("adapter") {
        measure("gpu", Painter::Gpu(Box::new(renderer)));
    }
}
