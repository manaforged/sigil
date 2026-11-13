use crate::kernel::NodeBuilder;
use crate::layout::{LayoutStyle, Length};
use crate::style::{Paint, StateStyleBuilder, States, Style};
use crate::widgets::Widget;
use crate::widgets::layout_ext::LayoutOverrides;
use crate::widgets::shared::caller_style;

/// Accumulates caller style, layout overrides, and node hooks for a custom widget builder.
#[must_use]
pub struct WidgetFrame {
    pub(crate) style: Style,
    pub(crate) layout: LayoutOverrides,
}

impl WidgetFrame {
    /// Creates an empty frame.
    pub const fn new() -> Self {
        Self {
            style: Style::new(),
            layout: LayoutOverrides::new(),
        }
    }

    /// Returns the explicit caller width when one was set.
    #[must_use]
    pub fn width(&self) -> Option<Length> {
        match self.layout.apply(LayoutStyle::default()).0.size.width {
            Length::Auto => None,
            width => Some(width),
        }
    }

    /// Returns the explicit caller height when one was set.
    #[must_use]
    pub fn height(&self) -> Option<Length> {
        match self.layout.apply(LayoutStyle::default()).0.size.height {
            Length::Auto => None,
            height => Some(height),
        }
    }

    /// Reports whether the caller set a fill width.
    #[must_use]
    pub fn fills_width(&self) -> bool {
        self.layout.fills_width()
    }

    /// Reports whether the caller set a fill height.
    #[must_use]
    pub fn fills_height(&self) -> bool {
        self.layout.fills_height()
    }

    /// Applies the accumulated caller style, layout overrides, and hooks onto `root`.
    pub fn finish(self, root: NodeBuilder) -> NodeBuilder {
        self.layout.finish(caller_style(root, self.style))
    }
}

impl Default for WidgetFrame {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for WidgetFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WidgetFrame")
            .field("style", &self.style)
            .field("layout", &self.layout)
            .finish_non_exhaustive()
    }
}

/// Grants `Paint`, `States`, and `Widget` implementations through a `WidgetFrame`.
pub trait Framed: Sized {
    /// Returns the mutable widget frame for this builder.
    fn frame_mut(&mut self) -> &mut WidgetFrame;
}

impl<T: Framed> Paint for T {
    fn paint_mut(&mut self) -> &mut StateStyleBuilder {
        self.frame_mut().style.paint_mut()
    }
}

impl<T: Framed> States for T {
    crate::style::state_methods!(style frame_mut().style);
}

impl<T: Framed> Widget for T {
    fn map_node(mut self, f: impl FnOnce(NodeBuilder) -> NodeBuilder + 'static) -> Self {
        self.frame_mut().layout.node_hook(f);
        self
    }

    crate::widgets::widget_layout_methods!(@edit frame_mut().layout.edit);
}

crate::widgets::impl_layout!([T: Framed] T, @edit frame_mut().layout.edit);
