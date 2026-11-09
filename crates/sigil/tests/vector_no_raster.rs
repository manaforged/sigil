#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::cast_precision_loss,
    reason = "pixel sizes and step counts are small"
)]

use std::time::{Duration, Instant};

use sigil::prelude::*;
use sigil_kittest::{Harness, Query};

const VIEW: (u32, u32) = (900, 700);
const WAIT: Duration = Duration::from_secs(20);
const BARS: &str = r#"{"data":{"values":[{"a":"A","b":28},{"a":"B","b":55}]},"mark":"bar","encoding":{"x":{"field":"a","type":"nominal"},"y":{"field":"b","type":"quantitative"}}}"#;

#[derive(Clone, Copy, Debug)]
enum Producer {
    Mermaid,
    Graphviz,
    Math,
    VegaLite,
    Checkbox,
    Icon,
    Path,
    Chart,
}

impl Producer {
    const ALL: [Self; 8] = [
        Self::Mermaid,
        Self::Graphviz,
        Self::Math,
        Self::VegaLite,
        Self::Checkbox,
        Self::Icon,
        Self::Path,
        Self::Chart,
    ];

    fn node(self) -> NodeBuilder {
        let root = container();
        match self {
            Self::Mermaid => root.child(mermaid(signal(
                "flowchart LR\n  A[Start] --> B[End]\n".to_string(),
            ))),
            Self::Graphviz => {
                root.child(graphviz(signal("digraph { a -> b; a -> c; }".to_string())))
            }
            Self::Math => root.child(math(signal(r"x^2 + \frac{a}{b}".to_string()))),
            Self::VegaLite => root.child(markdown(signal(format!("```vega-lite\n{BARS}\n```")))),
            Self::Checkbox => root.child(checkbox(signal(true)).label("on")),
            Self::Icon => root.child(icon(Icon::Check)).child(icon(Icon::Search)),
            Self::Path => root.child(
                path(24, 24)
                    .move_to(4.0, 12.0)
                    .line_to(20.0, 12.0)
                    .stroke(Token::OnSurface, 2.0),
            ),
            Self::Chart => root.child(
                chart(vec![1.0, 6.0, 2.0, 8.0])
                    .line()
                    .width(300.0)
                    .height(150.0),
            ),
        }
    }
}

struct Drawn {
    images: usize,
    sprites: usize,
    paths: usize,
}

fn drawn(producer: Producer) -> Drawn {
    let mut ui = Harness::new(move || column().bg(Token::Surface).child(producer.node()))
        .pixels(VIEW.0, VIEW.1);
    let start = Instant::now();
    ui.settle();
    while ui.query(&Query::text("Rendering chart")).is_some() {
        assert!(
            start.elapsed() < WAIT,
            "{producer:?} still rendering after {WAIT:?}"
        );
        std::thread::sleep(Duration::from_millis(5));
        ui.settle();
    }
    let scene = format!("{:?}", ui.host().emit((VIEW.0 as f32, VIEW.1 as f32)));
    Drawn {
        images: scene.matches("DrawImage {").count(),
        sprites: scene.matches("DrawSprite {").count(),
        paths: scene.matches("DrawPath {").count(),
    }
}

#[test]
fn every_vector_producer_draws_paths_and_no_bitmap() {
    let rastered: Vec<String> = Producer::ALL
        .into_iter()
        .filter_map(|producer| {
            let drawn = drawn(producer);
            (drawn.images > 0 || drawn.sprites > 0 || drawn.paths == 0).then(|| {
                format!(
                    "{producer:?}: {} images, {} sprites, {} paths",
                    drawn.images, drawn.sprites, drawn.paths
                )
            })
        })
        .collect();
    assert!(
        rastered.is_empty(),
        "vector producers drew bitmaps: {rastered:?}"
    );
}
