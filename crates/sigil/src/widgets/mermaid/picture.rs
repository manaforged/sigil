use std::borrow::Cow;
use std::sync::{Arc, LazyLock, Mutex};

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, Tree, fontdb};

use crate::fnv::{OFFSET, mix};
use crate::kernel::{ImageRaster, ImageSource, NodeBuilder, ThemedImage, cached_image, image};
use crate::layout::LayoutStyle;
use crate::style::{Rgba8, Theme, Token};
use crate::widgets::Widget;

const FONT_FAMILY: &str = "Fira Sans";
const FONT_SIZE: f64 = 14.0;
const MAX_PIXELS: f32 = 4096.0 * 4096.0;

const SERIES: [Token; 10] = [
    Token::Accent9,
    Token::Success9,
    Token::Warn9,
    Token::Error9,
    Token::Info9,
    Token::Accent7,
    Token::Success7,
    Token::Warn7,
    Token::Error7,
    Token::Info7,
];

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

struct Diagram {
    source: String,
    key: u64,
    tree: Mutex<Option<(u64, Arc<Tree>)>>,
}

impl Diagram {
    fn tree(&self, theme: &Theme) -> Option<(u64, Arc<Tree>)> {
        let key = theme_key(self.key, theme);
        let mut slot = self.tree.lock().ok()?;
        if let Some((cached, tree)) = slot.as_ref()
            && *cached == key
        {
            return Some((key, Arc::clone(tree)));
        }
        let tree = Arc::new(parse(&self.source, theme)?);
        *slot = Some((key, Arc::clone(&tree)));
        Some((key, tree))
    }

    fn source(&self, theme: &Theme) -> Option<ImageSource> {
        let (key, tree) = self.tree(theme)?;
        let size = tree.size();
        raster(key, &tree, (size.width(), size.height()))
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "device pixel extents are far below 2^24"
    )]
    fn bind(&self, theme: &Theme) -> ImageRaster {
        let found = self.tree(theme);
        let key = found.as_ref().map_or(0, |(key, _)| *key);
        ImageRaster::new(key, move |width, height, _| {
            let (key, tree) = found.as_ref()?;
            raster(*key, tree, (width as f32, height as f32))
        })
    }
}

pub(super) fn figure(source: &str) -> Option<(NodeBuilder, f32)> {
    let light = Theme::light();
    let tree = Arc::new(parse(source, &light)?);
    let (width, height) = (tree.size().width(), tree.size().height());
    let key = source_key(source);
    let diagram = Arc::new(Diagram {
        source: source.to_string(),
        key,
        tree: Mutex::new(Some((theme_key(key, &light), tree))),
    });
    let first = diagram.source(&light)?;
    let fallback = Arc::clone(&diagram);
    let themed = ThemedImage::new(
        move |theme| fallback.source(theme),
        move |theme| diagram.bind(theme),
    );
    let node = image(first)
        .layout(LayoutStyle::block().fixed_size(width, height).shrink(0.0))
        .mutate(move |node| {
            let _ = node.attach(themed);
        })
        .into_node_builder();
    Some((node, width))
}

fn parse(source: &str, theme: &Theme) -> Option<Tree> {
    let svg = mermaid_svg::render_with(source, &palette(theme)).ok()?;
    let options = usvg::Options {
        font_family: FONT_FAMILY.to_string(),
        fontdb: Arc::clone(&FONTS),
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..usvg::Options::default()
    };
    let tree = Tree::from_str(&svg, &options).ok()?;
    (tree.size().width() >= 1.0 && tree.size().height() >= 1.0).then_some(tree)
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the raster is clamped to MAX_PIXELS before the cast"
)]
fn raster(key: u64, tree: &Tree, (width, height): (f32, f32)) -> Option<ImageSource> {
    let shrink = (MAX_PIXELS / (width * height).max(1.0)).sqrt().min(1.0);
    let (width, height) = ((width * shrink).round(), (height * shrink).round());
    let sized = mix(mix(key, u64::from(width.to_bits())), u64::from(height.to_bits()));
    cached_image(sized, || {
        let mut pixmap = Pixmap::new(width.max(1.0) as u32, height.max(1.0) as u32)?;
        let size = tree.size();
        let transform = Transform::from_scale(width / size.width(), height / size.height());
        resvg::render(tree, transform, &mut pixmap.as_mut());
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

fn source_key(source: &str) -> u64 {
    source.bytes().fold(OFFSET, |key, byte| mix(key, u64::from(byte)))
}

fn theme_key(key: u64, theme: &Theme) -> u64 {
    Token::ALL.iter().fold(key, |key, token| {
        let Rgba8 { r, g, b, a } = theme.color(*token);
        mix(key, u64::from(u32::from_le_bytes([r, g, b, a])))
    })
}

fn hex(theme: &Theme, token: Token) -> Cow<'static, str> {
    let Rgba8 { r, g, b, .. } = theme.color(token);
    Cow::Owned(format!("#{r:02x}{g:02x}{b:02x}"))
}

fn palette(theme: &Theme) -> mermaid_svg::Theme {
    let c = |token| hex(theme, token);
    let series: Cow<'static, [Cow<'static, str>]> =
        Cow::Owned(SERIES.iter().map(|token| c(*token)).collect());
    mermaid_svg::Theme {
        fg: c(Token::OnSurface),
        fg_muted: c(Token::OnSurfaceMuted),
        bg: c(Token::Surface),
        actor_fill: c(Token::Accent3),
        actor_stroke: c(Token::Accent8),
        actor_text_color: None,
        lifeline: c(Token::BorderStrong),
        arrow_stroke: c(Token::OnSurfaceMuted),
        signal_text_color: None,
        note_fill: c(Token::Warn3),
        note_stroke: c(Token::Warn8),
        activation_fill: c(Token::Accent4),
        activation_stroke: c(Token::Accent8),
        frame_label_fill: c(Token::Accent3),
        title_color: None,
        flow_node_fill: c(Token::Accent3),
        flow_node_stroke: c(Token::Accent8),
        flow_edge_stroke: c(Token::OnSurfaceMuted),
        flow_label_bg: c(Token::SurfaceSubtle),
        flow_cluster_fill: c(Token::Gray2),
        flow_cluster_stroke: c(Token::Border),
        cscale_palette: series.clone(),
        pie_palette: series.clone(),
        git_palette: series.clone(),
        xychart_palette: series,
        pie_stroke: Some(c(Token::Surface)),
        pie_opacity: None,
        commit_label_color: None,
        tag_label_color: None,
        quadrant_fills: [None, None, None, None],
        quadrant_default_fills: [
            c(Token::Accent2),
            c(Token::Success3),
            c(Token::Warn4),
            c(Token::Info3),
        ],
        font_family: Cow::Borrowed(FONT_FAMILY),
        font_size: FONT_SIZE,
        responsive: false,
    }
}
