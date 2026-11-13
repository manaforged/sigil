use std::rc::Rc;

use crate::kernel::{NodeBuilder, Role, TextContent, text};
use crate::style::{StateStyle, StyleAtom, StyleVariant, TextDecoration, Tone};

use super::shared::{Activate, DIMMED, accessible_name, noop_activate};

const UNDERLINE: StyleAtom = StyleAtom::TextDecoration(TextDecoration::Underline);

static REST: &[StyleAtom] = &[Tone::Primary.step(11).text(), UNDERLINE];
static ACTIVE: &[StyleAtom] = &[Tone::Primary.step(12).text(), UNDERLINE];

static LINK: StyleVariant =
    StyleVariant::new(REST, StateStyle::new(&[], ACTIVE, ACTIVE, ACTIVE, DIMMED));

/// Builder of a link.
#[must_use]
pub struct LinkBuilder {
    label: TextContent,
    href: Option<String>,
    activate: Option<Activate>,
    layout_overrides: crate::widgets::layout_ext::LayoutOverrides,
}

impl core::fmt::Debug for LinkBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LinkBuilder")
            .field("label", &self.label)
            .field("href", &self.href)
            .field("layout_overrides", &self.layout_overrides)
            .finish_non_exhaustive()
    }
}

/// Focusable text that opens a URL or runs an action.
///
/// Click, Enter, or Space runs the action set with `activate`. Without one, a build with the
/// `desktop` or `terminal` feature opens `href` with the OS handler, on either host; a build
/// without a host does nothing. On the terminal host, `href` is also written as an OSC 8 hyperlink, so a terminal that
/// supports OSC 8 opens it on its own link gesture, and other terminals underline the label.
///
/// ```
/// use sigil::prelude::*;
/// use sigil::widgets::link;
///
/// fn docs() -> impl IntoNode {
///     let opened = signal(0);
///     link("Read the guide")
///         .href("https://docs.rs/sigil-ui")
///         .activate(move || opened.update(|n| *n += 1))
/// }
/// ```
pub fn link(label: impl Into<TextContent>) -> LinkBuilder {
    LinkBuilder {
        label: label.into(),
        href: None,
        activate: None,
        layout_overrides: crate::widgets::layout_ext::LayoutOverrides::new(),
    }
}

impl LinkBuilder {
    /// Sets the URL that the link opens and that screen readers and terminals report.
    pub fn href(mut self, href: impl Into<String>) -> Self {
        self.href = Some(href.into());
        self
    }

    /// Runs `activate` instead of opening the URL.
    pub fn activate(mut self, activate: impl Fn() + 'static) -> Self {
        self.activate = Some(Rc::new(activate));
        self
    }

    fn root(self) -> NodeBuilder {
        let name = accessible_name(&self.label);
        let activate = match (self.activate, &self.href) {
            (Some(activate), _) => activate,
            (None, Some(href)) => open_href(href.clone()),
            (None, None) => noop_activate(),
        };
        let node = text(self.label).nowrap().role(Role::Link).label(name);
        let node = match self.href {
            Some(href) => node.description(href),
            None => node,
        };
        node.node.variant(LINK).activate_with(activate)
    }
}

#[cfg(app_host)]
fn open_href(href: String) -> Activate {
    Rc::new(move || {
        if let Err(error) = crate::app::open_url(&href) {
            tracing::warn!(target: "sigil::link", %href, %error, "link did not open");
        }
    })
}

#[cfg(not(app_host))]
fn open_href(_href: String) -> Activate {
    noop_activate()
}

builder_impls!([] LinkBuilder: widget, node(pub));
