use std::cell::Cell;
use std::rc::Rc;

use crate::events::{ImeEvent, NodeEventsExt};
use crate::geometry::{Point, Rect};
use crate::hal::{HitPosition, TextAffinity};
use crate::kernel::NodeBuilder;
use crate::layout::{CaretGeometry, with_text_geometry};
use crate::reactive::{Signal, batch, effect, signal};

use super::handlers::TextEditRuntime;
use super::render::{floor_char_boundary, splice_preedit};

type CaretRectSlot = Rc<Cell<Option<Signal<Option<Rect>>>>>;

#[derive(Clone)]
pub(in crate::widgets) struct ImeState {
    pub(in crate::widgets) preedit: Signal<String>,
    preedit_cursor: Signal<Option<(usize, usize)>>,
    caret_anchor: Rc<Cell<Option<CaretGeometry>>>,
    caret_rect: CaretRectSlot,
}

impl ImeState {
    pub(super) fn new() -> Self {
        Self {
            preedit: signal(String::new()),
            preedit_cursor: signal(None),
            caret_anchor: Rc::new(Cell::new(None)),
            caret_rect: Rc::new(Cell::new(None)),
        }
    }

    pub(super) fn report_caret_rect(&self, target: Signal<Option<Rect>>) {
        self.caret_rect.set(Some(target));
    }

    pub(in crate::widgets) fn composing_caret(&self, value: &str, caret: usize) -> (String, usize) {
        let preedit = self.preedit.get();
        let base = floor_char_boundary(value, caret);
        if preedit.is_empty() {
            return (value.to_owned(), base);
        }
        let offset = self
            .preedit_cursor
            .get()
            .map_or(preedit.len(), |(_, end)| floor_char_boundary(&preedit, end));
        (splice_preedit(value, base, &preedit), base + offset)
    }
}

pub(in crate::widgets) fn attach_ime_handlers(
    node: NodeBuilder,
    runtime: &TextEditRuntime,
) -> NodeBuilder {
    let anchor = Rc::clone(&runtime.ime.caret_anchor);
    let event_rt = runtime.clone();
    let blur_rt = runtime.clone();
    let eligible_rt = runtime.clone();
    runtime.install_disable_clears_preedit();
    runtime.install_caret_anchor_sync();
    node.on_ime(move |event| on_ime_event(event, &event_rt))
        .focus_out(move || blur_rt.clear_preedit())
        .ime_eligible(move || !eligible_rt.disabled())
        .ime_caret(anchor)
}

fn on_ime_event(event: &ImeEvent, runtime: &TextEditRuntime) {
    if matches!(event, ImeEvent::Disabled) {
        runtime.clear_preedit();
        return;
    }
    if runtime.disabled() {
        return;
    }
    match event {
        ImeEvent::Preedit(text, range) => {
            batch(|| {
                runtime.ime.preedit.set(text.to_string());
                runtime.ime.preedit_cursor.set(*range);
            });
        }
        ImeEvent::Commit(text) => runtime.apply_commit(text),
        ImeEvent::Enabled | ImeEvent::Disabled => {}
    }
}

fn publish(
    anchor: &Cell<Option<CaretGeometry>>,
    caret_rect: Option<Signal<Option<Rect>>>,
    geometry: Option<CaretGeometry>,
) {
    anchor.set(geometry);
    let Some(target) = caret_rect else {
        return;
    };
    let rect = geometry.map(|g| Rect::new(Point::new(g.x, g.top), 0.0, g.height));
    if target.peek() != rect {
        target.set(rect);
    }
}

impl TextEditRuntime {
    fn clear_preedit(&self) {
        batch(|| {
            self.ime.preedit.set(String::new());
            self.ime.preedit_cursor.set(None);
        });
    }

    fn apply_commit(&self, text: &str) {
        batch(|| {
            self.clear_preedit();
            if !text.is_empty() {
                let mut edit = self.edit_mut();
                edit.insert_str(text);
                self.sync_value(&edit);
            }
        });
    }

    fn install_disable_clears_preedit(&self) {
        let Some(disabled) = self.disabled else {
            return;
        };
        let disabled_rt = self.clone();
        let preedit = self.ime.preedit;
        effect(move || {
            if disabled.get() && !preedit.peek().is_empty() {
                disabled_rt.clear_preedit();
            }
        });
    }

    fn install_caret_anchor_sync(&self) {
        let rt = self.clone();
        let anchor = Rc::clone(&self.ime.caret_anchor);
        let caret_rect = self.ime.caret_rect.get();
        let (padding_x, padding_y) = self.content_padding;
        effect(move || {
            if rt.disabled.is_some_and(Signal::get) {
                publish(&anchor, caret_rect, None);
                return;
            }
            let (composed, cursor) = rt.ime.composing_caret(&rt.value.get(), rt.caret.get());
            let width = rt.geometry_width();
            let style = rt.text_style.get();
            let rect = with_text_geometry(|engine| {
                let pos = engine.caret(
                    &composed,
                    &style,
                    width,
                    HitPosition {
                        byte_index: cursor,
                        affinity: TextAffinity::Downstream,
                    },
                );
                CaretGeometry {
                    x: (pos.x + padding_x - rt.scroll_x.get()).max(0.0),
                    top: pos.top + padding_y - rt.scroll_y.get(),
                    height: pos.height,
                }
            });
            publish(&anchor, caret_rect, rect);
        });
    }
}
