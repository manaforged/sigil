use crate::kernel::{ImageSource, NodeBuilder, ObjectFit, image};
use crate::layout::LayoutStyle;
use crate::layout::snap::CellForm;
use crate::widgets::Widget;

use super::render::note;
use super::spec::ImageSize;

#[cfg(feature = "image-decode")]
fn decode(url: &str) -> Option<ImageSource> {
    ImageSource::from_data_uri(url)
}

#[cfg(not(feature = "image-decode"))]
const fn decode(_: &str) -> Option<ImageSource> {
    None
}

const fn box_width(size: ImageSize) -> f32 {
    match size {
        ImageSize::Icon => 24.0,
        ImageSize::Avatar => 48.0,
        ImageSize::Small => 120.0,
        ImageSize::Medium => 240.0,
        ImageSize::Large => 400.0,
        ImageSize::Header => 640.0,
    }
}

#[expect(clippy::cast_precision_loss, reason = "image dimensions fit f32")]
fn layout(source: &ImageSource, size: ImageSize) -> LayoutStyle {
    let natural = (source.width() as f32, source.height() as f32);
    let width = box_width(size).min(natural.0);
    let height = match size {
        ImageSize::Icon | ImageSize::Avatar => width,
        _ => width * natural.1 / natural.0,
    };
    let window = (width, height);
    let (width, height) = CellForm::new(window, (width, height * 0.5)).pick();
    match size {
        ImageSize::Header => LayoutStyle::block()
            .fill_width()
            .max_width(width)
            .height(height),
        _ => LayoutStyle::block().width(width).height(height).shrink(0.0),
    }
}

pub(super) fn picture(url: &str, alt: String, size: ImageSize, fit: ObjectFit) -> NodeBuilder {
    let Some(source) = decode(url) else {
        let name = if alt.is_empty() {
            "image"
        } else {
            alt.as_str()
        };
        return note(format!("[{name}]"));
    };
    let layout = layout(&source, size);
    image(source).alt(alt).fit(fit).layout(layout).into()
}
