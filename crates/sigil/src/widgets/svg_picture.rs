use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, LazyLock};

use resvg::tiny_skia::{Pixmap, PixmapMut, PremultipliedColorU8, Transform};
use resvg::usvg::{self, Tree, fontdb};

use crate::fnv::{OFFSET, mix};
use crate::kernel::{ImageSource, NodeBuilder, cached_image};
use crate::style::srgb::{decode, encode_byte};
use crate::style::{Rgba8, Theme, Token};

mod clip;
mod fallback;
mod filter;
mod gradient;
mod group;
mod lower;
mod paint;
mod pattern;
mod shape;
mod vector;

use lower::Draw;
#[cfg(feature = "kittest")]
pub use lower::{SvgMode, svg_scene};

pub(crate) const FONT_FAMILY: &str = "Fira Sans";
const MAX_SIDE: f32 = 4096.0;

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
        let tree = Arc::new(tree);
        *self.tree.borrow_mut() = Some((key, Arc::clone(&tree)));
        Some((key, tree))
    }
}

/// A vector picture of the SVG that `render` writes for each theme, with its size in logical pixels.
pub(crate) fn picture(
    key: &str,
    render: impl Fn(&Theme) -> Option<String> + 'static,
) -> Option<(NodeBuilder, (f32, f32))> {
    drawn(key, Box::new(render), Draw::Vector)
}

/// A bitmap picture of the SVG that `render` writes for each theme, rastered again at the device size.
pub(crate) fn bitmap(
    key: &str,
    render: impl Fn(&Theme) -> Option<String> + 'static,
) -> Option<(NodeBuilder, (f32, f32))> {
    drawn(key, Box::new(render), Draw::Bitmap)
}

fn drawn(key: &str, render: Render, draw: Draw) -> Option<(NodeBuilder, (f32, f32))> {
    let light = Theme::light();
    let svg = run(&render, &light)?;
    let tree = Arc::new(parse(&svg)?);
    let size = (tree.size().width(), tree.size().height());
    let hash = key
        .bytes()
        .chain(svg.bytes())
        .fold(OFFSET, |key, byte| mix(key, u64::from(byte)));
    let picture = Picture {
        render,
        key: hash,
        tree: RefCell::new(Some((theme_key(hash, &light), tree))),
    };
    Some((vector::node(picture, size, draw), size))
}

/// The `#rrggbb` form of a theme color.
pub(crate) fn hex(theme: &Theme, token: Token) -> String {
    let Rgba8 { r, g, b, .. } = theme.color(token);
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn parse(svg: &str) -> Option<Tree> {
    let options = usvg::Options {
        font_family: FONT_FAMILY.to_string(),
        fontdb: fonts(),
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    let tree = Tree::from_str(svg, &options).ok()?;
    (tree.size().width() >= 1.0 && tree.size().height() >= 1.0).then_some(tree)
}

fn raster(
    key: u64,
    tree: &Tree,
    (width, height): (f32, f32),
    backdrop: [u8; 3],
) -> Option<ImageSource> {
    let size = tree.size();
    raster_with(key, (width, height), Some(backdrop), |pixels, (x, y)| {
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
    backdrop: Option<[u8; 3]>,
    paint: impl FnOnce(&mut PixmapMut<'_>, (f32, f32)),
) -> Option<ImageSource> {
    let shrink = (MAX_SIDE / width.max(height)).min(1.0);
    let width = (width * shrink).round().clamp(1.0, MAX_SIDE);
    let height = (height * shrink).round().clamp(1.0, MAX_SIDE);
    let sized = mix(
        mix(key, u64::from(width.to_bits())),
        u64::from(height.to_bits()),
    );
    cached_image(sized, || {
        let mut pixmap = Pixmap::new(width as u32, height as u32)?;
        paint(&mut pixmap.as_mut(), (width, height));
        let rgba = pixmap
            .pixels()
            .iter()
            .flat_map(|pixel| linear_over(*pixel, backdrop))
            .collect();
        ImageSource::from_rgba8(pixmap.width(), pixmap.height(), rgba)
    })
}

fn surface(theme: &Theme) -> [u8; 3] {
    let Rgba8 { r, g, b, .. } = theme.color(Token::Surface);
    [r, g, b]
}

fn linear_over(pixel: PremultipliedColorU8, backdrop: Option<[u8; 3]>) -> [u8; 4] {
    let color = pixel.demultiply();
    let rgba = [color.red(), color.green(), color.blue(), color.alpha()];
    backdrop.map_or(rgba, |backdrop| over_surface(rgba, backdrop))
}

/// The straight color whose linear-light composite over `backdrop` equals the sRGB composite of `rgba`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "alpha is clamped to 1..=255 before the cast"
)]
fn over_surface([r, g, b, alpha]: [u8; 4], backdrop: [u8; 3]) -> [u8; 4] {
    if alpha == 0 || alpha == u8::MAX {
        return [r, g, b, alpha];
    }
    let coverage = f32::from(alpha) / 255.0;
    let under = backdrop.map(decode);
    let mut over = [0.0; 3];
    for ((target, channel), paper) in over.iter_mut().zip([r, g, b]).zip(backdrop) {
        *target = decode(blend(channel, paper, coverage));
    }
    let least = over
        .iter()
        .zip(under)
        .map(|(target, paper)| least_alpha(*target, paper))
        .fold(coverage, f32::max);
    let raised = (least * 255.0).ceil().clamp(1.0, 255.0) as u8;
    let kept = f32::from(raised) / 255.0;
    let mut rgb = [0; 3];
    for ((channel, target), paper) in rgb.iter_mut().zip(over).zip(under) {
        *channel = encode_byte(paper.mul_add(kept - 1.0, target) / kept);
    }
    let [red, green, blue] = rgb;
    [red, green, blue, raised]
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a mix of two bytes stays within 0..=255"
)]
fn blend(color: u8, backdrop: u8, coverage: f32) -> u8 {
    f32::from(color)
        .mul_add(coverage, f32::from(backdrop) * (1.0 - coverage))
        .round() as u8
}

fn least_alpha(target: f32, backdrop: f32) -> f32 {
    if target < backdrop {
        1.0 - target / backdrop
    } else if target > backdrop {
        (target - backdrop) / (1.0 - backdrop)
    } else {
        0.0
    }
}

fn run(render: &impl Fn(&Theme) -> Option<String>, theme: &Theme) -> Option<String> {
    catch_unwind(AssertUnwindSafe(|| render(theme))).ok()?
}

fn theme_key(key: u64, theme: &Theme) -> u64 {
    Token::ALL.iter().fold(key, |key, token| {
        let Rgba8 { r, g, b, a } = theme.color(*token);
        mix(key, u64::from(u32::from_le_bytes([r, g, b, a])))
    })
}

#[cfg(test)]
#[path = "svg_picture_tests.rs"]
mod tests;
