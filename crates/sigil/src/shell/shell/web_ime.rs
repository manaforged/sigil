use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::JsCast as _;
use wasm_bindgen::closure::Closure;
use web_sys::{
    CompositionEvent, Event, EventTarget, FocusEvent, HtmlCanvasElement, HtmlTextAreaElement,
    InputEvent, KeyboardEvent, KeyboardEventInit,
};

use crate::events::focus::TabEdges;
use crate::events::{ShellEvent, WindowEvent};
use crate::geometry::Rect;
use crate::shell::WinitWaker;
use crate::shell::composition::{CompositionInput, KeyRoute, composition_events, key_route};

type Queue = Rc<RefCell<Vec<ShellEvent>>>;
type Listener = Closure<dyn FnMut(Event)>;

const AREA_STYLE: &str = "position:fixed;left:0;top:0;width:1px;height:1em;padding:0;border:0;\
margin:0;outline:none;resize:none;overflow:hidden;opacity:0;pointer-events:none;\
white-space:nowrap;background:transparent;color:transparent;caret-color:transparent;";

const AREA_ATTRS: [(&str, &str); 6] = [
    ("autocomplete", "off"),
    ("autocorrect", "off"),
    ("autocapitalize", "off"),
    ("spellcheck", "false"),
    ("aria-hidden", "true"),
    ("tabindex", "-1"),
];

pub(crate) struct WebIme {
    area: HtmlTextAreaElement,
    canvas: HtmlCanvasElement,
    queue: Queue,
    edges: Rc<Cell<TabEdges>>,
    handoff: Rc<Cell<bool>>,
    allowed: bool,
    _listeners: Vec<Listener>,
}

struct Wiring {
    area: HtmlTextAreaElement,
    canvas: HtmlCanvasElement,
    queue: Queue,
    edges: Rc<Cell<TabEdges>>,
    waker: WinitWaker,
}

impl WebIme {
    pub(crate) fn new(canvas: HtmlCanvasElement, waker: WinitWaker) -> Option<Self> {
        let document = canvas.owner_document()?;
        let area = document
            .create_element("textarea")
            .ok()?
            .dyn_into::<HtmlTextAreaElement>()
            .ok()?;
        for (name, value) in AREA_ATTRS {
            area.set_attribute(name, value).ok()?;
        }
        area.set_attribute("style", AREA_STYLE).ok()?;
        document.body()?.append_child(&area).ok()?;
        let queue = Queue::default();
        let edges = Rc::new(Cell::new(TabEdges::default()));
        let wiring = Wiring {
            area: area.clone(),
            canvas: canvas.clone(),
            queue: Rc::clone(&queue),
            edges: Rc::clone(&edges),
            waker,
        };
        Some(Self {
            area,
            canvas,
            queue,
            edges,
            handoff: Rc::new(Cell::new(false)),
            allowed: false,
            _listeners: wiring.listen(),
        })
    }

    pub(crate) fn drain_into(&self, events: &mut Vec<ShellEvent>) {
        events.append(&mut self.queue.borrow_mut());
    }

    pub(crate) fn set_tab_edges(&self, edges: TabEdges) {
        self.edges.set(edges);
    }

    pub(crate) fn owns_focus(&self) -> bool {
        self.handoff.get() || self.active(self.area.unchecked_ref())
    }

    pub(crate) fn set_allowed(&mut self, allowed: bool) {
        if allowed && self.active(self.canvas.unchecked_ref()) {
            self.handoff.set(true);
            drop(self.area.focus());
            self.handoff.set(false);
        } else if !allowed && self.allowed {
            if self.active(self.area.unchecked_ref()) {
                drop(self.canvas.focus());
            }
            self.area.set_value("");
            self.queue
                .borrow_mut()
                .push(ShellEvent::Ime(crate::events::ImeEvent::Disabled));
        }
        self.allowed = allowed;
    }

    pub(crate) fn set_cursor_area(&self, rect: Rect, zoom: f32) {
        let bounds = self.canvas.get_bounding_client_rect();
        let left = bounds.left() + f64::from(rect.origin.x * zoom);
        let top = bounds.top() + f64::from(rect.origin.y * zoom);
        let height = f64::from(rect.height * zoom).max(1.0);
        let style = self.area.style();
        drop(style.set_property("left", &format!("{left}px")));
        drop(style.set_property("top", &format!("{top}px")));
        drop(style.set_property("height", &format!("{height}px")));
        drop(style.set_property("font-size", &format!("{height}px")));
    }

    fn active(&self, element: &web_sys::Element) -> bool {
        self.area
            .owner_document()
            .and_then(|document| document.active_element())
            .is_some_and(|active| &active == element)
    }
}

impl Drop for WebIme {
    fn drop(&mut self) {
        self.area.remove();
    }
}

impl Wiring {
    fn listen(self) -> Vec<Listener> {
        let this = Rc::new(self);
        let names = [
            "keydown",
            "keyup",
            "compositionstart",
            "compositionupdate",
            "compositionend",
            "input",
            "blur",
        ];
        let mut listeners: Vec<Listener> = names
            .into_iter()
            .filter_map(|name| {
                let wiring = Rc::clone(&this);
                let listener = Listener::new(move |event: Event| wiring.handle(name, &event));
                let target: &EventTarget = this.area.unchecked_ref();
                target
                    .add_event_listener_with_callback(name, listener.as_ref().unchecked_ref())
                    .ok()
                    .map(|()| listener)
            })
            .collect();
        listeners.extend(this.release_tab_from_canvas());
        listeners
    }

    fn release_tab_from_canvas(self: &Rc<Self>) -> Option<Listener> {
        let wiring = Rc::clone(self);
        let listener = Listener::new(move |event: Event| {
            if wiring.releases_tab(&event) {
                event.stop_immediate_propagation();
            }
        });
        let target: &EventTarget = self.canvas.unchecked_ref();
        target
            .add_event_listener_with_callback_and_bool(
                "keydown",
                listener.as_ref().unchecked_ref(),
                true,
            )
            .ok()
            .map(|()| listener)
    }

    fn releases_tab(&self, event: &Event) -> bool {
        let Some(key) = event.dyn_ref::<KeyboardEvent>() else {
            return false;
        };
        let edges = self.edges.get();
        key.key() == "Tab"
            && !key.ctrl_key()
            && !key.alt_key()
            && !key.meta_key()
            && if key.shift_key() {
                edges.first
            } else {
                edges.last
            }
    }

    fn handle(&self, name: &str, event: &Event) {
        match name {
            "keydown" | "keyup" => self.key(name, event),
            "compositionstart" => self.compose(CompositionInput::Start),
            "compositionupdate" | "compositionend" => {
                let data = event
                    .dyn_ref::<CompositionEvent>()
                    .and_then(CompositionEvent::data)
                    .unwrap_or_default();
                if name == "compositionend" {
                    self.area.set_value("");
                    self.compose(CompositionInput::End(&data));
                } else {
                    self.compose(CompositionInput::Update(&data));
                }
            }
            "input" => self.input(event),
            _ => self.blur(event),
        }
    }

    fn key(&self, name: &str, event: &Event) {
        let Some(key) = event.dyn_ref::<KeyboardEvent>() else {
            return;
        };
        if key_route(&key.key(), key.key_code(), key.is_composing()) == KeyRoute::Browser
            || (name == "keydown" && self.releases_tab(event))
        {
            return;
        }
        event.prevent_default();
        let init = KeyboardEventInit::new();
        init.set_bubbles(true);
        init.set_cancelable(true);
        init.set_key(&key.key());
        init.set_code(&key.code());
        init.set_location(key.location());
        init.set_repeat(key.repeat());
        init.set_ctrl_key(key.ctrl_key());
        init.set_shift_key(key.shift_key());
        init.set_alt_key(key.alt_key());
        init.set_meta_key(key.meta_key());
        if let Ok(copy) = KeyboardEvent::new_with_keyboard_event_init_dict(name, &init) {
            drop(self.canvas.dispatch_event(&copy));
        }
    }

    fn input(&self, event: &Event) {
        let Some(input) = event.dyn_ref::<InputEvent>() else {
            return;
        };
        if input.is_composing() || input.input_type() != "insertText" {
            return;
        }
        let data = input.data().unwrap_or_default();
        self.area.set_value("");
        self.compose(CompositionInput::Text(&data));
    }

    fn blur(&self, event: &Event) {
        let to_canvas = event
            .dyn_ref::<FocusEvent>()
            .and_then(FocusEvent::related_target)
            .is_some_and(|target| &target == self.canvas.unchecked_ref::<EventTarget>());
        if !to_canvas {
            self.push(vec![ShellEvent::Window(WindowEvent::Focused(false))]);
        }
    }

    fn compose(&self, input: CompositionInput<'_>) {
        self.push(
            composition_events(input)
                .into_iter()
                .map(ShellEvent::Ime)
                .collect(),
        );
    }

    fn push(&self, mut events: Vec<ShellEvent>) {
        if events.is_empty() {
            return;
        }
        self.queue.borrow_mut().append(&mut events);
        drop(self.waker.wake());
    }
}
