//! Splitter builder.
//!
//! Public surface: [`splitter`] + [`SplitterBuilder`]. The builder collects
//! per-pane children, initial sizes, and per-pane constraints, then
//! materialises a flex container with alternating pane nodes and drag handles.
//!
//! Drag handles are 6 px wide (or tall for vertical splitters) and capture
//! the pointer on mouse-down. All drag logic lives in the [`super::drag`]
//! module so this file stays focused on composition.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use kernel::{Cx, IntoNode, NodeBuilder, Role, container};
use layout::prelude::{AlignItems, LayoutBuilderExt, LayoutStyle};
use style::{Dimension as StyleDimension, NodeStyleExt, Px, StyleAtom};

use super::super::layout_ext::{LayoutOverrides, WidgetLayoutExt};
use super::drag::{SplitterState, install_handle_hooks};

/// Axis orientation for the splitter container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitterOrientation {
    /// Panes laid out left-to-right; drag handles are vertical bars.
    Horizontal,
    /// Panes laid out top-to-bottom; drag handles are horizontal bars.
    Vertical,
}

/// Handle thickness in logical pixels.
const HANDLE_PX: f32 = 6.0;
/// Atom cache capacity (number of distinct pixel sizes to keep).
const ATOM_CACHE_CAP: usize = 512;

// ---- per-pane entry ---------------------------------------------------------

struct PaneEntry {
    content: Box<dyn IntoNode>,
    initial_size: f32,
    min: f32,
    max: f32,
}

// ---- builder ----------------------------------------------------------------

/// Builder returned by [`splitter`].
pub struct SplitterBuilder {
    orientation: SplitterOrientation,
    panes: Vec<PaneEntry>,
    sizes_bind: Option<reactive::Signal<Vec<f32>>>,
    snap_points: Vec<(usize, Vec<f32>)>,
    snap_threshold: f32,
    layout_overrides: LayoutOverrides,
}

/// Create a splitter widget.
///
/// Call `.child(node, initial_size)` to add each pane, then optionally
/// `.min_size(idx, px)` / `.max_size(idx, px)` for constraints.
#[must_use]
pub const fn splitter(orientation: SplitterOrientation) -> SplitterBuilder {
    SplitterBuilder {
        orientation,
        panes: Vec::new(),
        sizes_bind: None,
        snap_points: Vec::new(),
        snap_threshold: 8.0,
        layout_overrides: LayoutOverrides::new(),
    }
}

impl SplitterBuilder {
    /// Append a pane with an initial size in logical pixels.
    #[must_use]
    pub fn child(mut self, node: impl IntoNode + 'static, initial_size: f32) -> Self {
        self.panes.push(PaneEntry {
            content: Box::new(node),
            initial_size: initial_size.max(0.0),
            min: 0.0,
            max: f32::INFINITY,
        });
        self
    }

    /// Start a per-pane builder for `node`.
    ///
    /// Chain `.initial(px)`, `.min(px)`, `.max(px)` on the returned
    /// [`PaneBuilder`]; the pane is pushed onto the splitter when the
    /// builder is dropped (or [`PaneBuilder::build`] is called
    /// explicitly). Replaces the positional
    /// `.child(node, initial).min_size(idx, px).max_size(idx, px)`
    /// triplet with a single fluent block; the positional API stays for
    /// back-compat.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use sigil::widgets::{SplitterOrientation, splitter};
    /// use kernel::container;
    ///
    /// let _node = splitter(SplitterOrientation::Horizontal)
    ///     .pane(container()).initial(240.0).min(120.0).max(480.0)
    ///     .pane(container()).initial(360.0).min(200.0);
    /// ```
    pub fn pane(self, node: impl IntoNode + 'static) -> PaneBuilder {
        PaneBuilder {
            splitter: self,
            entry: PaneEntry {
                content: Box::new(node),
                initial_size: 0.0,
                min: 0.0,
                max: f32::INFINITY,
            },
        }
    }

    /// Set the minimum size for the pane at `idx`.
    #[must_use]
    pub fn min_size(mut self, idx: usize, px: f32) -> Self {
        if let Some(pane) = self.panes.get_mut(idx) {
            pane.min = px.max(0.0);
        }
        self
    }

    /// Set the maximum size for the pane at `idx`.
    #[must_use]
    pub fn max_size(mut self, idx: usize, px: f32) -> Self {
        if let Some(pane) = self.panes.get_mut(idx) {
            pane.max = px.max(0.0);
        }
        self
    }

    /// Bind an external signal that receives the live pane-sizes vector.
    #[must_use]
    pub const fn sizes_bind(mut self, signal: reactive::Signal<Vec<f32>>) -> Self {
        self.sizes_bind = Some(signal);
        self
    }

    /// Register snap points for the handle between pane `handle_idx` and
    /// `handle_idx + 1`. On pointer-up, if the current split is within
    /// `snap_threshold` of any point, it snaps to that point.
    #[must_use]
    pub fn snap_points(mut self, handle_idx: usize, points: &[f32]) -> Self {
        self.snap_points.push((handle_idx, points.to_vec()));
        self
    }

    /// Set the snap pull threshold in pixels (default 8.0).
    #[must_use]
    pub const fn snap_threshold(mut self, threshold: f32) -> Self {
        self.snap_threshold = threshold.max(0.0);
        self
    }

    fn into_node_builder(self) -> NodeBuilder {
        let orientation = self.orientation;
        let count = self.panes.len();

        let sizes: Vec<f32> = self.panes.iter().map(|p| p.initial_size).collect();
        let mins: Vec<f32> = self.panes.iter().map(|p| p.min).collect();
        let maxs: Vec<f32> = self.panes.iter().map(|p| p.max).collect();

        let state = SplitterState::new(orientation, sizes, mins, maxs);

        for (handle_idx, pts) in self.snap_points {
            state.set_snap(handle_idx, pts);
        }
        state.set_snap_threshold(self.snap_threshold);

        if let Some(signal) = self.sizes_bind {
            state.bind_sizes(signal);
        }

        let root_layout = match orientation {
            SplitterOrientation::Horizontal => {
                LayoutStyle::flex_row().align_items(AlignItems::Stretch)
            }
            SplitterOrientation::Vertical => {
                LayoutStyle::flex_column().align_items(AlignItems::Stretch)
            }
        };

        let mut root = container()
            .role(Role::GenericContainer)
            .layout(root_layout.clone());

        for (pane_idx, entry) in self.panes.into_iter().enumerate() {
            root = root.child(build_pane(entry.content, pane_idx, orientation, state));
            if pane_idx + 1 < count {
                root = root.child(build_handle(pane_idx, orientation, state));
            }
        }

        root.layout(self.layout_overrides.apply(root_layout))
    }
}

impl IntoNode for SplitterBuilder {
    fn build(self, cx: &mut Cx) -> kernel::NodeId {
        self.into_node_builder().build(cx)
    }
}

/// Per-pane builder returned by [`SplitterBuilder::pane`].
///
/// Chain `.initial(px)` / `.min(px)` / `.max(px)` to set the pane's
/// constraints, then either call [`PaneBuilder::pane`] to start another
/// pane or pass the builder to `cx.add(...)` to materialise the
/// splitter (which commits the in-flight pane via [`IntoNode`]).
///
/// Call [`PaneBuilder::commit`] to return the underlying
/// [`SplitterBuilder`] when you need to chain non-pane setters
/// (`sizes_bind`, `snap_points`, `snap_threshold`, layout overrides).
#[must_use = "PaneBuilder must be committed by calling .commit(), .pane(...), or by passing it to cx.add()"]
pub struct PaneBuilder {
    splitter: SplitterBuilder,
    entry: PaneEntry,
}

impl PaneBuilder {
    /// Set this pane's initial size in logical pixels.
    pub const fn initial(mut self, px: f32) -> Self {
        self.entry.initial_size = px.max(0.0);
        self
    }

    /// Set this pane's minimum size in logical pixels.
    pub const fn min(mut self, px: f32) -> Self {
        self.entry.min = px.max(0.0);
        self
    }

    /// Set this pane's maximum size in logical pixels.
    pub const fn max(mut self, px: f32) -> Self {
        self.entry.max = px.max(0.0);
        self
    }

    /// Commit the in-flight pane and start another one.
    pub fn pane(self, node: impl IntoNode + 'static) -> Self {
        self.commit().pane(node)
    }

    /// Commit the in-flight pane and return the underlying
    /// [`SplitterBuilder`] for further configuration.
    #[must_use]
    pub fn commit(mut self) -> SplitterBuilder {
        self.splitter.panes.push(self.entry);
        self.splitter
    }
}

impl IntoNode for PaneBuilder {
    fn build(self, cx: &mut Cx) -> kernel::NodeId {
        self.commit().build(cx)
    }
}

impl WidgetLayoutExt for SplitterBuilder {
    fn layout_overrides_mut(&mut self) -> &mut LayoutOverrides {
        &mut self.layout_overrides
    }
}

// ---- pane node --------------------------------------------------------------

fn build_pane(
    content: Box<dyn IntoNode>,
    pane_idx: usize,
    orientation: SplitterOrientation,
    state: SplitterState,
) -> NodeBuilder {
    let initial_px = state.size_at(pane_idx);
    let initial_layout = pane_layout(orientation, initial_px);
    container()
        .role(Role::GenericContainer)
        .layout(initial_layout)
        .style_reactive(move || pane_size_atom(orientation, state.size_at(pane_idx)))
        .child(content)
}

/// Build a layout for a pane using `f32` size (what `LayoutStyle::width/height`
/// accept — they impl `From<f32>` for the internal `layout::Dimension`).
fn pane_layout(orientation: SplitterOrientation, size_px: f32) -> LayoutStyle {
    match orientation {
        SplitterOrientation::Horizontal => LayoutStyle::flex_column().width(size_px),
        SplitterOrientation::Vertical => LayoutStyle::flex_column().height(size_px),
    }
}

fn pane_size_atom(orientation: SplitterOrientation, size_px: f32) -> &'static [StyleAtom] {
    let px = to_i16(size_px);
    match orientation {
        SplitterOrientation::Horizontal => cached_width_atom(px),
        SplitterOrientation::Vertical => cached_height_atom(px),
    }
}

// ---- drag handle node -------------------------------------------------------

fn build_handle(
    handle_idx: usize,
    orientation: SplitterOrientation,
    state: SplitterState,
) -> NodeBuilder {
    // The handle itself must be the deepest hittable node (no hittable children)
    // so the router captures pointer-down on the handle — per Phase 2.3 lesson.
    let layout = handle_layout(orientation);
    let node = container()
        .role(Role::Splitter)
        .layout(layout)
        .class(&[StyleAtom::BackgroundThemeRef("border")] as &[_]);
    install_handle_hooks(node, state, handle_idx)
}

/// `LayoutStyle::width/height` accept `f32` directly.
fn handle_layout(orientation: SplitterOrientation) -> LayoutStyle {
    match orientation {
        SplitterOrientation::Horizontal => LayoutStyle::flex_column()
            .width(HANDLE_PX)
            .align_items(AlignItems::Stretch),
        SplitterOrientation::Vertical => LayoutStyle::flex_column()
            .height(HANDLE_PX)
            .align_items(AlignItems::Stretch),
    }
}

// ---- atom cache helpers -----------------------------------------------------

/// Cache `StyleAtom::Width` slices keyed by i16 pixel value.
/// `style::Dimension::Px(Px(i16))` is the correct atom type.
fn cached_width_atom(px: i16) -> &'static [StyleAtom] {
    static CACHE: OnceLock<Mutex<HashMap<i16, &'static [StyleAtom]>>> = OnceLock::new();
    cached_atom(&CACHE, px, |p| {
        vec![StyleAtom::Width(StyleDimension::Px(Px(p)))]
    })
}

fn cached_height_atom(px: i16) -> &'static [StyleAtom] {
    static CACHE: OnceLock<Mutex<HashMap<i16, &'static [StyleAtom]>>> = OnceLock::new();
    cached_atom(&CACHE, px, |p| {
        vec![StyleAtom::Height(StyleDimension::Px(Px(p)))]
    })
}

fn cached_atom(
    cache: &'static OnceLock<Mutex<HashMap<i16, &'static [StyleAtom]>>>,
    key: i16,
    build: impl FnOnce(i16) -> Vec<StyleAtom>,
) -> &'static [StyleAtom] {
    let map = cache.get_or_init(|| Mutex::new(HashMap::new()));
    let Ok(mut guard) = map.lock() else {
        return Box::leak(build(key).into_boxed_slice());
    };
    if let Some(atoms) = guard.get(&key).copied() {
        return atoms;
    }
    let leaked: &'static [StyleAtom] = Box::leak(build(key).into_boxed_slice());
    if guard.len() < ATOM_CACHE_CAP {
        guard.insert(key, leaked);
    }
    leaked
}

/// Clamp an `f32` pixel value to the `i16` range for style atoms.
fn to_i16(px: f32) -> i16 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let clamped = px.clamp(f32::from(i16::MIN), f32::from(i16::MAX)).round() as i16;
    clamped
}
