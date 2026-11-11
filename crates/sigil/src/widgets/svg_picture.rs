use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, LazyLock};

use resvg::tiny_skia::{Pixmap, PixmapMut, Transform};
use resvg::usvg::{self, Tree, fontdb};

use crate::cache::Bounded;
use crate::fnv::{OFFSET, bytes, mix, mix_f32};
use crate::kernel::{ImageSource, NodeBuilder, cached_image};
use crate::style::{Color, Theme, Token};

mod budget;
mod clip;
mod fallback;
mod filter;
mod gradient;
mod group;
mod lower;
mod paint;
mod pattern;
mod shape;
mod srgb;
mod vector;

#[cfg(feature = "kittest")]
pub use lower::{SvgMode, svg_scene};

pub(crate) const FONT_FAMILY: &str = "Fira Sans";
const MAX_SIDE: f32 = 4096.0;
const TREE_CAP: usize = 256;

type Render = Box<dyn Fn(&Theme) -> Option<String>>;

static FONTS: LazyLock<Arc<fontdb::Database>> = LazyLock::new(|| {
    let mut fonts = fontdb::Database::new();
    for face in [
        sigil_fonts::FIRA_SANS_REGULAR,
        sigil_fonts::FIRA_SANS_MEDIUM,
        sigil_fonts::FIRA_SANS_SEMIBOLD,
        sigil_fonts::CASCADIA_MONO_REGULAR,
    ] {
        fonts.load_font_data(face.to_vec());
    }
    fonts.set_sans_serif_family(FONT_FAMILY);
    fonts.set_monospace_family("Cascadia Mono");
    Arc::new(fonts)
});

thread_local! {
    static TREES: RefCell<Bounded<u64, Arc<Tree>, TREE_CAP>> = RefCell::new(Bounded::default());
}

/// The faces every SVG picture lays out its text with.
pub(crate) fn fonts() -> Arc<fontdb::Database> {
    Arc::clone(&FONTS)
}

struct Picture {
    render: Render,
    key: u64,
    tree: RefCell<Option<(u64, Arc<Tree>)>>,
}

impl Picture {
    fn tree(&self, theme: &Theme) -> Option<(u64, Arc<Tree>)> {
        let key = theme_key(self.key, theme);
        if let Some((cached, tree)) = self.tree.borrow().as_ref()
            && *cached == key
        {
            return Some((key, Arc::clone(tree)));
        }
        let Some(tree) = run(&self.render, theme).and_then(|svg| parse(&svg)) else {
            return self.tree.borrow().clone();
        };
        *self.tree.borrow_mut() = Some((key, Arc::clone(&tree)));
        Some((key, tree))
    }
}

/// A vector picture of the SVG that `render` writes for each theme, with its size in logical pixels.
pub(crate) fn picture(
    key: &str,
    render: impl Fn(&Theme) -> Option<String> + 'static,
) -> Option<(NodeBuilder, (f32, f32))> {
    drawn(key, Box::new(render), None)
}

/// A vector picture of the SVG that `render` writes for each theme, drawn in a square of `side` logical pixels.
pub(crate) fn square(
    key: &str,
    side: f32,
    render: impl Fn(&Theme) -> Option<String> + 'static,
) -> Option<NodeBuilder> {
    drawn(key, Box::new(render), Some((side, side))).map(|(node, _)| node)
}

fn drawn(key: &str, render: Render, fit: Option<(f32, f32)>) -> Option<(NodeBuilder, (f32, f32))> {
    let light = Theme::light();
    let svg = run(&render, &light)?;
    let tree = parse(&svg)?;
    let size = fit.unwrap_or_else(|| (tree.size().width(), tree.size().height()));
    let hash = bytes(bytes(OFFSET, key.as_bytes()), svg.as_bytes());
    let picture = Picture {
        render,
        key: hash,
        tree: RefCell::new(Some((theme_key(hash, &light), tree))),
    };
    Some((vector::node(picture, size), size))
}

/// The `#rrggbb` form of a theme color.
pub(crate) fn hex(theme: &Theme, token: Token) -> String {
    let Color { r, g, b, .. } = theme.color(token);
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn parse(svg: &str) -> Option<Arc<Tree>> {
    let hash = bytes(OFFSET, svg.as_bytes());
    if let Some(tree) = TREES.with(|trees| trees.borrow().get(&hash).cloned()) {
        return Some(tree);
    }
    let tree = Arc::new(parse_tree(svg)?);
    TREES.with_borrow_mut(|trees| trees.insert(hash, Arc::clone(&tree)));
    Some(tree)
}

const MAX_SOURCE_BYTES: usize = 32 << 20;
const MAX_XML_NODES: u32 = 1 << 22;

fn parse_tree(svg: &str) -> Option<Tree> {
    if svg.len() > MAX_SOURCE_BYTES {
        return None;
    }
    let parsing = usvg::roxmltree::ParsingOptions {
        allow_dtd: true,
        nodes_limit: MAX_XML_NODES,
        ..Default::default()
    };
    let document = usvg::roxmltree::Document::parse_with_options(svg, parsing).ok()?;
    if !budget::document_fits(&document) {
        return None;
    }
    let options = usvg::Options {
        font_family: FONT_FAMILY.to_string(),
        fontdb: fonts(),
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    let tree = Tree::from_xmltree(&document, &options).ok()?;
    (tree.size().width() >= 1.0 && tree.size().height() >= 1.0).then_some(tree)
}

fn raster(key: u64, tree: &Tree, (width, height): (f32, f32)) -> Option<ImageSource> {
    let size = tree.size();
    raster_with(key, (width, height), |pixels, (x, y)| {
        let fit = Transform::from_scale(x / size.width(), y / size.height());
        resvg::render(tree, fit, pixels);
    })
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "each side is clamped to MAX_SIDE before the cast"
)]
fn raster_with(
    key: u64,
    (width, height): (f32, f32),
    paint: impl FnOnce(&mut PixmapMut<'_>, (f32, f32)),
) -> Option<ImageSource> {
    let shrink = (MAX_SIDE / width.max(height)).min(1.0);
    let width = (width * shrink).round().clamp(1.0, MAX_SIDE);
    let height = (height * shrink).round().clamp(1.0, MAX_SIDE);
    let sized = mix_f32(mix_f32(key, width), height);
    cached_image(sized, || {
        let mut pixmap = Pixmap::new(width as u32, height as u32)?;
        paint(&mut pixmap.as_mut(), (width, height));
        let rgba = pixmap
            .pixels()
            .iter()
            .flat_map(|pixel| {
                let color = pixel.demultiply();
                [color.red(), color.green(), color.blue(), color.alpha()]
            })
            .collect();
        ImageSource::from_rgba8(pixmap.width(), pixmap.height(), rgba)
    })
}

fn run(render: &impl Fn(&Theme) -> Option<String>, theme: &Theme) -> Option<String> {
    catch_unwind(AssertUnwindSafe(|| render(theme))).ok()?
}

fn theme_key(key: u64, theme: &Theme) -> u64 {
    Token::ALL.iter().fold(key, |key, token| {
        mix(key, crate::fnv::rgba(theme.color(*token)))
    })
}

#[cfg(test)]
#[path = "svg_picture_tests.rs"]
mod tests;
