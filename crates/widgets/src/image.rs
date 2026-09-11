use sigil_kernel::{Cx, ImageSource, IntoNode, NodeId, ObjectFit};
use sigil_layout::LayoutStyle;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

pub struct ImageBuilder {
    source: ImageSource,
    alt: Option<String>,
    fit: ObjectFit,
    crop: Option<(u32, u32)>,
    layout_overrides: LayoutOverrides,
}

#[must_use]
pub fn image_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Option<ImageBuilder> {
    ImageSource::from_rgba8(width, height, rgba).map(ImageBuilder::new)
}

#[cfg(feature = "image-decode")]
#[must_use]
pub fn image_bytes(encoded: &[u8]) -> Option<ImageBuilder> {
    let decoded = ::image::load_from_memory(encoded).ok()?.into_rgba8();
    let (width, height) = decoded.dimensions();
    image_rgba(width, height, decoded.into_raw())
}

impl ImageBuilder {
    const fn new(source: ImageSource) -> Self {
        Self {
            source,
            alt: None,
            fit: ObjectFit::Fill,
            crop: None,
            layout_overrides: LayoutOverrides::new(),
        }
    }

    #[must_use]
    pub fn alt(mut self, text: impl Into<String>) -> Self {
        self.alt = Some(text.into());
        self
    }

    #[must_use]
    pub const fn fit(mut self, fit: ObjectFit) -> Self {
        self.fit = fit;
        self
    }

    #[must_use]
    pub const fn crop(mut self, top: u32, height: u32) -> Self {
        self.crop = Some((top, height));
        self
    }
}

impl WidgetLayoutExt for ImageBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

impl IntoNode for ImageBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        let mut builder = sigil_kernel::image(self.source).fit(self.fit);
        if let Some(alt) = self.alt {
            builder = builder.alt(alt);
        }
        if let Some((top, height)) = self.crop {
            builder = builder.crop(top, height);
        }
        let id = builder.build(cx);
        let style = self.layout_overrides.apply(LayoutStyle::block());
        if let Some(node) = cx.get_mut(id) {
            node.attach(style);
        }
        id
    }
}
