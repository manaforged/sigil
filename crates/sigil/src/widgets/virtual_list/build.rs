use crate::kernel::{Cx, IntoNode, NodeBuilder, NodeId, container};
use crate::layout::LayoutReactiveExt;
use crate::layout::{Dimension, LayoutStyle, Overflow};
use crate::layout::{LayoutBuilderExt, Position};
use crate::reactive::{Signal, derived, signal};
use crate::style::{NodeStyleExt, NumericAtomKind};

use std::ops::Deref;

use super::keyboard::{KeyboardRuntime, attach_key_handlers};
use super::measure::Measure;
use super::render::{content_container, viewport_container, width_spec};
use super::scroll::{self, apply_bound_scroll, attach_wheel};
use super::selection::{SelectionRuntime, SelectionState};
use super::strip::{BoundStrip, bound_strip};
use super::window::{self, strip_height};
use super::{
    BoundItems, BoundVisibleThunk, BuildAxes, DEFAULT_ROW_HEIGHT, DEFAULT_VIEWPORT_HEIGHT,
    RowRender, Sizing, VirtualListBuilder, bound_slice,
};

impl<T, R> VirtualListBuilder<T, R>
where
    T: 'static,
    R: RowRender<T>,
{
    #[must_use]
    pub const fn scrollbar(mut self, on: bool) -> Self {
        self.scrollbar = on;
        self
    }

    pub(super) fn bind_items<S>(mut self, signal: Signal<S>) -> Self
    where
        S: Deref + Clone + 'static,
        S::Target: AsRef<[T]>,
    {
        let live_rows = derived(move || bound_slice(&signal.get()).len());
        let thunk: Box<BoundVisibleThunk<T, R>> =
            Box::new(move |spec: BoundStrip<T, R>, scroll_y| {
                let height = strip_height(spec.viewport_height, spec.row_height, spec.overscan);
                container()
                    .layout(spec.width.apply(LayoutStyle::block()))
                    .child(
                        content_container(height, spec.width)
                            .child(bound_strip(signal, scroll_y, spec)),
                    )
            });
        self.bound = Some(BoundItems {
            thunk,
            measure: Some(Box::new(move |row_key, estimate| {
                Measure::new(signal, row_key, estimate)
            })),
            live_rows,
            count: std::rc::Rc::new(move || bound_slice(&signal.peek()).len()),
        });
        self
    }

    fn take_measure(&mut self, estimate: f32) -> Option<Measure> {
        let make = self.bound.as_mut()?.measure.take()?;
        self.measured.then(|| make(self.row_key.clone(), estimate))
    }

    pub(super) fn into_node_builder(mut self) -> NodeBuilder {
        if self.bound.is_none() {
            if self.item_height.is_some() && !self.measured {
                self.ensure_row_tops();
            }
            let items = signal(std::rc::Rc::new(std::mem::take(&mut self.items)));
            return self.bind_items(items).into_node_builder();
        }
        let (axes, measure) = self.axes();
        let count = self
            .bound
            .as_ref()
            .map(|bound| std::rc::Rc::clone(&bound.count));
        scroll::bind_extent(axes, self.content_height_bind, self.max_scroll_bind);
        scroll::anchor(axes);
        if let Some(target) = self.follow {
            scroll::follow(target, axes, move || {
                count.as_ref().map_or(0, |count| count())
            });
        }
        let paint = std::mem::replace(&mut self.style_accumulator, crate::style::Style::new());
        let root = self.build_bound(axes, measure);
        let root = if paint.is_empty() {
            root
        } else {
            root.style(paint)
        };
        root.size_reactive(axes.viewport)
    }

    fn axes(&mut self) -> (BuildAxes, Option<Measure>) {
        let row_height = if self.measured {
            window::row_px(self.estimate.unwrap_or(self.row_height))
        } else {
            window::row_px(self.row_height)
        };
        let viewport_height = window::positive_px(self.viewport_height, DEFAULT_VIEWPORT_HEIGHT);
        let measure = self.take_measure(row_height);
        let tops = measure.as_ref().map(|measure| measure.tops).or_else(|| {
            (!self.row_tops.is_empty())
                .then(|| signal(std::rc::Rc::new(std::mem::take(&mut self.row_tops))))
        });
        let keys = measure.as_ref().and_then(|measure| measure.keys);
        let live_rows = self.bound.as_ref().map(|bound| bound.live_rows);
        let viewport = signal((0.0, 0.0));
        let (content, max) = scroll::extent(viewport, viewport_height, move || {
            let rows = live_rows.map_or(0, |rows| rows.get());
            scroll::content_px(tops, row_height, rows)
        });
        let axes = BuildAxes {
            row_height,
            viewport_height,
            width: width_spec(self.viewport_width),
            scroll_y: self.scroll_y.take().unwrap_or_else(|| signal(0.0)),
            scroll_x: self.scroll_x,
            focused_row: self.focused_row.take().unwrap_or_else(|| signal(0)),
            selection_state: self.ensure_selection_state(),
            viewport,
            tops,
            keys,
            content,
            max,
        };
        (axes, measure)
    }

    fn build_bound(mut self, axes: BuildAxes, measure: Option<Measure>) -> NodeBuilder {
        let bound = self
            .bound
            .take()
            .expect("build_bound called without virtual_list_bind");
        let content = (bound.thunk)(
            BoundStrip {
                render: std::rc::Rc::new(self.render),
                row_height: axes.row_height,
                viewport_height: axes.viewport_height,
                overscan: self.overscan,
                width: axes.width,
                selection: SelectionRuntime::new(axes.selection_state),
                row_key: self.row_key,
                measure,
                tops: axes.tops,
            },
            axes.scroll_y,
        );
        apply_bound_scroll(axes, self.initial_scroll, self.pin_bottom);
        let root = with_scrollbar(
            self.scrollbar,
            viewport_container(axes.viewport_height, axes.width).child(content),
            axes,
        );
        let root = apply_viewport_overrides(
            root,
            &self.layout_overrides,
            axes.viewport_height,
            self.viewport_width,
            self.sizing == Sizing::Fill,
        );
        let root = shrink_bound(root, axes, self.sizing == Sizing::ShrinkToContent);
        let root = attach_wheel(root, axes);
        if self.keyboard_enabled {
            attach_key_handlers(root, KeyboardRuntime::new(axes, bound.live_rows))
        } else {
            root
        }
    }

    fn ensure_row_tops(&mut self) {
        if self.row_tops.len() == self.items.len().saturating_add(1) {
            return;
        }
        self.row_tops.clear();
        self.row_tops.reserve(self.items.len().saturating_add(1));
        self.row_tops.push(0.0);
        let Some(item_height) = &self.item_height else {
            return;
        };
        let mut cursor = 0.0;
        for (index, item) in self.items.iter().enumerate() {
            cursor += window::positive_px(item_height(index, item), DEFAULT_ROW_HEIGHT);
            self.row_tops.push(cursor);
        }
    }

    pub(super) fn ensure_selection_state(&mut self) -> Signal<SelectionState> {
        if let Some(state) = self.selection_state {
            state.update(|state| {
                if state.mode() != self.selection_mode {
                    *state = SelectionState::new(self.selection_mode);
                }
            });
            state
        } else {
            let state = signal(SelectionState::new(self.selection_mode));
            self.selection_state = Some(state);
            state
        }
    }
}

fn with_scrollbar(show: bool, root: NodeBuilder, axes: BuildAxes) -> NodeBuilder {
    if !show {
        return root;
    }
    let viewport_height =
        derived(move || scroll::viewport_px(axes.viewport.get(), axes.viewport_height));
    root.child(
        container()
            .layout(
                LayoutStyle::block()
                    .position(Position::Absolute)
                    .inset_right(0.0)
                    .inset_top(0.0)
                    .inset_bottom(0.0)
                    .width(crate::widgets::scroll::SCROLLBAR_WIDTH),
            )
            .child(crate::widgets::scroll::scrollbar(
                axes.scroll_y,
                axes.max,
                axes.content,
                viewport_height,
                true,
            )),
    )
}

fn apply_viewport_overrides(
    root: NodeBuilder,
    overrides: &super::LayoutOverrides,
    height: f32,
    width: Option<f32>,
    fill: bool,
) -> NodeBuilder {
    let base = LayoutStyle::block().overflow_y(Overflow::Hidden);
    let base = if fill {
        base.width(Dimension::Fill)
            .height(Dimension::Fill)
            .grow(1.0)
            .flex_basis(Dimension::Px(0.0))
            .min_size(0.0, 0.0)
    } else {
        let base = base.height(height);
        match width {
            Some(w) => base.width(w),
            None => base,
        }
    };
    root.with(overrides.clone().apply(base))
}

fn shrink_bound(root: NodeBuilder, axes: BuildAxes, shrinks: bool) -> NodeBuilder {
    if !shrinks {
        return root;
    }
    let height = derived(move || {
        let content = axes.content.get();
        if content > 0.0 {
            content.min(axes.viewport_height)
        } else {
            axes.viewport_height
        }
    });
    root.animate(NumericAtomKind::Height, height)
}

impl<T: 'static, R: RowRender<T>> IntoNode for VirtualListBuilder<T, R> {
    fn build(self, cx: &mut Cx) -> NodeId {
        self.layout_overrides
            .hooks()
            .finish(self.into_node_builder())
            .build(cx)
    }
}
