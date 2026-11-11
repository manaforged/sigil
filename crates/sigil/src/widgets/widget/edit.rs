use std::borrow::Cow;

use smallvec::SmallVec;

use crate::events::builder_ext as on;
use crate::kernel::{Node, NodeBuilder};
use crate::layout::builder_ext::install_layout;
use crate::layout::{Axes, LayoutStyle, Length};
use crate::reactive::Signal;
use crate::style::{StateSlot, StyleAtom};

mod sealed {
    use super::{LayoutStyle, Length, NodeBuilder, Signal, SmallVec, StateSlot, StyleAtom};

    #[allow(unnameable_types, reason = "sealed")]
    pub trait Sealed: Sized {
        fn edit(self, edit: NodeEdit) -> Self;

        fn map_target(self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self;
    }

    #[allow(unnameable_types, reason = "sealed")]
    #[derive(Debug)]
    pub struct StatePaint {
        pub(crate) slot: StateSlot,
        pub(crate) atoms: SmallVec<[StyleAtom; 16]>,
    }

    #[allow(unnameable_types, reason = "sealed")]
    pub trait StateTarget: Sized {
        fn set_state(self, state: StatePaint) -> Self;
    }

    #[allow(unnameable_types, reason = "sealed")]
    #[derive(Debug)]
    pub struct NodeEdit(pub(crate) Edit);

    #[allow(
        clippy::large_enum_variant,
        reason = "an edit is built and consumed within one setter call"
    )]
    #[derive(Debug)]
    pub(crate) enum Edit {
        Layout(LayoutEdit),
        Autofocus,
        FocusWhen(Signal<u64>),
    }

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
}

pub(crate) use sealed::{Edit, LayoutEdit, NodeEdit, Sealed, StatePaint, StateTarget};

impl Edit {
    pub(crate) fn apply(self, node: NodeBuilder) -> NodeBuilder {
        match self {
            Self::Layout(edit) => node.mutate(move |node| edit.install(node)),
            Self::Autofocus => on::autofocus(node),
            Self::FocusWhen(trigger) => on::focus_when(node, trigger),
        }
    }
}

impl LayoutEdit {
    pub(crate) fn apply(&self, style: &mut LayoutStyle) {
        let delta = self.delta(style.0.min_size);
        style.merge(&delta);
    }

    pub(crate) fn install(&self, node: &mut Node) {
        let min = node
            .get_extension::<LayoutStyle>()
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
