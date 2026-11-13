use crate::kernel::{ImageSource, Load, NodeBuilder, ObjectFit, container, image, text};
use crate::layout::{LayoutStyle, Length};
use crate::reactive::Signal;
use crate::style::Style;
use crate::tasks::Status;
use crate::widgets::Widget;
use crate::widgets::layout_ext::LayoutOverrides;
use crate::{Error, ErrorKind};

use super::shared::caller_style;

type View = Box<dyn Fn() -> NodeBuilder>;
type ErrorView = Box<dyn Fn(&Error) -> NodeBuilder>;

/// An image that loads off the UI thread and shows a placeholder, then the image or an error.
#[must_use]
pub struct ImageLoadBuilder {
    status: Signal<Load>,
    alt: Option<String>,
    fit: ObjectFit,
    placeholder: View,
    error: ErrorView,
    style_accumulator: Style,
    layout_overrides: LayoutOverrides,
}

/// An image loaded from a file path, `file://` URL, base64 `data:` URI, or http(s) URL (feature `image-http` outside a browser).
///
/// Size the box with the layout setters, such as `.width(..)` and `.height(..)`; the loaded image fills a sized axis with `fit`, and keeps its own size on an axis left unset.
///
/// ```
/// use sigil::node::ObjectFit;
/// use sigil::prelude::*;
///
/// fn logo() -> impl IntoNode {
///     load_image("assets/logo.png")
///         .alt("Logo")
///         .fit(ObjectFit::Contain)
///         .placeholder(|| text("Loading logo"))
///         .error(|err| text(format!("No logo: {err}")))
/// }
/// ```
pub fn load_image(location: impl Into<String>) -> ImageLoadBuilder {
    ImageLoadBuilder {
        status: ImageSource::load(location),
        alt: None,
        fit: ObjectFit::Fill,
        placeholder: Box::new(|| container().child(text("Loading image"))),
        error: Box::new(|err| container().child(text(format!("Could not load image: {err}")))),
        style_accumulator: Style::new(),
        layout_overrides: LayoutOverrides::new(),
    }
}

impl ImageLoadBuilder {
    /// Sets the alt text of the loaded image.
    pub fn alt(mut self, text: impl Into<String>) -> Self {
        self.alt = Some(text.into());
        self
    }

    /// Sets how the loaded image fills a sized box.
    pub const fn fit(mut self, fit: ObjectFit) -> Self {
        self.fit = fit;
        self
    }

    /// Sets the view shown while the image loads.
    pub fn placeholder<N: crate::kernel::IntoNode + 'static>(
        mut self,
        view: impl Fn() -> N + 'static,
    ) -> Self {
        self.placeholder = Box::new(move || container().child(view()));
        self
    }

    /// Sets the view shown when the image fails to load, given the reason.
    pub fn error<N: crate::kernel::IntoNode + 'static>(
        mut self,
        view: impl Fn(&Error) -> N + 'static,
    ) -> Self {
        self.error = Box::new(move |err| container().child(view(err)));
        self
    }

    /// Returns the load status signal.
    #[must_use]
    pub const fn status(&self) -> Signal<Load> {
        self.status
    }

    fn root(self) -> NodeBuilder {
        let size = self
            .layout_overrides
            .apply(LayoutStyle::default())
            .data()
            .size;
        let fill = |set: bool| {
            if set {
                Length::Percent(1.0)
            } else {
                Length::Auto
            }
        };
        let (width, height) = (
            fill(size.width != Length::Auto),
            fill(size.height != Length::Auto),
        );
        let status = self.status;
        let node = container().children_reactive(move || {
            [match status.get() {
                Status::Ok(source) => {
                    let shown = image(source).fit(self.fit).width(width).height(height);
                    match &self.alt {
                        Some(alt) => shown.alt(alt.clone()),
                        None => shown,
                    }
                    .into_node_builder()
                }
                Status::Err(err) => (self.error)(&err),
                Status::Panicked(reason) => (self.error)(&Error::message(ErrorKind::Task, reason)),
                Status::Idle | Status::Pending => (self.placeholder)(),
            }]
        });
        caller_style(node, self.style_accumulator)
    }
}

impl core::fmt::Debug for ImageLoadBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ImageLoadBuilder")
            .field("status", &self.status)
            .field("alt", &self.alt)
            .field("fit", &self.fit)
            .field("style_accumulator", &self.style_accumulator)
            .finish_non_exhaustive()
    }
}

builder_impls!([] ImageLoadBuilder: paint(style_accumulator), widget, node);
