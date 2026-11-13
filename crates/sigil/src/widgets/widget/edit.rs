use std::borrow::Cow;

use crate::kernel::Node;
use crate::layout::builder_ext::install_layout;
use crate::layout::{Axes, LayoutStyle, Length};

#[allow(
    clippy::large_enum_variant,
    reason = "an edit is built and consumed within one setter call"
)]
#[derive(Clone, Debug)]
pub(crate) enum LayoutEdit {
    Merge(LayoutStyle),
    Width(Length),
    Height(Length),
    Grow(f32),
}

impl LayoutEdit {
    pub(crate) fn apply(&self, style: &mut LayoutStyle) {
        let delta = self.delta(style.0.min_size);
        style.merge(&delta);
    }

    pub(crate) fn install(&self, node: &mut Node) {
        let min = node
            .extension::<LayoutStyle>()
            .map_or_else(|| Axes::splat(Length::Auto), |style| style.0.min_size);
        install_layout(node, &self.delta(min));
    }

    fn delta(&self, min: Axes<Length>) -> Cow<'_, LayoutStyle> {
        let delta = match self {
            Self::Merge(style) => return Cow::Borrowed(style),
            Self::Width(width) => {
                let delta = LayoutStyle::default().width(*width);
                match (min.width, width.px()) {
                    (Length::Px(floor), Some(px)) if floor > px => delta.min_width(*width),
                    _ => delta,
                }
            }
            Self::Height(height) => LayoutStyle::default().height(*height),
            Self::Grow(factor) => keep_min(LayoutStyle::default().grow(*factor), min),
        };
        Cow::Owned(delta)
    }
}

fn keep_min(delta: LayoutStyle, min: Axes<Length>) -> LayoutStyle {
    let delta = match min.width {
        Length::Auto => delta,
        width => delta.min_width(width),
    };
    match min.height {
        Length::Auto => delta,
        height => delta.min_height(height),
    }
}
