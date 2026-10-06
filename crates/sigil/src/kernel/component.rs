use crate::kernel::{Cx, NodeId};

pub trait IntoNode: IntoNodeBox {
    fn build(self, cx: &mut Cx) -> NodeId;
}

pub trait IntoNodeBox {
    fn build_box(self: Box<Self>, cx: &mut Cx) -> NodeId;
}

impl<T> IntoNodeBox for T
where
    T: IntoNode,
{
    fn build_box(self: Box<Self>, cx: &mut Cx) -> NodeId {
        (*self).build(cx)
    }
}

impl IntoNode for Box<dyn IntoNode> {
    fn build(self, cx: &mut Cx) -> NodeId {
        <dyn IntoNode as IntoNodeBox>::build_box(self, cx)
    }
}

impl<F, N> IntoNode for F
where
    F: FnOnce() -> N,
    N: IntoNode,
{
    fn build(self, cx: &mut Cx) -> NodeId {
        self().build(cx)
    }
}

pub(crate) struct WithCx<F>(pub(crate) F);

impl<F> IntoNode for WithCx<F>
where
    F: FnOnce(&mut Cx) -> NodeId,
{
    fn build(self, cx: &mut Cx) -> NodeId {
        (self.0)(cx)
    }
}

#[must_use]
pub fn node(child: impl IntoNode + 'static) -> Box<dyn IntoNode> {
    Box::new(child)
}

macro_rules! into_node_builder {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for crate::kernel::NodeBuilder {
                fn from(builder: $ty) -> Self {
                    crate::kernel::fragment().child(builder)
                }
            }
        )*
    };
}

into_node_builder!(
    crate::kernel::TextBuilder,
    crate::kernel::ImageBuilder,
    crate::widgets::ButtonBuilder,
    crate::widgets::ChartBuilder,
    crate::widgets::CheckboxBuilder,
    crate::widgets::CodeEditBuilder,
    crate::widgets::ColorFieldBuilder,
    crate::widgets::CommandMenuBuilder,
    crate::widgets::DateFieldBuilder,
    crate::widgets::DiffBuilder,
    crate::widgets::DotBuilder,
    crate::widgets::DropdownBuilder,
    crate::widgets::GaugeBuilder,
    crate::widgets::IconBuilder,
    crate::widgets::MarkdownEditBuilder,
    crate::widgets::MenuBarBuilder,
    crate::widgets::MenuBuilder,
    crate::widgets::ModalBuilder,
    crate::widgets::PanelBuilder,
    crate::widgets::PathNode,
    crate::widgets::ProgressBuilder,
    crate::widgets::RadioGroupBuilder,
    crate::widgets::SegmentedBuilder,
    crate::widgets::SliderBuilder,
    crate::widgets::SparklineBuilder,
    crate::widgets::TabsBuilder,
    crate::widgets::TextEditBuilder,
    crate::widgets::TextFieldBuilder,
    crate::widgets::ToggleBuilder,
    crate::widgets::TooltipBuilder,
);

pub(crate) trait ErasedComponent {
    fn build_box(self: Box<Self>, cx: &mut Cx) -> NodeId;
}

impl<T> ErasedComponent for T
where
    T: IntoNode,
{
    fn build_box(self: Box<Self>, cx: &mut Cx) -> NodeId {
        IntoNodeBox::build_box(self, cx)
    }
}
