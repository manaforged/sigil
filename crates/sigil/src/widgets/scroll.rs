//! Scroll widget — viewport clip with reactive scroll position.
//!
//! Content is offset by a negative `MarginTop` (or `MarginLeft` for horizontal
//! scrolling) driven by a `Signal<f32>`. Viewport clipping relies on layout
//! `overflow: hidden` via Taffy's escape hatch; visual clipping is deferred
//! to a future renderer integration pass.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use sigil_events::prelude::{Key, KeyEvent, NodeEventsExt};
use sigil_kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use sigil_layout::prelude::{LayoutBuilderExt, LayoutStyle, Position};
use sigil_reactive::{Signal, derived_eq, signal};
use sigil_style::{NodeStyleExt, Px, StyleAtom};
use taffy::Overflow;

const VIEWPORT_WIDTH: f32 = 320.0;
const VIEWPORT_HEIGHT: f32 = 240.0;
const SCROLLBAR_WIDTH: f32 = 8.0;
const THUMB_MIN_HEIGHT: f32 = 24.0;
const SCROLL_LINE: f32 = 40.0;

static VIEWPORT_ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("surface")];
static THUMB_ATOMS: &[StyleAtom] = &[StyleAtom::BackgroundThemeRef("accent")];

/// Builder returned by [`scroll`].
pub struct ScrollBuilder {
    content: NodeBuilder,
    content_height: f32,
    viewport_height: f32,
    /// Enable horizontal scrolling (default: vertical only).
    pub horizontal: bool,
    /// Enable kinetic momentum on release (default: true — stub until timer lands).
    pub momentum: bool,
}

/// Create a scroll widget that clips `content` to a vertical viewport.
///
/// `content_height` is the intrinsic height of the content in logical pixels.
/// `viewport_height` is the visible window height; defaults to 240 px.
#[must_use]
pub const fn scroll(content: NodeBuilder, content_height: f32) -> ScrollBuilder {
    ScrollBuilder::new(content, content_height)
}

impl ScrollBuilder {
    /// Create a scroll builder with defaults.
    #[must_use]
    pub const fn new(content: NodeBuilder, content_height: f32) -> Self {
        Self {
            content,
            content_height: content_height.max(1.0),
            viewport_height: VIEWPORT_HEIGHT,
            horizontal: false,
            momentum: true,
        }
    }

    /// Override the visible viewport height.
    #[must_use]
    pub const fn viewport_height(mut self, px: f32) -> Self {
        self.viewport_height = px;
        self
    }

    /// Enable or disable kinetic momentum on gesture release.
    #[must_use]
    pub const fn momentum(mut self, enabled: bool) -> Self {
        self.momentum = enabled;
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let scroll_y = signal(0.0_f32);
        let content_h = self.content_height;
        let viewport_h = self.viewport_height;
        let content = self.content;

        let max_scroll = (content_h - viewport_h).max(0.0);
        let content_with_offset = offset_content(content, scroll_y, content_h, viewport_h);
        let scrollbar = scrollbar_track(scroll_y, content_h, viewport_h);

        viewport_container(viewport_h)
            .child(content_with_offset)
            .child(scrollbar)
            .on_key(Key::ArrowDown, move |ev| {
                scroll_by(ev, scroll_y, SCROLL_LINE, max_scroll);
            })
            .on_key(Key::ArrowUp, move |ev| {
                scroll_by(ev, scroll_y, -SCROLL_LINE, max_scroll);
            })
            .on_key(Key::PageDown, move |ev| {
                scroll_by(ev, scroll_y, viewport_h, max_scroll);
            })
            .on_key(Key::PageUp, move |ev| {
                scroll_by(ev, scroll_y, -viewport_h, max_scroll);
            })
            .on_key(Key::Home, move |ev| {
                scroll_to(ev, scroll_y, 0.0);
            })
            .on_key(Key::End, move |ev| {
                scroll_to(ev, scroll_y, max_scroll);
            })
    }
}

impl Default for ScrollBuilder {
    fn default() -> Self {
        Self::new(container(), 0.0)
    }
}

impl IntoNode for ScrollBuilder {
    fn build(self, cx: &mut Cx) -> sigil_kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

fn viewport_container(viewport_h: f32) -> NodeBuilder {
    container()
        .role(Role::ScrollView)
        .layout(
            LayoutStyle::flex_row()
                .fixed_size(VIEWPORT_WIDTH, viewport_h)
                .with_taffy(|s| {
                    s.overflow.y = Overflow::Hidden;
                }),
        )
        .class(VIEWPORT_ATOMS)
}

fn offset_content(
    content: NodeBuilder,
    scroll_y: Signal<f32>,
    content_h: f32,
    viewport_h: f32,
) -> NodeBuilder {
    let max_scroll = (content_h - viewport_h).max(0.0);
    let offset_px = derived_eq(move || {
        let clamped = scroll_y.get().clamp(0.0, max_scroll);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = clamped.round() as i16;
        px
    });
    content
        .layout(LayoutStyle::block().fixed_size(VIEWPORT_WIDTH - SCROLLBAR_WIDTH, content_h))
        .style_reactive(move || margin_top_atoms_px(offset_px.get()))
}

fn scrollbar_track(scroll_y: Signal<f32>, content_h: f32, viewport_h: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::block()
                .fixed_size(SCROLLBAR_WIDTH, viewport_h)
                .position(Position::Relative),
        )
        .class(&[StyleAtom::BackgroundThemeRef("border")] as &[_])
        .child(scrollbar_thumb(scroll_y, content_h, viewport_h))
}

fn scrollbar_thumb(scroll_y: Signal<f32>, content_h: f32, viewport_h: f32) -> NodeBuilder {
    let ratio = (viewport_h / content_h).clamp(0.0, 1.0);
    let thumb_h = (ratio * viewport_h).max(THUMB_MIN_HEIGHT);
    let max_scroll = (content_h - viewport_h).max(0.0);
    let max_thumb_travel = viewport_h - thumb_h;

    let thumb_top = derived_eq(move || {
        let t = if max_scroll > 0.0 {
            scroll_y.get().clamp(0.0, max_scroll) / max_scroll
        } else {
            0.0
        };
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let px = (t * max_thumb_travel).round() as i16;
        px
    });

    container()
        .layout(LayoutStyle::block().fixed_size(SCROLLBAR_WIDTH, thumb_h))
        .class(THUMB_ATOMS)
        .style_reactive(move || margin_top_atoms_px(thumb_top.get()))
        .on_pointer_down(move |ev| {
            ev.capture_pointer();
        })
        .on_pointer_move(move |ev| {
            if !ev.is_captured() {
                return;
            }
            if max_thumb_travel <= 0.0 || max_scroll <= 0.0 {
                return;
            }
            let t = (ev.position.y / max_thumb_travel).clamp(0.0, 1.0);
            scroll_y.set(t * max_scroll);
            ev.stop_propagation();
        })
}

fn scroll_by(ev: &mut KeyEvent, scroll_y: Signal<f32>, delta: f32, max_scroll: f32) {
    ev.stop_propagation();
    let next = (scroll_y.peek() + delta).clamp(0.0, max_scroll);
    scroll_y.set(next);
}

fn scroll_to(ev: &mut KeyEvent, scroll_y: Signal<f32>, target: f32) {
    ev.stop_propagation();
    scroll_y.set(target);
}

fn margin_top_atoms_px(px: i16) -> &'static [StyleAtom] {
    cached_atoms(px, || vec![StyleAtom::MarginTop(Px(px.wrapping_neg()))])
}

fn cached_atoms(key: i16, build: impl FnOnce() -> Vec<StyleAtom>) -> &'static [StyleAtom] {
    static CACHE: OnceLock<Mutex<HashMap<i16, &'static [StyleAtom]>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut map) = cache.lock() else {
        return Box::leak(build().into_boxed_slice());
    };
    if let Some(atoms) = map.get(&key).copied() {
        return atoms;
    }
    let leaked: &'static [StyleAtom] = Box::leak(build().into_boxed_slice());
    map.insert(key, leaked);
    leaked
}
