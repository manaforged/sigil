use std::rc::Rc;

use accesskit::Role;

use crate::kernel::{A11yProps, AccessibleName, Cx, IntoNode, NodeBuilder, NodeId, NodeType};
use crate::style::{PaintTarget, Style, StyleSlot};
use crate::widgets::layout_ext::{LayoutOverrides, WidgetLayoutExt};

#[derive(Clone)]
pub struct ImageSource {
    id: u64,
    width: u32,
    height: u32,
    rgba: Rc<Vec<u8>>,
}

impl ImageSource {
    #[must_use]
    pub fn from_rgba8(width: u32, height: u32, rgba: Vec<u8>) -> Option<Self> {
        let expected = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        if rgba.len() != expected || expected == 0 {
            return None;
        }
        let id = content_id(width, height, &rgba);
        Some(Self {
            id,
            width,
            height,
            rgba: Rc::new(rgba),
        })
    }

    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageCrop {
    pub top: u32,
    pub height: u32,
}

impl PartialEq for ImageSource {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ImageSource {}

impl core::fmt::Debug for ImageSource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ImageSource")
            .field("id", &self.id)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("bytes", &self.rgba.len())
            .finish()
    }
}

fn content_id(width: u32, height: u32, rgba: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in width
        .to_le_bytes()
        .into_iter()
        .chain(height.to_le_bytes())
        .chain(rgba.iter().copied())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

type RasterFn = dyn Fn(u32, u32, f32) -> Option<ImageSource>;

#[derive(Clone)]
pub struct ImageRaster {
    key: u64,
    raster: Rc<RasterFn>,
}

impl ImageRaster {
    pub(crate) fn new(
        key: u64,
        raster: impl Fn(u32, u32, f32) -> Option<ImageSource> + 'static,
    ) -> Self {
        Self {
            key,
            raster: Rc::new(raster),
        }
    }

    #[must_use]
    pub fn at(&self, width: u32, height: u32, scale: f32) -> Option<ImageSource> {
        (self.raster)(width, height, scale)
    }
}

impl PartialEq for ImageRaster {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl core::fmt::Debug for ImageRaster {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ImageRaster")
            .field("key", &self.key)
            .finish_non_exhaustive()
    }
}

type ThemedSource = Box<dyn Fn(&crate::style::Theme) -> Option<ImageSource>>;
type ThemedRaster = Box<dyn Fn(&crate::style::Theme) -> ImageRaster>;

pub(crate) struct ThemedImage {
    source: ThemedSource,
    raster: ThemedRaster,
}

impl ThemedImage {
    pub(crate) fn new(
        source: impl Fn(&crate::style::Theme) -> Option<ImageSource> + 'static,
        raster: impl Fn(&crate::style::Theme) -> ImageRaster + 'static,
    ) -> Self {
        Self {
            source: Box::new(source),
            raster: Box::new(raster),
        }
    }

    pub(crate) fn resolve(&self, theme: &crate::style::Theme) -> Option<ImageSource> {
        (self.source)(theme)
    }

    pub(crate) fn raster(&self, theme: &crate::style::Theme) -> ImageRaster {
        (self.raster)(theme)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ObjectFit {
    #[default]
    Fill,
    Contain,
    Cover,
}

type NodeExt = Box<dyn FnOnce(&mut crate::kernel::Node)>;

pub struct ImageBuilder {
    source: ImageSource,
    fit: ObjectFit,
    a11y: A11yProps,
    pre_extensions: Vec<NodeExt>,
    layout_overrides: LayoutOverrides,
    style_accumulator: Style,
}

#[must_use]
pub fn image(source: ImageSource) -> ImageBuilder {
    ImageBuilder {
        source,
        fit: ObjectFit::Fill,
        a11y: A11yProps {
            role: Role::Image,
            ..A11yProps::default()
        },
        pre_extensions: Vec::new(),
        layout_overrides: LayoutOverrides::new(),
        style_accumulator: Style::new(),
    }
}

impl ImageBuilder {
    #[must_use]
    pub fn alt(mut self, text: impl Into<String>) -> Self {
        self.a11y.name = AccessibleName::Static(text.into());
        self
    }

    #[must_use]
    pub const fn fit(mut self, fit: ObjectFit) -> Self {
        self.fit = fit;
        self
    }

    #[must_use]
    pub fn crop(self, top: u32, height: u32) -> Self {
        self.mutate(move |node| {
            node.attach(ImageCrop { top, height });
        })
    }

    #[must_use]
    pub fn mutate(mut self, apply: impl FnOnce(&mut crate::kernel::Node) + 'static) -> Self {
        self.pre_extensions.push(Box::new(apply));
        self
    }
}

impl PaintTarget for ImageBuilder {
    fn style_slot(&mut self) -> StyleSlot<'_> {
        StyleSlot(&mut self.style_accumulator)
    }
}

impl WidgetLayoutExt for ImageBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for ImageBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        let kind = NodeType::Image {
            source: self.source,
            fit: self.fit,
        };
        let mut node = NodeBuilder::from_parts(kind, self.a11y, self.pre_extensions);
        if self.layout_overrides != LayoutOverrides::new() {
            node = node.with(
                self.layout_overrides
                    .clone()
                    .apply(crate::layout::LayoutStyle::default()),
            );
        }
        if !self.style_accumulator.is_empty() {
            node = node.style(self.style_accumulator);
        }
        self.layout_overrides.finish(node).build(cx)
    }
}

#[cfg(test)]
#[path = "image_tests.rs"]
mod tests;
