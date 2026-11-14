use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use crate::geometry::Rect;
use crate::reactive::{Reactive, SendableSignal, memo};
use crate::scene::command::fills::TRANSPARENT;
use crate::scene::corner_radii::CornerRadii;
use crate::style::{Color, ColorValue, Theme};

pub(crate) const VARS: usize = 8;
pub(crate) const COLORS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct ShaderKey(u32);

#[derive(Default)]
struct Sources {
    index: HashMap<Arc<str>, u32>,
    list: Vec<Arc<str>>,
}

fn with_sources<R>(job: impl FnOnce(&mut Sources) -> R) -> R {
    static SOURCES: OnceLock<Mutex<Sources>> = OnceLock::new();
    let mut sources = SOURCES
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    job(&mut sources)
}

impl ShaderKey {
    fn of(source: Arc<str>) -> Self {
        with_sources(|sources| {
            if let Some(&index) = sources.index.get(&source) {
                return Self(index);
            }
            let index = u32::try_from(sources.list.len()).unwrap_or(u32::MAX);
            sources.list.push(Arc::clone(&source));
            sources.index.insert(source, index);
            Self(index)
        })
    }

    #[cfg(feature = "scene")]
    pub(crate) fn source(self) -> Option<Arc<str>> {
        with_sources(|sources| sources.list.get(self.0 as usize).cloned())
    }
}

fn next_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[derive(Clone)]
enum Var {
    Fixed(f32),
    Live(SendableSignal<f32>),
}

impl Var {
    fn get(&self) -> f32 {
        match self {
            Self::Fixed(value) => *value,
            Self::Live(read) => read.into_signal().try_peek().unwrap_or_default(),
        }
    }
}

#[derive(Clone)]
struct Parts {
    id: u64,
    key: ShaderKey,
    vars: [Var; VARS],
    colors: [Option<ColorValue>; COLORS],
}

/// A WGSL `shade` function the GPU runs as a fill or over a subtree; on WebGPU keep `fwidth`, `dpdx`, and `textureSample` out of branches.
#[derive(Clone)]
#[must_use]
pub struct Shader(Arc<Parts>);

#[allow(
    clippy::cast_possible_truncation,
    reason = "shader vars are f32 on the GPU"
)]
const fn narrow(value: f64) -> f32 {
    value as f32
}

impl Shader {
    /// Wraps WGSL `source` that defines `fn shade(in: Shading) -> vec4<f32>`; the GPU compiles each distinct source once.
    pub fn new(source: impl Into<Arc<str>>) -> Self {
        Self(Arc::new(Parts {
            id: next_id(),
            key: ShaderKey::of(source.into()),
            vars: std::array::from_fn(|_| Var::Fixed(0.0)),
            colors: [None; COLORS],
        }))
    }

    /// Binds `in.vars[index]`, 0 to 7, to a number, signal, animator, or closure; a change redraws only this node.
    pub fn var<V: Into<f64> + 'static>(mut self, index: usize, value: impl Reactive<V>) -> Self {
        let var = match value.try_static() {
            Some(fixed) => Var::Fixed(narrow(fixed.into())),
            None => Var::Live(memo(move || narrow(value.read().into())).into_sendable()),
        };
        if let Some(slot) = self.parts().vars.get_mut(index) {
            *slot = var;
        }
        self
    }

    /// Binds `in.colors[index]`, 0 to 3, to a color or theme token; color 0 is also the fallback where WGSL cannot run.
    pub fn color(mut self, index: usize, color: impl Into<ColorValue>) -> Self {
        let color = color.into();
        if let Some(slot) = self.parts().colors.get_mut(index) {
            *slot = Some(color);
        }
        self
    }

    fn parts(&mut self) -> &mut Parts {
        let parts = Arc::make_mut(&mut self.0);
        parts.id = next_id();
        parts
    }

    pub(crate) fn id(&self) -> u64 {
        self.0.id
    }

    pub(crate) fn is_live(&self) -> bool {
        self.0.vars.iter().any(|var| matches!(var, Var::Live(_)))
    }

    pub(crate) fn track(&self) {
        for var in &self.0.vars {
            if let Var::Live(read) = var {
                let _ = read.into_signal().get();
            }
        }
    }

    pub(crate) fn shading(&self, theme: &Theme) -> Shading {
        Shading {
            key: self.0.key,
            vars: self.0.vars.each_ref().map(Var::get),
            colors: resolve(&self.0.colors, theme),
        }
    }

    pub(crate) fn draw(&self, rect: Rect, radii: CornerRadii, theme: &Theme) -> DrawShader {
        DrawShader {
            rect,
            radii,
            opacity: 10_000,
            shading: self.shading(theme),
            paints: self.0.colors,
        }
    }
}

fn resolve(paints: &[Option<ColorValue>; COLORS], theme: &Theme) -> [Color; COLORS] {
    paints.map(|paint| paint.map_or(TRANSPARENT, |color| theme.color(color)))
}

impl PartialEq for Shader {
    fn eq(&self, other: &Self) -> bool {
        self.0.id == other.0.id
    }
}

impl Eq for Shader {}

impl Hash for Shader {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.id.hash(state);
    }
}

impl core::fmt::Debug for Shader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Shader")
            .field("id", &self.0.id)
            .field("key", &self.0.key)
            .field("live", &self.is_live())
            .finish_non_exhaustive()
    }
}

impl Reactive<Self> for Shader {
    fn read(&self) -> Self {
        self.clone()
    }

    fn try_static(&self) -> Option<Self> {
        Some(self.clone())
    }
}

thread_local! {
    static HANDOFF: RefCell<Option<Shader>> = const { RefCell::new(None) };
}

pub(crate) fn hand_off(shader: Shader) -> u64 {
    let id = shader.id();
    HANDOFF.with_borrow_mut(|slot| *slot = Some(shader));
    id
}

pub(crate) fn claim(id: u64) -> Option<Shader> {
    HANDOFF.with_borrow_mut(|slot| slot.take_if(|shader| shader.id() == id))
}

/// The program, numbers, and colors that one draw of a `Shader` reads.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shading {
    pub(crate) key: ShaderKey,
    pub(crate) vars: [f32; VARS],
    pub(crate) colors: [Color; COLORS],
}

impl Shading {
    pub(crate) const fn fallback(&self) -> Color {
        let [first, ..] = self.colors;
        first
    }

    #[cfg(feature = "scene")]
    pub(crate) const fn still(self) -> Self {
        Self {
            key: self.key,
            vars: [0.0; VARS],
            colors: [TRANSPARENT; COLORS],
        }
    }
}

/// Runs a `Shader` over a rounded rectangle.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawShader {
    pub(crate) rect: Rect,
    pub(crate) radii: CornerRadii,
    pub(crate) opacity: u16,
    pub(crate) shading: Shading,
    pub(crate) paints: [Option<ColorValue>; COLORS],
}

impl DrawShader {
    pub(crate) const fn opacity(mut self, opacity: u16) -> Self {
        self.opacity = opacity;
        self
    }

    pub(crate) fn theme(&mut self, theme: &Theme) {
        self.shading.colors = resolve(&self.paints, theme);
    }

    pub(crate) fn fallback(&self) -> crate::scene::command::DrawRect {
        crate::scene::command::DrawRect::new(self.rect, self.shading.fallback())
            .border_radius(self.radii)
            .opacity(self.opacity)
    }
}
