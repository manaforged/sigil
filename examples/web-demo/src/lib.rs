//! Browser-hosted Sigil renderer proof.

#![allow(missing_docs)]
#![allow(
    unused_crate_dependencies,
    reason = "getrandom selects the browser entropy backend for ahash"
)]
// Native targets compile an empty cdylib — the demo is wasm-only, and
// `cargo check --workspace --all-targets` must stay green on the host.
#![cfg(target_arch = "wasm32")]
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    reason = "browser dimensions and pointer coordinates are bounded CSS pixel values"
)]
#![allow(
    clippy::future_not_send,
    reason = "browser GPU futures run on wasm-bindgen's main-thread local executor"
)]

use std::cell::RefCell;
use std::rc::Rc;

use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil::render::{Color, ScreenSubmission, WgpuRenderer};
use wasm_bindgen::JsCast as _;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, MessageEvent, PointerEvent as BrowserPointerEvent};

#[derive(Clone, Copy, PartialEq)]
enum Variant {
    Primary,
    Neutral,
    Ghost,
    Destructive,
}

#[derive(Clone, Copy)]
struct DemoState {
    label: Signal<String>,
    variant: Signal<Variant>,
    width: Signal<f32>,
    disabled: Signal<bool>,
    clicks: Signal<u32>,
}

impl DemoState {
    fn new() -> Self {
        Self {
            label: signal("Save changes".to_owned()),
            variant: signal(Variant::Primary),
            width: signal(148.0),
            disabled: signal(false),
            clicks: signal(0),
        }
    }
}

struct Demo {
    _owner: Owner,
    canvas: HtmlCanvasElement,
    state: DemoState,
    host: Host,
    router: EventRouter,
    renderer: WgpuRenderer,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl Demo {
    async fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let (width, height, _) = canvas_size(&canvas)?;
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(js_error)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(js_error)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("sigil browser device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(js_error)?;
        let config = surface
            .get_default_config(&adapter, width, height)
            .ok_or_else(|| JsValue::from_str("WebGPU surface is unsupported"))?;
        surface.configure(&device, &config);

        let owner = Owner::new("web-demo", None);
        let (host, state) = owner.with(|| {
            let state = DemoState::new();
            (build_host(state), state)
        });
        Ok(Self {
            _owner: owner,
            canvas,
            state,
            host,
            router: EventRouter::new(),
            renderer: WgpuRenderer::new().map_err(js_error)?,
            surface,
            device,
            queue,
            config,
        })
    }

    fn render(&mut self) -> Result<(), JsValue> {
        let (width, height, scale) = canvas_size(&self.canvas)?;
        if self.config.width != width || self.config.height != height {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
        let viewport = (width as f32 / scale, height as f32 / scale);
        let scene = self.host.scene(viewport).map_err(js_error)?;
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                return Err(JsValue::from_str("WebGPU surface validation failed"));
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.renderer
            .submit_screen(
                &ScreenSubmission {
                    device: &self.device,
                    queue: &self.queue,
                    view: &view,
                    format: self.config.format,
                    viewport,
                    scale,
                    clear: Some(Color::rgba(0.055, 0.055, 0.067, 1.0)),
                },
                &scene,
            )
            .map_err(js_error)?;
        self.queue.present(frame);
        Ok(())
    }

    fn pointer(&mut self, event: &BrowserPointerEvent, phase: PointerPhase) {
        let pressed = phase != PointerPhase::Up && event.buttons() & 1 == 1;
        self.host.route(
            &mut self.router,
            ShellEvent::Pointer(PointerEvent::new(PointerEventInit {
                position: Point::new(event.offset_x() as f32, event.offset_y() as f32),
                delta: Point::new(event.movement_x() as f32, event.movement_y() as f32),
                kind: PointerKind::Mouse,
                button: (phase != PointerPhase::Move).then_some(MouseButton::Left),
                buttons: if pressed {
                    MouseButtons::LEFT
                } else {
                    MouseButtons::EMPTY
                },
                modifiers: ModifiersState::empty(),
                phase,
            })),
        );
    }
}

fn build_host(state: DemoState) -> Host {
    let mut host = Host::new();
    host.mount(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .fill()
                    .padding(32.0)
                    .align_items(AlignItems::Center)
                    .justify_content(JustifyContent::Center),
            )
            .class("bg-surface")
            .child(
                container()
                    .layout(
                        LayoutStyle::flex_column()
                            .fill()
                            .padding(24.0)
                            .gap(20.0)
                            .align_items(AlignItems::Start)
                            .justify_content(JustifyContent::Center),
                    )
                    .class("bg-surface")
                    .child(text("BUTTON / LIVE COMPONENT").class("text-xs font-mono"))
                    .child(text("Try the actual control.").class("text-xl font-bold"))
                    .child(
                        text("The component below is laid out, shaped, hit-tested, and painted by Sigil.")
                            .class("text-sm text-on-surface opacity-75"),
                    )
                    .child(container().children_reactive(move || demo_children(state))),
            ),
    );
    host
}

fn demo_children(state: DemoState) -> Vec<Box<dyn IntoNode>> {
    let label = state.label.get();
    let width = state.width.get();
    let disabled = state.disabled.get();
    let variant = state.variant.get();
    let clicks = state.clicks.get();
    let action = button(label)
        .width(width)
        .enabled(!disabled)
        .activate(move || state.clicks.set(state.clicks.peek() + 1));
    let action = match variant {
        Variant::Primary => action.primary(),
        Variant::Neutral => action.neutral(),
        Variant::Ghost => action.ghost(),
        Variant::Destructive => action.destructive(),
    };
    vec![node(
        container()
            .layout(
                LayoutStyle::flex_column()
                    .gap(12.0)
                    .align_items(AlignItems::Start),
            )
            .child(action)
            .child(text(format!("{clicks} activations")).class("text-sm font-mono")),
    )]
}

fn apply_message(state: DemoState, message: &str) {
    if let Some(value) = message.strip_prefix("sigil-demo:label:") {
        state.label.set(value.chars().take(24).collect());
    } else if let Some(value) = message.strip_prefix("sigil-demo:width:") {
        if let Ok(width) = value.parse::<f32>() {
            state.width.set(width.clamp(96.0, 280.0));
        }
    } else if let Some(value) = message.strip_prefix("sigil-demo:disabled:") {
        state.disabled.set(value == "true");
    } else if let Some(value) = message.strip_prefix("sigil-demo:variant:") {
        let variant = match value {
            "neutral" => Variant::Neutral,
            "ghost" => Variant::Ghost,
            "destructive" => Variant::Destructive,
            _ => Variant::Primary,
        };
        state.variant.set(variant);
    }
}

fn install_pointer_listener(
    canvas: &HtmlCanvasElement,
    demo: Rc<RefCell<Demo>>,
    name: &'static str,
    phase: PointerPhase,
) -> Result<(), JsValue> {
    let callback =
        Closure::<dyn FnMut(BrowserPointerEvent)>::new(move |event: BrowserPointerEvent| {
            event.prevent_default();
            let mut demo = demo.borrow_mut();
            demo.pointer(&event, phase);
            if let Err(error) = demo.render() {
                web_sys::console::error_1(&error);
            }
        });
    canvas.add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())?;
    callback.forget();
    Ok(())
}

fn install_resize_listener(demo: Rc<RefCell<Demo>>) -> Result<(), JsValue> {
    let callback = Closure::<dyn FnMut()>::new(move || {
        if let Err(error) = demo.borrow_mut().render() {
            web_sys::console::error_1(&error);
        }
    });
    web_sys::window()
        .ok_or_else(|| JsValue::from_str("window is unavailable"))?
        .add_event_listener_with_callback("resize", callback.as_ref().unchecked_ref())?;
    callback.forget();
    Ok(())
}

fn install_message_listener(demo: Rc<RefCell<Demo>>) -> Result<(), JsValue> {
    let callback = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let trusted = web_sys::window()
            .and_then(|window| window.location().origin().ok())
            .is_some_and(|origin| origin == event.origin());
        if !trusted {
            return;
        }
        let Some(message) = event.data().as_string() else {
            return;
        };
        let mut demo = demo.borrow_mut();
        apply_message(demo.state, &message);
        if let Err(error) = demo.render() {
            web_sys::console::error_1(&error);
        }
    });
    web_sys::window()
        .ok_or_else(|| JsValue::from_str("window is unavailable"))?
        .add_event_listener_with_callback("message", callback.as_ref().unchecked_ref())?;
    callback.forget();
    Ok(())
}

fn canvas_size(canvas: &HtmlCanvasElement) -> Result<(u32, u32, f32), JsValue> {
    let scale = web_sys::window()
        .ok_or_else(|| JsValue::from_str("window is unavailable"))?
        .device_pixel_ratio() as f32;
    let width = (canvas.client_width().max(1) as f32 * scale).round() as u32;
    let height = (canvas.client_height().max(1) as f32 * scale).round() as u32;
    Ok((width, height, scale))
}

async fn start() -> Result<(), JsValue> {
    sigil::set_text_tuning(sigil::TextTuning {
        enabled: false,
        ..sigil::TextTuning::default()
    });
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    let canvas = document
        .get_element_by_id("sigil-canvas")
        .ok_or_else(|| JsValue::from_str("#sigil-canvas is missing"))?
        .dyn_into::<HtmlCanvasElement>()?;
    let demo = Rc::new(RefCell::new(Demo::new(canvas.clone()).await?));
    demo.borrow_mut().render()?;
    if let Some(status) = document.get_element_by_id("status") {
        status.set_attribute("hidden", "")?;
    }
    install_pointer_listener(&canvas, Rc::clone(&demo), "pointermove", PointerPhase::Move)?;
    install_pointer_listener(&canvas, Rc::clone(&demo), "pointerdown", PointerPhase::Down)?;
    install_pointer_listener(&canvas, Rc::clone(&demo), "pointerup", PointerPhase::Up)?;
    install_resize_listener(Rc::clone(&demo))?;
    install_message_listener(demo)?;
    Ok(())
}

fn js_error(error: impl core::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
#[wasm_bindgen(start)]
pub fn main() {
    std::panic::set_hook(Box::new(|panic| {
        web_sys::console::error_1(&JsValue::from_str(&panic.to_string()));
    }));
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = start().await {
            web_sys::console::error_1(&error);
            if let Some(status) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id("status"))
            {
                status.set_text_content(Some("WebGPU is unavailable in this browser."));
            }
        }
    });
}
