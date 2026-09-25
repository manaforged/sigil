use crate::kernel::{NodeBuilder, container};
use crate::layout::LayoutReactiveExt;
use crate::layout::LayoutStyle;
use crate::layout::prelude::LayoutBuilderExt;
use crate::reactive::{Signal, derived_eq, signal};
use crate::style::{NodeStyleExt, NumericAtomKind};

use std::ops::Deref;

use super::keyboard::{KeyboardRuntime, attach_key_handlers};
use super::measure::Measure;
use super::render::{
    HeightSpec, VisibleRenderSpec, build_visible_window, content_container, viewport_container,
    width_apply, width_spec,
};
use super::scroll::{self, apply_bound_scroll, attach_wheel};
use super::selection::{SelectionRuntime, SelectionState};
use super::strip::{BoundStrip, bound_strip};
use super::viewport::{ViewportSize, apply_viewport_overrides};
use super::window::{
    self, VisibleWindow, prefix_row_top, row_top, strip_height, total_height,
    total_height_variable, visible_window, visible_window_variable,
};
use super::{
    BoundCtx, BoundItems, BoundVisibleThunk, BuildAxes, DEFAULT_ROW_HEIGHT,
    DEFAULT_VIEWPORT_HEIGHT, InitialScroll, Sizing, VirtualListBuilder, bound_slice,
};

impl<T, R> VirtualListBuilder<T, R>
where
    T: 'static,
    R: Fn(usize, &T) -> NodeBuilder + 'static,
{
    pub(super) fn bind_items<S>(mut self, signal: Signal<S>) -> Self
    where
        S: Deref + Clone + 'static,
        S::Target: AsRef<[T]>,
    {
        let initial_rows = bound_slice(&signal.peek()).len();
        let live_rows = derived_eq(move || bound_slice(&signal.get()).len());
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
        if self.measured && self.bound.is_none() {
            let items = signal(std::rc::Rc::new(std::mem::take(&mut self.items)));
            return self.bind_items(items).into_node_builder();
        }
        let (axes, measure) = self.axes();
        let count = self
            .bound
            .as_ref()
            .map(|bound| std::rc::Rc::clone(&bound.count));
        let static_rows = self.items.len();
        scroll::bind_extent(axes, self.content_height_bind, self.max_scroll_bind);
        scroll::anchor(axes);
        if let Some(target) = self.follow {
            scroll::follow(target, axes, move || {
                count.as_ref().map_or(static_rows, |count| count())
            });
        }
        let root = if self.bound.is_some() {
            self.build_bound(axes, measure)
        } else {
            self.build_static(axes)
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
            (self.bound.is_none() && self.item_height.is_some()).then(|| {
                self.ensure_row_tops();
                signal(std::rc::Rc::new(self.row_tops.clone()))
            })
        });
        let live_rows = self.bound.as_ref().map(|bound| bound.live_rows);
        let static_rows = self.items.len();
        let viewport = signal((0.0, 0.0));
        let (content, max) = scroll::extent(viewport, viewport_height, move || {
            let rows = live_rows.map_or(static_rows, |rows| rows.get());
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
        });
        let initial_rows = bound.initial_rows;
        let live_rows = bound.live_rows;
        apply_bound_scroll(axes, self.initial_scroll, self.pin_bottom);
        let root = viewport_container(axes.viewport_height, axes.width).child(content);
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

    fn build_static(mut self, axes: BuildAxes) -> NodeBuilder {
        let using_variable_heights =
            self.prepare_scroll(axes.scroll_y, axes.row_height, axes.viewport_height);
        let window = self.current_window(
            axes.scroll_y.get(),
            axes.row_height,
            axes.viewport_height,
            using_variable_heights,
        );
        let content_height = self.content_height(axes.row_height, using_variable_heights);
        let heights = self.height_spec(axes.row_height, using_variable_heights);
        let visible = build_visible_window(
            &self.items,
            &self.render,
            window,
            VisibleRenderSpec::new(
                heights,
                axes.width,
                SelectionRuntime::new(axes.selection_state),
                axes.scroll_y.get(),
            ),
        );
        let content = content_container(content_height, axes.width).child(visible);
        let root = viewport_container(axes.viewport_height, axes.width).child(content);
        let root = apply_viewport_overrides(
            root,
            &self.layout_overrides,
            ViewportSize {
                height: self.shrunk_height(content_height, axes.viewport_height),
                width: self.viewport_width,
                fill: self.sizing == Sizing::Fill,
            },
        );
        let root = attach_wheel(root, axes);
        if self.keyboard_enabled {
            attach_key_handlers(root, KeyboardRuntime::new(axes, self.items.len(), None))
        } else {
            root
        }
    }

    const fn shrinks(&self) -> bool {
        matches!(self.sizing, Sizing::ShrinkToContent)
    }

    fn shrunk_height(&self, content_height: f32, viewport_height: f32) -> f32 {
        if self.shrinks() && content_height > 0.0 {
            content_height.min(viewport_height)
        } else {
            viewport_height
        }
    }

    fn current_window(
        &self,
        scroll_y: f32,
        row_height: f32,
        vh: f32,
        variable: bool,
    ) -> VisibleWindow {
        if variable {
            visible_window_variable(scroll_y, &self.row_tops, vh, self.overscan)
        } else {
            visible_window(scroll_y, self.items.len(), row_height, vh, self.overscan)
        }
    }

    fn content_height(&self, row_height: f32, variable: bool) -> f32 {
        if variable {
            total_height_variable(&self.row_tops)
        } else {
            total_height(self.items.len(), row_height)
        }
    }

    fn height_spec(&self, row_height: f32, variable: bool) -> HeightSpec<'_> {
        (row_height, variable.then_some(self.row_tops.as_slice()))
    }

    fn prepare_scroll(&mut self, scroll_y: Signal<f32>, row_height: f32, vh: f32) -> bool {
        let variable = self.item_height.is_some();
        if variable {
            self.ensure_row_tops();
        }
        let Some(initial) = self.initial_scroll else {
            return variable;
        };
        let max = (self.content_height(row_height, variable) - vh).max(0.0);
        let target = match initial {
            InitialScroll::End => max,
            InitialScroll::Row(index) if variable => prefix_row_top(&self.row_tops, index),
            InitialScroll::Row(index) => row_top(index, row_height),
        };
        scroll_y.set(target.clamp(0.0, max));
        variable
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

fn shrink_bound(root: NodeBuilder, axes: BuildAxes, shrinks: bool) -> NodeBuilder {
    if !shrinks {
        return root;
    }
    let height = derived_eq(move || {
        let content = axes.content.get();
        if content > 0.0 {
            content.min(axes.viewport_height)
        } else {
            axes.viewport_height
        }
    });
    root.animate(NumericAtomKind::Height, height)
}
