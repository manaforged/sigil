use crate::kernel::Role;

use crate::kernel::hooks::NodeHooks;
use crate::kernel::{A11yProps, Cx, IntoNode, NodeBuilder, NodeId, NodeType};
use crate::style::{Paint, StateStyleBuilder};

/// Decoded RGBA pixels.
#[derive(Clone)]
pub struct ImageSource {
    id: u64,
    width: u32,
    height: u32,
    rgba: std::sync::Arc<Vec<u8>>,
}

impl ImageSource {
    /// Decodes PNG, JPEG, and other encoded bytes, cached by content.
    #[cfg(feature = "image-decode")]
    #[must_use]
    pub fn decode(encoded: &[u8]) -> Option<Self> {
        let key = crate::fnv::bytes(crate::fnv::OFFSET, encoded);
        crate::kernel::cached_image(key, || Self::decode_uncached(encoded).ok())
    }

    #[cfg(feature = "image-decode")]
    pub(crate) fn decode_uncached(encoded: &[u8]) -> Result<Self, crate::Error> {
        let decoded = ::image::load_from_memory(encoded)
            .map_err(|err| crate::Error::message(crate::ErrorKind::Other, err))?
            .into_rgba8();
        let (width, height) = decoded.dimensions();
        Self::from_rgba8(width, height, decoded.into_raw())
            .ok_or_else(|| crate::Error::fixed(crate::ErrorKind::Other, "the image has no pixels"))
    }

    /// Decodes a base64 raster `data:` URI; SVG and non-base64 URIs are refused.
    #[cfg(feature = "image-decode")]
    pub(crate) fn from_data_uri(uri: &str) -> Option<Self> {
        Self::decode(&data_uri_bytes(uri)?)
    }

    /// Creates an image source from rgba8.
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
            rgba: std::sync::Arc::new(rgba),
        })
    }

    /// A content id computed from the size and the pixels.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// The width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// The height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The pixel bytes, four per pixel in RGBA order.
    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

/// Vertical crop of an image: rows `top..top + height`, clamped to the image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageCrop {
    /// The first pixel row of the image to show, counted from 0.
    pub top: u32,
    /// The number of pixel rows to show.
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

#[cfg(feature = "image-decode")]
pub(crate) fn data_uri_bytes(uri: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    let (meta, payload) = uri.strip_prefix("data:")?.split_once(',')?;
    if !meta.ends_with(";base64") || meta.contains("svg") {
        return None;
    }
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .ok()
}

fn content_id(width: u32, height: u32, rgba: &[u8]) -> u64 {
    let hash = crate::fnv::bytes(crate::fnv::OFFSET, &width.to_le_bytes());
    let hash = crate::fnv::bytes(hash, &height.to_le_bytes());
    crate::fnv::bytes(hash, rgba)
}

#[cfg(raster)]
type RasterFn = dyn Fn(u32, u32, f32) -> Option<ImageSource> + Send + Sync;

/// Resolution-independent source of an image.
#[derive(Clone)]
pub struct ImageRaster {
    key: u64,
    #[cfg(raster)]
    raster: std::sync::Arc<RasterFn>,
}

impl ImageRaster {
    pub(crate) fn new(
        key: u64,
        raster: impl Fn(u32, u32, f32) -> Option<ImageSource> + Send + Sync + 'static,
    ) -> Self {
        #[cfg(not(raster))]
        let _ = raster;
        Self {
            key,
            #[cfg(raster)]
            raster: std::sync::Arc::new(raster),
        }
    }

    #[cfg(raster)]
    /// Pixels for a physical size in device pixels at `scale`.
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

/// How an image fills its box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ObjectFit {
    /// Stretches the image to the box and ignores its aspect ratio.
    #[default]
    Fill,
    /// Scales the whole image to fit inside the box, centred, and keeps its aspect ratio.
    Contain,
    /// Scales the image to cover the box, keeps its aspect ratio, and crops around the centre.
    Cover,
}

/// Builds an image node; `image` creates one.
#[must_use]
#[derive(Debug)]
pub struct ImageBuilder {
    pub(crate) node: NodeBuilder,
    pub(crate) hooks: NodeHooks,
}

/// An image node from an `ImageSource`.
pub fn image(source: ImageSource) -> ImageBuilder {
    let kind = NodeType::Image {
        source,
        fit: ObjectFit::Fill,
    };
    let a11y = A11yProps {
        role: Role::Image,
        ..A11yProps::default()
    };
    ImageBuilder {
        node: NodeBuilder::from_parts(kind, a11y, Vec::new()),
        hooks: NodeHooks::default(),
    }
}

impl ImageBuilder {
    /// Sets how the image fits its box: fill, contain, or cover.
    pub const fn fit(mut self, fit: ObjectFit) -> Self {
        if let NodeType::Image { fit: slot, .. } = &mut self.node.kind {
            *slot = fit;
        }
        self
    }

    /// Shows only pixel rows `top..top + height` of the image, clamped to the image.
    pub fn crop(self, top: u32, height: u32) -> Self {
        self.mutate(move |node| {
            node.attach(ImageCrop { top, height });
        })
    }

    shared_node_methods!(node);

    pub(crate) fn into_node_builder(self) -> NodeBuilder {
        self.hooks.finish(self.node)
    }
}

impl Paint for ImageBuilder {
    fn paint_mut(&mut self) -> &mut StateStyleBuilder {
        self.node.paint_mut()
    }
}

impl IntoNode for ImageBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.into_node_builder().build(cx)
    }
}

#[cfg(test)]
#[path = "image_tests.rs"]
mod tests;
