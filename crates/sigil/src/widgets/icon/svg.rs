use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use crate::icons::{BakedIcon, IconBaker};
use crate::kernel::{Cx, ImageSource, IntoNode, NodeId, ThemedImage, container, image};
use crate::layout::LayoutStyle;
use crate::style::{Rgba8, Theme, Token};

const CACHE_CAP: usize = 256;

type TintKey = (u64, [u8; 4]);

thread_local! {
    static BAKED: RefCell<HashMap<u64, Option<Rc<BakedIcon>>>> = RefCell::new(HashMap::new());
    static TINTED: RefCell<HashMap<TintKey, ImageSource>> = RefCell::new(HashMap::new());
}

pub(super) struct SvgGlyph {
    pub(super) svg: Rc<str>,
    pub(super) pixels: u32,
    pub(super) token: Token,
    pub(super) extent: f32,
}

fn bake_key(svg: &str, pixels: u32) -> u64 {
    let mut hasher = DefaultHasher::new();
    svg.hash(&mut hasher);
    pixels.hash(&mut hasher);
    hasher.finish()
}

fn baked(svg: &str, pixels: u32) -> Option<(u64, Rc<BakedIcon>)> {
    let key = bake_key(svg, pixels);
    BAKED.with(|map| {
        let mut map = map.borrow_mut();
        if map.len() >= CACHE_CAP {
            map.clear();
        }
        map.entry(key)
            .or_insert_with(|| IconBaker::new().bake(svg, pixels, pixels).ok().map(Rc::new))
            .clone()
            .map(|icon| (key, icon))
    })
}

fn tint(key: u64, icon: &BakedIcon, color: Rgba8) -> Option<ImageSource> {
    let tint_key = (key, [color.r, color.g, color.b, color.a]);
    if let Some(hit) = TINTED.with(|map| map.borrow().get(&tint_key).cloned()) {
        return Some(hit);
    }
    let rgba = icon
        .alpha
        .iter()
        .flat_map(|&alpha| {
            let scaled = u16::from(alpha) * u16::from(color.a) / 255;
            [
                color.r,
                color.g,
                color.b,
                u8::try_from(scaled).unwrap_or(u8::MAX),
            ]
        })
        .collect();
    let source = ImageSource::from_rgba8(icon.width, icon.height, rgba)?;
    TINTED.with(|map| {
        let mut map = map.borrow_mut();
        if map.len() >= CACHE_CAP {
            map.clear();
        }
        map.insert(tint_key, source.clone());
    });
    Some(source)
}

impl SvgGlyph {
    fn source(&self, theme: &Theme) -> Option<ImageSource> {
        let (key, icon) = baked(&self.svg, self.pixels)?;
        tint(key, &icon, theme.color(self.token))
    }
}

impl IntoNode for SvgGlyph {
    fn build(self, cx: &mut Cx) -> NodeId {
        let Some(source) = self.source(&Theme::light()) else {
            return container().build(cx);
        };
        let extent = self.extent;
        let id = image(source).build(cx);
        let layout = LayoutStyle::block().fixed_size(extent, extent).shrink(0.0);
        if let Some(node) = cx.get_mut(id) {
            let _ = node.attach(layout);
            let _ = node.attach(ThemedImage::new(move |theme| self.source(theme)));
        }
        id
    }
}
