use super::{
    App, AvailableSpace, Bounds, Context, Element, ElementId, GlobalElementId, InspectorElementId,
    Instant, IntoElement, LayoutId, Pixels, Rc, Render, SeedableRng, StdRng, Surface,
    TestAppContext, TestDispatcher, VisualTestContext, Window, div, milli_px, point, px, size,
};

struct EmptyRoot;
impl Render for EmptyRoot {
    fn render(&mut self, _w: &mut Window, _cx: &mut Context<'_, Self>) -> impl IntoElement {
        div()
    }
}

pub(super) struct Host {
    vc: VisualTestContext,
}

pub(super) fn new_host() -> Host {
    let dispatcher = TestDispatcher::new(StdRng::seed_from_u64(0));
    let mut cx = TestAppContext::build(dispatcher, Some("bench-parity-gpui"));
    let window = cx.add_window(|_w, _cx| EmptyRoot);
    Host {
        vc: VisualTestContext::from_window(window.into(), &cx),
    }
}

fn space(p: &bench_parity::WorkloadParams) -> gpui::Size<AvailableSpace> {
    let (vw, vh) = p.viewport();
    size(px(vw), px(vh)).map(AvailableSpace::Definite)
}

pub(super) fn time_construct_layout(
    host: &mut Host,
    surface: Surface,
    p: &bench_parity::WorkloadParams,
    build: impl FnOnce() -> gpui::AnyElement,
) -> (u64, bool, [i64; 2]) {
    let origin = point(px(0.0), px(0.0));
    let avail = space(p);
    let state = Rc::new(TimingState::new(surface));
    let shared = Rc::clone(&state);
    host.vc.draw(origin, avail, move |_window, _cx| {
        shared.started.set(Some(Instant::now()));
        TimedElement {
            inner: build(),
            state: shared,
        }
    });
    (
        state.elapsed.get(),
        state.painted.get(),
        state.root_size.get(),
    )
}

struct TimingState {
    surface: Surface,
    started: std::cell::Cell<Option<Instant>>,
    elapsed: std::cell::Cell<u64>,
    painted: std::cell::Cell<bool>,
    root_size: std::cell::Cell<[i64; 2]>,
}

impl TimingState {
    fn new(surface: Surface) -> Self {
        Self {
            surface,
            started: std::cell::Cell::new(None),
            elapsed: std::cell::Cell::new(0),
            painted: std::cell::Cell::new(surface != Surface::LayoutPaint),
            root_size: std::cell::Cell::new([0, 0]),
        }
    }

    fn stop(&self) {
        let elapsed = self.started.get().map_or(0, |start| {
            u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
        });
        self.elapsed.set(elapsed);
    }
}

struct TimedElement {
    inner: gpui::AnyElement,
    state: Rc<TimingState>,
}

impl IntoElement for TimedElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TimedElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.inner.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.inner.prepaint(window, cx);
        self.state.root_size.set([
            milli_px(f32::from(bounds.size.width)),
            milli_px(f32::from(bounds.size.height)),
        ]);
        if self.state.surface == Surface::Layout {
            self.state.stop();
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request: &mut (),
        _prepaint: &mut (),
        _window: &mut Window,
        _cx: &mut App,
    ) {
        if self.state.surface == Surface::LayoutPaint {
            // The headless test window has no current view, which GPUI's
            // interactive list paint requires. Fail the paint proof without
            // calling into a path that corrupts the test window on unwind.
            self.state.painted.set(false);
            self.state.stop();
        }
    }
}
