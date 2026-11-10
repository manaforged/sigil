use std::borrow::Cow;

use smallvec::SmallVec;

use crate::events::builder_ext as on;
use crate::kernel::{Node, NodeBuilder};
use crate::layout::builder_ext::install_layout;
use crate::layout::{Axes, Dimension, LayoutStyle};
use crate::reactive::Signal;
use crate::style::{StateSlot, StyleAtom};

mod sealed {
    use super::{LayoutStyle, NodeBuilder, Signal, SmallVec, StateSlot, StyleAtom};

    pub trait Sealed: Sized {
        fn edit(self, edit: Edit) -> Self;

        fn map_target(self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self;
    }

    pub trait StateTarget: Sized {
        fn set_state(self, slot: StateSlot, atoms: SmallVec<[StyleAtom; 16]>) -> Self;
    }

    #[allow(
        clippy::large_enum_variant,
        reason = "an edit is built and consumed within one setter call"
    )]
    #[derive(Debug)]
    pub enum Edit {
        Layout(LayoutEdit),
        Autofocus,
        FocusWhen(Signal<u64>),
    }

    #[allow(
        clippy::large_enum_variant,
        reason = "an edit is built and consumed within one setter call"
    )]
    #[derive(Clone, Debug)]
    pub enum LayoutEdit {
        Merge(LayoutStyle),
        Width(f32),
        Height(f32),
        Grow(f32),
    }
}

pub(crate) use sealed::{Edit, LayoutEdit, Sealed, StateTarget};

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
            .map_or_else(|| Axes::splat(Dimension::Auto), |style| style.0.min_size);
        install_layout(node, &self.delta(min));
    }

    fn delta(&self, min: Axes<Dimension>) -> Cow<'_, LayoutStyle> {
        let delta = match self {
            Self::Merge(style) => return Cow::Borrowed(style),
            Self::Width(px) => match min.width {
                Dimension::Px(floor) if floor > *px => {
                    LayoutStyle::default().width(*px).min_width(*px)
                }
                _ => LayoutStyle::default().width(*px),
            },
            Self::Height(px) => LayoutStyle::default().height(*px),
            Self::Grow(factor) => keep_min(LayoutStyle::default().grow(*factor), min),
        };
        Cow::Owned(delta)
    }
}

fn keep_min(delta: LayoutStyle, min: Axes<Dimension>) -> LayoutStyle {
    let delta = match min.width {
        Dimension::Auto => delta,
        width => delta.min_width(width),
    };
    match min.height {
        Dimension::Auto => delta,
        height => delta.min_height(height),
    }
}
