use std::borrow::Cow;

use sigil::geometry::PhysicalSize;
use sigil::scene::{CommandScene, SceneCommand};
use sigil::style::Rgba;

mod draws;
mod shapes;
/// Fixed scenes that stress nesting depth, glyph and image counts, and viewport size.
pub mod stress;

/// The device scales that `generate` picks from.
pub const SCALES: [f32; 5] = [1.0, 1.25, 1.5, 2.0, 3.0];
const SEED_BYTES: usize = 16_384;
const MAX_DEPTH: u32 = 6;

/// A byte stream that drives scene generation and random walks.
#[derive(Clone)]
pub struct Entropy<'a> {
    bytes: Cow<'a, [u8]>,
    at: usize,
}

impl std::fmt::Debug for Entropy<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entropy")
            .field("len", &self.bytes.len())
            .field("at", &self.at)
            .finish()
    }
}

impl<'a> Entropy<'a> {
    /// Reads entropy from `bytes`, starting at the first byte.
    #[must_use]
    pub const fn from_bytes(bytes: &'a [u8]) -> Self {
        Self {
            bytes: Cow::Borrowed(bytes),
            at: 0,
        }
    }

    /// Expands `seed` into 16,384 bytes of deterministic entropy.
    #[must_use]
    pub fn from_seed(seed: u64) -> Entropy<'static> {
        let mut state = seed;
        let mut bytes = Vec::with_capacity(SEED_BYTES);
        while bytes.len() < SEED_BYTES {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            bytes.extend_from_slice(&(z ^ (z >> 31)).to_le_bytes());
        }
        Entropy {
            bytes: Cow::Owned(bytes),
            at: 0,
        }
    }

    /// Takes the next byte, or 0 once the stream is exhausted.
    pub fn byte(&mut self) -> u8 {
        let value = self.bytes.get(self.at).copied().unwrap_or(0);
        self.at += 1;
        value
    }

    /// Reports whether every byte has been taken.
    #[must_use]
    pub fn exhausted(&self) -> bool {
        self.at >= self.bytes.len()
    }

    /// Takes two bytes and returns a value below `bound`, or 0 when `bound` is 0.
    pub fn below(&mut self, bound: u32) -> u32 {
        let raw = u32::from(u16::from_le_bytes([self.byte(), self.byte()]));
        raw % bound.max(1)
    }

    /// Takes one byte and returns true when its value modulo 100 is below `percent`.
    pub fn chance(&mut self, percent: u8) -> bool {
        self.byte() % 100 < percent
    }

    /// Takes two bytes and returns a value from 0 to 1 inclusive.
    pub fn unit(&mut self) -> f32 {
        f32::from(u16::from_le_bytes([self.byte(), self.byte()])) / f32::from(u16::MAX)
    }

    /// Takes two bytes and returns a value from `low` to `high` inclusive.
    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        (high - low).mul_add(self.unit(), low)
    }

    /// Takes two bytes and returns one of `items`, and panics when `items` is empty.
    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        let len = u32::try_from(items.len()).unwrap_or(u32::MAX);
        let index = usize::try_from(self.below(len)).unwrap_or(0);
        items
            .get(index)
            .or_else(|| items.last())
            .copied()
            .expect("pick needs at least one item")
    }
}

/// A generated command scene with its viewport, device scale, and clear color.
#[derive(Clone, Debug)]
pub struct GeneratedScene {
    commands: Vec<SceneCommand>,
    viewport: (f32, f32),
    scale: f32,
    clear: Rgba,
}

impl GeneratedScene {
    /// The drawing commands of the scene.
    #[must_use]
    pub fn commands(&self) -> &[SceneCommand] {
        &self.commands
    }

    /// The logical viewport of the scene.
    #[must_use]
    pub const fn viewport(&self) -> (f32, f32) {
        self.viewport
    }

    /// The device scale of the scene.
    #[must_use]
    pub const fn scale(&self) -> f32 {
        self.scale
    }

    /// The clear color of the scene.
    #[must_use]
    pub const fn clear(&self) -> Rgba {
        self.clear
    }

    /// Generates the scene that `generate` builds from the entropy of `seed`.
    #[must_use]
    pub fn from_seed(seed: u64) -> Self {
        generate(&mut Entropy::from_seed(seed))
    }

    /// Copies the commands into a `CommandScene` for a renderer.
    #[must_use]
    pub fn scene(&self) -> CommandScene {
        CommandScene {
            commands: self.commands.clone(),
        }
    }

    /// The size of the render target in pixels: the viewport times the scale, rounded.
    #[must_use]
    pub fn physical(&self) -> PhysicalSize {
        let [width, height] = crate::physical_extent(self.viewport, self.scale);
        PhysicalSize::new(width, height)
    }

    /// Returns a copy that holds only the commands at `indices`, in the given order, skipping indices that are out of range.
    #[must_use]
    pub fn keep(&self, indices: &[usize]) -> Self {
        Self {
            commands: indices
                .iter()
                .filter_map(|&i| self.commands.get(i).cloned())
                .collect(),
            ..self.clone()
        }
    }

    /// Parses fixture text with a `seed` line or a hex `bytes` line and an optional `keep` line; `None` means a value did not parse or neither source line was present.
    #[must_use]
    pub fn from_fixture(text: &str) -> Option<Self> {
        let mut base = None;
        let mut keep = None;
        for line in text.lines() {
            match line.split_once(' ') {
                Some(("seed", value)) => base = Some(Self::from_seed(value.trim().parse().ok()?)),
                Some(("bytes", hex)) => {
                    base = Some(generate(&mut Entropy::from_bytes(&unhex(hex)?)));
                }
                Some(("keep", list)) => {
                    keep = Some(
                        list.split_whitespace()
                            .map(str::parse)
                            .collect::<Result<Vec<usize>, _>>()
                            .ok()?,
                    );
                }
                _ => {}
            }
        }
        let base = base?;
        Some(keep.map_or_else(|| base.clone(), |keep| base.keep(&keep)))
    }
}

/// Formats a `seed` line and a `keep` line that `GeneratedScene::from_fixture` reads back.
#[must_use]
pub fn fixture_text(seed: u64, keep: &[usize]) -> String {
    let list: Vec<String> = keep.iter().map(ToString::to_string).collect();
    format!("seed {seed}\nkeep {}\n", list.join(" "))
}

fn unhex(hex: &str) -> Option<Vec<u8>> {
    let hex = hex.trim().as_bytes();
    hex.chunks(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect()
}

/// Builds a scene from `entropy`: a viewport of 4 to 256 logical pixels per side in steps of 4, a scale from `SCALES`, an opaque clear color, and at most 48 draw or nesting items.
#[must_use]
pub fn generate(entropy: &mut Entropy<'_>) -> GeneratedScene {
    let width = f32::from(u16::try_from(entropy.below(64) + 1).unwrap_or(1)) * 4.0;
    let height = f32::from(u16::try_from(entropy.below(64) + 1).unwrap_or(1)) * 4.0;
    let scale = entropy.pick(&SCALES);
    let clear = Rgba::new(entropy.unit(), entropy.unit(), entropy.unit(), 1.0);
    let mut out = Vec::new();
    let budget = entropy.below(48) + 1;
    let mut builder = Gen {
        entropy,
        viewport: (width, height),
        out: &mut out,
        budget,
    };
    builder.body(0);
    GeneratedScene {
        commands: out,
        viewport: (width, height),
        scale,
        clear,
    }
}

pub(crate) struct Gen<'e, 'b, 'o> {
    pub(crate) entropy: &'e mut Entropy<'b>,
    pub(crate) viewport: (f32, f32),
    pub(crate) out: &'o mut Vec<SceneCommand>,
    budget: u32,
}

impl Gen<'_, '_, '_> {
    fn body(&mut self, depth: u32) {
        let items = self.entropy.below(6) + 1;
        for _ in 0..items {
            if self.budget == 0 || self.entropy.exhausted() {
                return;
            }
            self.budget -= 1;
            self.item(depth);
        }
    }

    fn item(&mut self, depth: u32) {
        let nest = depth < MAX_DEPTH;
        match self.entropy.below(20) {
            0 | 1 if nest => {
                let clip = self.rect();
                self.nested(
                    SceneCommand::PushClip(clip.into()),
                    SceneCommand::PopClip,
                    depth,
                );
            }
            2 | 3 if nest => {
                let transform = self.transform();
                self.nested(
                    SceneCommand::PushTransform(transform),
                    SceneCommand::PopTransform,
                    depth,
                );
            }
            4 if nest => {
                let opacity = self.opacity_f32();
                self.nested(
                    SceneCommand::SaveLayer { opacity },
                    SceneCommand::RestoreLayer,
                    depth,
                );
            }
            5 if nest => {
                let scope = self.layer_scope();
                self.nested(
                    SceneCommand::PushLayer(scope),
                    SceneCommand::PopLayer,
                    depth,
                );
            }
            6 => self.stray_pop(),
            _ => self.draw(),
        }
    }

    fn nested(&mut self, open: SceneCommand, close: SceneCommand, depth: u32) {
        self.out.push(open);
        if !self.entropy.chance(8) {
            self.body(depth + 1);
        }
        if !self.entropy.chance(4) {
            self.out.push(close);
        }
    }

    fn stray_pop(&mut self) {
        let pop = match self.entropy.below(4) {
            0 => SceneCommand::PopClip,
            1 => SceneCommand::PopTransform,
            2 => SceneCommand::RestoreLayer,
            _ => SceneCommand::PopLayer,
        };
        self.out.push(pop);
    }
}
