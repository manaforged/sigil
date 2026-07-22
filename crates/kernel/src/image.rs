//! Built-in raster image node primitive.
//!
//! Mirrors [`crate::text`]: a leaf content node carrying decoded pixels
//! rather than a string. The pixels live in an [`ImageSource`] with a
//! stable content id so the renderer uploads each distinct image to a GPU
//! texture exactly once, however many times it is drawn.

use std::rc::Rc;

use accesskit::Role;

use crate::{A11yProps, AccessibleName, Cx, IntoNode, NodeId, NodeType};

/// Decoded RGBA8 image pixels plus a stable content identity.
///
/// `rgba` is tightly packed, row-major, `width * height * 4` bytes. The
/// `id` is a content hash: the renderer keys its GPU texture cache on it,
/// so the same image drawn in many places uploads once. `Rc` keeps clones
/// cheap — the node owns one and each emitted draw command clones the
/// handle, never the pixels.
#[derive(Clone)]
pub struct ImageSource {
    id: u64,
    width: u32,
    height: u32,
    rgba: Rc<Vec<u8>>,
}

impl ImageSource {
    /// Build a source from tightly-packed RGBA8 pixels.
    ///
    /// Returns `None` when `rgba.len()` is not exactly `width * height * 4`
    /// — a trust-boundary check so a malformed buffer can never reach the
    /// GPU upload as a mis-sized texture.
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

    /// Content-hash identity used as the renderer's texture-cache key.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Pixel width.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Pixel height.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Borrow the packed RGBA8 pixels.
    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

/// Identity by content id. The id is a hash of the dimensions + pixels,
/// so equal ids mean equal images; this keeps `ImageSource` comparisons
/// (and any `SceneCommand` equality that transitively includes one) O(1)
/// instead of a per-pixel compare.
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

/// FNV-1a over dimensions + pixels. Deterministic across runs (fixed
/// offset basis), so a given image always hashes to the same cache key.
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

/// How an image's pixels map into its laid-out box when the box aspect
/// ratio differs from the image's (CSS `object-fit`). The box size itself
/// is unaffected — this only changes how the content fills it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ObjectFit {
    /// Stretch the image to fill the box exactly, ignoring aspect ratio.
    #[default]
    Fill,
    /// Scale to the largest size that fits fully inside the box, centered,
    /// preserving aspect ratio (letterboxed).
    Contain,
    /// Scale to the smallest size that fully covers the box, centered,
    /// preserving aspect ratio (overflow cropped).
    Cover,
}

/// Builder for a raster image node.
pub struct ImageBuilder {
    source: ImageSource,
    fit: ObjectFit,
    a11y: A11yProps,
}

/// Create a raster image node from decoded pixels.
///
/// The image is decorative by default (no accessible name); call
/// [`ImageBuilder::alt`] to give it one for screen readers.
#[must_use]
pub fn image(source: ImageSource) -> ImageBuilder {
    ImageBuilder {
        source,
        fit: ObjectFit::Fill,
        a11y: A11yProps {
            role: Role::Image,
            ..A11yProps::default()
        },
    }
}

impl ImageBuilder {
    /// Set the alternative text announced by assistive tech.
    #[must_use]
    pub fn alt(mut self, text: impl Into<String>) -> Self {
        self.a11y.name = AccessibleName::Static(text.into());
        self
    }

    /// Set how the pixels map into the laid-out box (CSS `object-fit`).
    #[must_use]
    pub const fn fit(mut self, fit: ObjectFit) -> Self {
        self.fit = fit;
        self
    }
}

impl IntoNode for ImageBuilder {
    fn build(self, cx: &mut Cx) -> NodeId {
        cx.new_node(
            NodeType::Image {
                source: self.source,
                fit: self.fit,
            },
            self.a11y,
        )
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn rejects_mismatched_buffer_length() {
        // 2x2 needs 16 bytes; 15 must be refused at the trust boundary.
        assert!(ImageSource::from_rgba8(2, 2, vec![0; 15]).is_none());
        assert!(ImageSource::from_rgba8(2, 2, vec![0; 16]).is_some());
        // Zero-area images carry no pixels and cannot upload.
        assert!(ImageSource::from_rgba8(0, 4, Vec::new()).is_none());
    }

    #[test]
    fn content_id_is_stable_and_distinguishes_pixels() {
        let a = ImageSource::from_rgba8(1, 1, vec![10, 20, 30, 255]).unwrap();
        let same = ImageSource::from_rgba8(1, 1, vec![10, 20, 30, 255]).unwrap();
        let different = ImageSource::from_rgba8(1, 1, vec![11, 20, 30, 255]).unwrap();
        assert_eq!(a.id(), same.id(), "identical pixels must hash equal");
        assert_ne!(a.id(), different.id(), "a changed pixel must change the id");
        assert_eq!(a, same);
        assert_ne!(a, different);
    }
}
