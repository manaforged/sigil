use std::cell::RefCell;
use std::collections::HashMap;
use std::hint::black_box;
use std::rc::Rc;
use std::sync::Once;
use std::time::{Duration, Instant};

use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType};
use slint::platform::{Platform, WindowAdapter, WindowEvent};
use slint::{
    ComponentHandle, LogicalPosition, Model, ModelRc, PhysicalSize, PlatformError, Rgb8Pixel,
    SharedString, VecModel,
};

use crate::measure::{Change, Rig};
use crate::scene::{BODY, MESSAGES, WHEEL_LINE, extend, titles, view};

slint::slint! {
    import { ScrollView, ListView } from "std-widgets.slint";

    export struct Message { title: string, body: string }

    component Entry inherits VerticalLayout {
        in property <Message> message;
        Text { text: message.title; wrap: no-wrap; }
        Text { text: message.body; wrap: word-wrap; }
    }

    export component Chat inherits Window {
        in property <[Message]> messages;
        in property <bool> windowed;
        in-out property <length> offset;
        if !windowed: ScrollView {
            content-y <=> root.offset;
            VerticalLayout {
                for message in root.messages: Entry { message: message; }
            }
        }
        if windowed: ListView {
            content-y <=> root.offset;
            for message in root.messages: Entry { message: message; }
        }
    }
}

thread_local! {
    static WINDOW: RefCell<Option<Rc<MinimalSoftwareWindow>>> = const { RefCell::new(None) };
}

static PLATFORM: Once = Once::new();

struct Headless(Instant);

impl Platform for Headless {
    fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, PlatformError> {
        let window = MinimalSoftwareWindow::new(RepaintBufferType::ReusedBuffer);
        WINDOW.with(|slot| slot.replace(Some(window.clone())));
        Ok(window)
    }

    fn duration_since_start(&self) -> Duration {
        self.0.elapsed()
    }
}

pub struct SlintRig {
    ui: Chat,
    window: Rc<MinimalSoftwareWindow>,
    model: Rc<VecModel<Message>>,
    stream: String,
    buffer: Vec<Rgb8Pixel>,
    drawn: bool,
}

fn size(narrow: bool) -> PhysicalSize {
    let (width, height) = view(narrow);
    PhysicalSize::new(width as u32, height as u32)
}

fn message(title: &str, body: &str) -> Message {
    Message {
        title: SharedString::from(title),
        body: SharedString::from(body),
    }
}

impl SlintRig {
    pub fn new(windowed: bool) -> Self {
        PLATFORM.call_once(|| {
            slint::platform::set_platform(Box::new(Headless(Instant::now()))).expect("platform");
        });
        let ui = Chat::new().expect("component");
        let window = WINDOW.with(RefCell::take).expect("window");
        let mut rows: Vec<Message> = titles().iter().map(|title| message(title, BODY)).collect();
        rows.push(message("", ""));
        let model = Rc::new(VecModel::from(rows));
        ui.set_messages(ModelRc::from(model.clone()));
        ui.set_windowed(windowed);
        window.set_size(size(false));
        ui.show().expect("show");
        let full = size(false);
        Self {
            ui,
            window,
            model,
            stream: String::new(),
            buffer: vec![Rgb8Pixel::default(); (full.width * full.height) as usize],
            drawn: false,
        }
    }

    fn draw(&mut self) {
        slint::platform::update_timers_and_animations();
        let stride = size(false).width as usize;
        let buffer = &mut self.buffer;
        self.drawn = self.window.draw_if_needed(|renderer| {
            black_box(renderer.render(buffer, stride));
        });
    }
}

impl Rig for SlintRig {
    fn frame(&mut self, change: Change) {
        let full = size(false);
        let at = LogicalPosition::new(full.width as f32 / 2.0, full.height as f32 / 2.0);
        if let Some(narrow) = change.narrow() {
            self.window.set_size(size(narrow));
        }
        match change {
            Change::Grow => {
                extend(&mut self.stream);
                self.model.set_row_data(MESSAGES, message("", &self.stream));
            }
            Change::Point => self
                .window
                .dispatch_event(WindowEvent::PointerMoved { position: at }),
            Change::Wheel(lines) => self.window.dispatch_event(WindowEvent::PointerScrolled {
                position: at,
                delta_x: 0.0,
                delta_y: lines * WHEEL_LINE,
            }),
            Change::None | Change::Narrow | Change::Wide => {}
        }
        self.draw();
    }

    fn show(&mut self) -> String {
        self.draw();
        let mut colors: HashMap<(u8, u8, u8), usize> = HashMap::new();
        for pixel in &self.buffer {
            *colors.entry((pixel.r, pixel.g, pixel.b)).or_default() += 1;
        }
        let ground = colors.values().copied().max().unwrap_or_default();
        let inked = self.buffer.len() - ground;
        format!(
            "offset {} drawn {} inked pixels {inked}",
            self.ui.get_offset(),
            self.drawn
        )
    }
}
