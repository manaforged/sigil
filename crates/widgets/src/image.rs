//! Raster image widget.
//!
//! Wraps the kernel [`sigil_kernel::ImageSource`] primitive with the
//! layout ergonomics widgets expect: build one from decoded RGBA pixels,
//! size it with the shared `.width()`/`.height()` setters, and give it
//! alt text.
//!
//! An unsized image lays out at its intrinsic pixel dimensions; a pinned
//! `.width()`/`.height()` overrides the corresponding axis.
//!
//! The input is decoded RGBA8 (decode-agnostic on purpose — the core UI
//! crate stays free of image-format decoders). Callers wanting to load an
//! encoded file decode it with their own `image` crate and hand the
//! pixels here:
//!
//! ```ignore
//! let decoded = image::open("logo.png")?.into_rgba8();
//! let (w, h) = decoded.dimensions();
//! image_rgba(w, h, decoded.into_raw()).unwrap().width(64.0)
//! ```

use sigil_kernel::{Cx, ImageSource, IntoNode, NodeId, ObjectFit};
use sigil_layout::LayoutStyle;

use super::layout_ext::{LayoutOverrides, WidgetLayoutExt};

/// Builder returned by [`image_rgba`].
pub struct ImageBuilder {
    source: ImageSource,
    alt: Option<String>,
    fit: ObjectFit,
    layout_overrides: LayoutOverrides,
}

/// Build an image from tightly-packed RGBA8 pixels (`width * height * 4`
/// bytes). Returns `None` when the buffer length does not match the
/// dimensions — a malformed buffer can never reach the GPU.
#[must_use]
pub fn image_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Option<ImageBuilder> {
    ImageSource::from_rgba8(width, height, rgba).map(ImageBuilder::new)
}

/// Build an image by decoding encoded bytes (PNG). Returns `None` when
/// decoding fails. Requires the `image-decode` feature.
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
            layout_overrides: LayoutOverrides::new(),
        }
    }

    /// Set the alternative text announced by assistive tech. Without it
    /// the image is decorative and screen readers skip it.
    #[must_use]
    pub fn alt(mut self, text: impl Into<String>) -> Self {
        self.alt = Some(text.into());
        self
    }

    /// Set how the pixels map into the box (CSS `object-fit`): `Fill`
    /// (default, stretch), `Contain` (letterbox), or `Cover` (crop).
    #[must_use]
    pub const fn fit(mut self, fit: ObjectFit) -> Self {
        self.fit = fit;
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
        let id = builder.build(cx);
        // Attach the resolved layout so `.width()`/`.height()` reach the
        // node; unset axes stay `Auto` and the engine fills them with the
        // intrinsic pixel size.
        let style = self.layout_overrides.apply(LayoutStyle::block());
        if let Some(node) = cx.get_mut(id) {
            node.attach(style);
        }
        id
    }
}
