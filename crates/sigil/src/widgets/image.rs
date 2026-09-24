use crate::kernel::{ImageBuilder, ImageSource, image};
use crate::layout::LayoutStyle;

use super::layout_ext::WidgetLayoutExt;

#[must_use]
pub fn image_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Option<ImageBuilder> {
    ImageSource::from_rgba8(width, height, rgba)
        .map(|source| image(source).layout(LayoutStyle::block()))
}

#[cfg(feature = "image-decode")]
#[must_use]
pub fn image_bytes(encoded: &[u8]) -> Option<ImageBuilder> {
    let decoded = ::image::load_from_memory(encoded).ok()?.into_rgba8();
    let (width, height) = decoded.dimensions();
    image_rgba(width, height, decoded.into_raw())
}
