use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::AppConfig;
use crate::kernel::IntoNode;
use crate::terminal::caps::Caps;
use crate::terminal::graphics::Mux;

#[cfg(feature = "scene")]
mod draw;
#[cfg(feature = "scene")]
mod tiles;

const ENV: &str = "SIGIL_TERMINAL";
/// The cell in pixels when the terminal reports none; Kitty scales each tile to its cells, so a guess only sets sharpness.
const GUESS: [u16; 2] = [16, 34];

static REQUESTED: AtomicBool = AtomicBool::new(false);
static AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Asks the terminal host to draw the window layout as images, and returns whether it will.
pub fn set_pixels(on: bool) -> bool {
    REQUESTED.store(on, Ordering::Relaxed);
    crate::reactive::Runtime::mark_frame_dirty();
    pixels()
}

/// Returns whether the terminal host draws the window layout as images.
#[must_use]
pub fn pixels() -> bool {
    REQUESTED.load(Ordering::Relaxed) && AVAILABLE.load(Ordering::Relaxed)
}

fn available(config: &AppConfig, caps: Caps) -> bool {
    cfg!(feature = "scene") && !config.inline && cell(caps).is_some()
}

/// The cell size pixel mode draws at: the reported one, else a guess on a Kitty terminal.
fn cell(caps: Caps) -> Option<[u16; 2]> {
    if caps.graphics != Some(crate::terminal::Protocol::Kitty) {
        return None;
    }
    Some(caps.cell.filter(|[w, h]| *w > 0 && *h > 0).unwrap_or(GUESS))
}

/// The pixel host of a terminal run, mounted the first time pixel mode turns on.
pub(super) struct Pixels {
    #[cfg(feature = "scene")]
    inner: Option<draw::Draw>,
    on: bool,
}

impl Pixels {
    pub(super) fn open(config: &AppConfig, caps: Caps) -> Self {
        let available = available(config, caps);
        AVAILABLE.store(available, Ordering::Relaxed);
        if std::env::var(ENV).as_deref() == Ok("pixels") {
            REQUESTED.store(true, Ordering::Relaxed);
            if !available {
                tracing::warn!(
                    target: "sigil::terminal",
                    "{ENV}=pixels needs the desktop feature and the kitty graphics protocol; drawing cells"
                );
            }
        }
        Self {
            #[cfg(feature = "scene")]
            inner: None,
            on: false,
        }
    }

    /// Applies a pending switch and returns whether the mode changed.
    pub(super) fn sync<N: IntoNode>(
        &mut self,
        tree: &std::rc::Rc<impl Fn() -> N + 'static>,
        cells: &mut crate::Host,
    ) -> io::Result<bool> {
        let want = pixels();
        if want == self.on {
            return Ok(false);
        }
        self.on = want;
        #[cfg(feature = "scene")]
        {
            let caps = crate::terminal::term::current_caps();
            if want {
                let inner = self.inner.get_or_insert_with(|| draw::Draw::mount(tree));
                inner.host_mut().attach_theme(cells.theme().clone());
                inner.enter()?;
            } else if let Some(inner) = &mut self.inner {
                cells.attach_theme(inner.host().theme().clone());
                inner.leave(caps.mux)?;
            }
        }
        #[cfg(not(feature = "scene"))]
        let _ = (tree, cells);
        Ok(true)
    }

    /// Returns the pixel host while pixel mode is on.
    pub(super) fn host(&self) -> Option<&crate::Host> {
        #[cfg(feature = "scene")]
        if self.on {
            return self.inner.as_ref().map(draw::Draw::host);
        }
        None
    }

    /// Runs one pixel frame and returns whether the app asked to exit.
    pub(super) fn turn(
        &mut self,
        input: Vec<crate::events::Event>,
        size: (u16, u16),
        term: &mut crate::terminal::term::Term,
        dt: f32,
    ) -> crate::AppResult<bool> {
        #[cfg(feature = "scene")]
        if let (Some(inner), Some(cell)) = (&mut self.inner, cell(term.caps())) {
            return inner.turn(input, size, cell, term, dt);
        }
        let _ = (input, size, term, dt);
        Ok(false)
    }

    /// Removes the image when the run ends in pixel mode.
    pub(super) fn close(mut self, mux: Option<Mux>) -> io::Result<()> {
        #[cfg(feature = "scene")]
        if let (true, Some(inner)) = (self.on, &mut self.inner) {
            return inner.leave(mux);
        }
        let _ = (&mut self, mux);
        Ok(())
    }
}
