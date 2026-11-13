use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::AppConfig;
use crate::kernel::IntoNode;
use crate::terminal::caps::Capabilities;
use crate::terminal::graphics::Mux;
use crate::terminal::run::{isolated, recover};

#[cfg(feature = "scene")]
mod draw;
#[cfg(not(feature = "scene"))]
#[path = "pixels/inert.rs"]
mod draw;
#[cfg(feature = "scene")]
mod tiles;

const ENV: &str = "SIGIL_TERMINAL";
/// The cell in pixels when the terminal reports none; Kitty scales each tile to its cells, so a guess only sets sharpness.
const GUESS: [u16; 2] = [16, 34];

static REQUESTED: AtomicBool = AtomicBool::new(false);
static AVAILABLE: AtomicBool = AtomicBool::new(false);

/// Asks the terminal host to draw the window layout as images, and returns whether it will.
pub(crate) fn set_pixels(on: bool) -> bool {
    REQUESTED.store(on, Ordering::Relaxed);
    crate::reactive::Runtime::mark_frame_dirty();
    pixels()
}

/// Returns whether the terminal host draws the window layout as images.
#[must_use]
pub(crate) fn pixels() -> bool {
    REQUESTED.load(Ordering::Relaxed) && AVAILABLE.load(Ordering::Relaxed)
}

fn available(config: &AppConfig, caps: Capabilities) -> bool {
    cfg!(feature = "scene") && !config.inline && cell(caps).is_some()
}

/// The cell size pixel mode draws at: the reported one, else a guess on a Kitty terminal.
fn cell(caps: Capabilities) -> Option<[u16; 2]> {
    if caps.graphics != Some(crate::terminal::Protocol::Kitty) {
        return None;
    }
    Some(caps.cell.filter(|[w, h]| *w > 0 && *h > 0).unwrap_or(GUESS))
}

/// The pixel host of a terminal run, mounted the first time pixel mode turns on.
pub(super) struct Pixels {
    inner: Option<draw::Draw>,
    #[cfg(feature = "scene")]
    quit: crate::app::config::QuitKeys,
    on: bool,
}

#[cfg_attr(
    not(feature = "scene"),
    allow(
        clippy::unused_self,
        clippy::unnecessary_wraps,
        clippy::needless_pass_by_ref_mut,
        clippy::missing_const_for_fn,
        reason = "pixel mode needs the scene feature; without it these keep the signatures the run loop calls"
    )
)]
impl Pixels {
    pub(super) fn open(config: &AppConfig, caps: Capabilities) -> Self {
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
            inner: None,
            #[cfg(feature = "scene")]
            quit: config.quit,
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
        let caps = crate::terminal::term::current_caps();
        if want {
            #[cfg(feature = "scene")]
            {
                let inner = self.inner.get_or_insert_with(|| draw::Draw::mount(tree));
                inner.host_mut().set_quit(self.quit);
                inner.host_mut().attach_theme(cells.theme().clone());
                inner.enter()?;
            }
        } else if let Some(inner) = &mut self.inner {
            cells.attach_theme(inner.host().theme().clone());
            inner.leave(caps.mux)?;
        }
        #[cfg(not(feature = "scene"))]
        let _ = tree;
        Ok(true)
    }

    /// Returns the pixel host while pixel mode is on.
    pub(super) fn host(&self) -> Option<&crate::Host> {
        self.inner
            .as_ref()
            .filter(|_| self.on)
            .map(draw::Draw::host)
    }

    /// Runs one pixel frame under `policy` and returns whether the app asked to exit.
    pub(super) fn turn(
        &mut self,
        policy: crate::app::PanicPolicy,
        (input, dt): (Vec<crate::events::Event>, f32),
        size: (u16, u16),
        term: &mut crate::terminal::term::Term,
    ) -> crate::AppResult<bool> {
        if let (Some(inner), Some(cell)) = (&mut self.inner, cell(term.caps())) {
            let frame = isolated(policy, || inner.turn((input, dt), size, cell, term))?;
            if frame.is_none() {
                recover(term)?;
                inner.enter().map_err(crate::Error::io)?;
            }
            return Ok(frame.unwrap_or(false));
        }
        Ok(false)
    }

    /// Removes the image when the run ends in pixel mode.
    pub(super) fn close(mut self, mux: Option<Mux>) -> io::Result<()> {
        self.leave(mux)
    }

    fn leave(&mut self, mux: Option<Mux>) -> io::Result<()> {
        if let (true, Some(inner)) = (core::mem::take(&mut self.on), &mut self.inner) {
            return inner.leave(mux);
        }
        Ok(())
    }
}

impl Drop for Pixels {
    fn drop(&mut self) {
        AVAILABLE.store(false, Ordering::Relaxed);
        if let Err(err) = self.leave(crate::terminal::term::current_caps().mux) {
            tracing::warn!(target: "sigil::terminal", error = %err, "pixel image cleanup failed");
        }
    }
}
