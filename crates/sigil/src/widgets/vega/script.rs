use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rquickjs::{Coerced, Context, Ctx, Function, Object, Runtime};

use super::engine::{Chart, Failure};
use super::measure::{Font, Widths};

const PRELUDE: &str = include_str!("prelude.js");
const VEGA: &str = include_str!("../../../assets/vega/vega.min.js");
const VEGA_LITE: &str = include_str!("../../../assets/vega/vega-lite.min.js");
const GLUE: &str = include_str!("glue.js");
const MEMORY_LIMIT: usize = 256 << 20;
const STACK_LIMIT: usize = 8 << 20;
const DEADLINE: Duration = Duration::from_secs(2);
const DISARMED: u64 = u64::MAX;
const OUT_OF_MEMORY: &str = "out of memory";

pub(super) struct Script {
    context: Context,
    runtime: Runtime,
    deadline: Arc<AtomicU64>,
    epoch: Instant,
}

impl Script {
    pub(super) fn new() -> Option<Self> {
        let runtime = Runtime::new().ok()?;
        runtime.set_memory_limit(MEMORY_LIMIT);
        runtime.set_max_stack_size(STACK_LIMIT);
        let deadline = Arc::new(AtomicU64::new(DISARMED));
        let epoch = Instant::now();
        let armed = Arc::clone(&deadline);
        runtime.set_interrupt_handler(Some(Box::new(move || {
            millis(epoch) > armed.load(Ordering::Relaxed)
        })));
        let context = Context::full(&runtime).ok()?;
        context.with(|ctx| install(&ctx)).ok()?;
        Some(Self {
            context,
            runtime,
            deadline,
            epoch,
        })
    }

    pub(super) fn render(&self, spec: &str, config: &str) -> Result<Chart, Failure> {
        let limit = millis(self.epoch).saturating_add(millis_of(DEADLINE));
        self.deadline.store(limit, Ordering::Relaxed);
        let outcome = self
            .start(spec, config)
            .and_then(|()| self.drain())
            .and_then(|()| self.collect());
        self.deadline.store(DISARMED, Ordering::Relaxed);
        self.runtime.run_gc();
        outcome
    }

    fn start(&self, spec: &str, config: &str) -> Result<(), Failure> {
        self.context.with(|ctx| {
            let render: Function<'_> =
                ctx.globals().get("__render").map_err(|_| Failure::Engine)?;
            render
                .call::<_, ()>((spec, config))
                .map_err(|_| self.failure(&caught(&ctx)))
        })
    }

    fn drain(&self) -> Result<(), Failure> {
        loop {
            if self.expired() {
                return Err(Failure::Deadline);
            }
            match self.runtime.execute_pending_job() {
                Ok(true) => continue,
                Ok(false) => {}
                Err(job) => return Err(job.0.with(|ctx| self.failure(&caught(&ctx)))),
            }
            if self.flag("__done")? {
                return Ok(());
            }
            if !self.tick()? {
                return Err(Failure::Script);
            }
        }
    }

    fn flag(&self, name: &str) -> Result<bool, Failure> {
        self.context
            .with(|ctx| ctx.globals().get::<_, bool>(name))
            .map_err(|_| Failure::Script)
    }

    fn tick(&self) -> Result<bool, Failure> {
        self.context.with(|ctx| {
            let tick: Function<'_> = ctx.globals().get("__tick").map_err(|_| Failure::Engine)?;
            tick.call::<_, bool>(())
                .map_err(|_| self.failure(&caught(&ctx)))
        })
    }

    fn collect(&self) -> Result<Chart, Failure> {
        self.context.with(|ctx| {
            let out: Object<'_> = ctx.globals().get("__out").map_err(|_| Failure::Script)?;
            let read = |key: &str| out.get::<_, Option<String>>(key).ok().flatten();
            if let Some(error) = read("err") {
                return Err(classify(&error));
            }
            let svg = read("svg").ok_or(Failure::Script)?;
            let text = read("text").unwrap_or_default();
            Ok(Chart { svg, text })
        })
    }

    fn expired(&self) -> bool {
        millis(self.epoch) > self.deadline.load(Ordering::Relaxed)
    }

    fn failure(&self, message: &str) -> Failure {
        if self.expired() {
            Failure::Deadline
        } else {
            classify(message)
        }
    }
}

fn install(ctx: &Ctx<'_>) -> rquickjs::Result<()> {
    let widths = Rc::new(RefCell::new(Widths::default()));
    let measure = Function::new(
        ctx.clone(),
        move |text: String, size: f64, family: String, weight: String, style: String| {
            let font = Font {
                size,
                family: &family,
                weight: &weight,
                style: &style,
            };
            widths.borrow_mut().width(&text, &font)
        },
    )?;
    ctx.globals().set("__measure", measure)?;
    for source in [PRELUDE, VEGA, VEGA_LITE, GLUE] {
        ctx.eval::<(), _>(source)?;
    }
    Ok(())
}

fn caught(ctx: &Ctx<'_>) -> String {
    ctx.catch()
        .get::<Coerced<String>>()
        .map(|text| text.0)
        .unwrap_or_default()
}

fn classify(message: &str) -> Failure {
    if message.contains(OUT_OF_MEMORY) {
        Failure::Memory
    } else {
        Failure::Script
    }
}

fn millis(epoch: Instant) -> u64 {
    millis_of(epoch.elapsed())
}

fn millis_of(span: Duration) -> u64 {
    u64::try_from(span.as_millis()).unwrap_or(DISARMED)
}
