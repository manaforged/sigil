use std::fmt::Write;
use std::sync::LazyLock;

use resvg::usvg::{self, Node, Tree};

use crate::cache::Bounded;
use crate::widgets::svg_picture::{FONT_FAMILY, fonts};

const MAX_ENTRIES: usize = 16_384;

static OPTIONS: LazyLock<usvg::Options<'static>> = LazyLock::new(|| usvg::Options {
    font_family: FONT_FAMILY.to_string(),
    fontdb: fonts(),
    ..usvg::Options::default()
});

pub(super) struct Font<'a> {
    pub(super) size: f64,
    pub(super) family: &'a str,
    pub(super) weight: &'a str,
    pub(super) style: &'a str,
}

#[derive(Default)]
pub(super) struct Widths {
    known: Bounded<(String, String), f64, MAX_ENTRIES>,
}

impl Widths {
    pub(super) fn width(&mut self, text: &str, font: &Font<'_>) -> f64 {
        if font.size <= 0.0 || text.trim().is_empty() {
            return 0.0;
        }
        let face = format!(
            "{}|{}|{}|{}",
            font.size, font.family, font.weight, font.style
        );
        let key = (text.to_string(), face);
        if let Some(width) = self.known.get(&key) {
            return *width;
        }
        let width = measure(text, font);
        self.known.insert(key, width);
        width
    }
}

fn measure(text: &str, font: &Font<'_>) -> f64 {
    width_of(&svg(text, font, false))
        .or_else(|| width_of(&svg(&"0".repeat(text.chars().count()), font, true)))
        .unwrap_or(0.0)
}

fn svg(text: &str, font: &Font<'_>, plain: bool) -> String {
    let mut attrs = format!("font-size=\"{}\"", font.size);
    let family = if plain { "sans-serif" } else { font.family };
    if !family.is_empty() {
        let _ = write!(
            attrs,
            " font-family=\"{}\"",
            family.replace(['"', '\''], "")
        );
    }
    for (name, value) in [("font-weight", font.weight), ("font-style", font.style)] {
        if !plain && !value.is_empty() {
            let _ = write!(attrs, " {name}=\"{}\"", escape(value));
        }
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"100\" height=\"100\"><text x=\"20\" y=\"50\" {attrs}>{}</text></svg>",
        escape(text)
    )
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '&' => out.push_str("&amp;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            _ => out.push(c),
        }
    }
    out
}

fn width_of(svg: &str) -> Option<f64> {
    let tree = Tree::from_str(svg, &OPTIONS).ok()?;
    tree.root().children().iter().find_map(|node| match node {
        Node::Text(text) => {
            let bounds = text.bounding_box();
            Some(f64::from(bounds.right() - bounds.left()))
        }
        _ => None,
    })
}
