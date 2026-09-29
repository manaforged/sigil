use crate::kernel::{NodeBuilder, container};
use crate::layout::LayoutReactiveExt;
use crate::layout::LayoutStyle;
use crate::layout::{LayoutBuilderExt, Position};
use crate::reactive::{Signal, derived, signal};
use crate::style::{NodeStyleExt, NumericAtomKind};

use std::ops::Deref;

use super::keyboard::{KeyboardRuntime, attach_key_handlers};
use super::measure::Measure;
use super::render::{content_container, viewport_container, width_apply, width_spec};
use super::scroll::{self, apply_bound_scroll, attach_wheel};
use super::selection::{SelectionRuntime, SelectionState};
use super::strip::{BoundStrip, bound_strip};
use super::viewport::{ViewportSize, apply_viewport_overrides};
use super::window::{self, strip_height};
use super::{
    BoundCtx, BoundItems, BoundVisibleThunk, BuildAxes, DEFAULT_ROW_HEIGHT,
    DEFAULT_VIEWPORT_HEIGHT, Sizing, VirtualListBuilder, bound_slice,
};

impl<T, R> VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
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
        let initial_rows = bound_slice(&signal.peek()).len();
        let live_rows = derived(move || bound_slice(&signal.get()).len());
        let thunk: Box<BoundVisibleThunk<T, R>> = Box::new(move |ctx: BoundCtx<T, R>| {
            let BoundCtx {
                render,
                row_height,
                width,
                selection,
                scroll_y,
                viewport_height,
                overscan,
                row_key,
                measure,
                tops,
            } = ctx;
            let render = std::rc::Rc::new(render);
            container()
                .layout(width_apply(width, LayoutStyle::block()))
                .child(
                    content_container(strip_height(viewport_height, row_height, overscan), width)
                        .child(bound_strip(
                            signal,
                            scroll_y,
                            BoundStrip {
                                render: std::rc::Rc::clone(&render),
                                row_height,
                                viewport_height,
                                overscan,
                                width,
                                selection,
                                row_key: row_key.clone(),
                                measure,
                                tops,
                            },
                        )),
                )
        });
        self.bound = Some(BoundItems {
            thunk,
            measure: Some(Box::new(move |row_key, estimate| {
                Measure::new(signal, row_key, estimate)
            })),
            initial_rows,
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
        self.build_bound(axes, measure).size_reactive(axes.viewport)
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
        let shrinks = self.shrinks();
        let selection = SelectionRuntime::new(axes.selection_state);
        let content = (bound.thunk)(BoundCtx {
            render: self.render,
            row_height: axes.row_height,
            width: axes.width,
            selection,
            scroll_y: axes.scroll_y,
            viewport_height: axes.viewport_height,
            overscan: self.overscan,
            row_key: self.row_key,
            measure,
            tops: axes.tops,
        });
        let initial_rows = bound.initial_rows;
        let live_rows = bound.live_rows;
        apply_bound_scroll(axes, self.initial_scroll, self.pin_bottom);
        let root = with_scrollbar(
            self.scrollbar,
            viewport_container(axes.viewport_height, axes.width).child(content),
            axes,
        );
        let root = apply_viewport_overrides(
            root,
            &self.layout_overrides,
            ViewportSize {
                height: axes.viewport_height,
                width: self.viewport_width,
                fill: self.sizing == Sizing::Fill,
            },
        );
        let root = shrink_bound(root, axes, shrinks);
        let root = attach_wheel(root, axes);
        if self.keyboard_enabled {
            attach_key_handlers(
                root,
                KeyboardRuntime::new(axes, initial_rows, Some(live_rows)),
            )
        } else {
            root
        }
    }

    const fn shrinks(&self) -> bool {
        matches!(self.sizing, Sizing::ShrinkToContent)
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
    let viewport_height = derived(move || {
        let measured = axes.viewport.get().1;
        if measured > 0.0 {
            measured
        } else {
            axes.viewport_height
        }
    });
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
